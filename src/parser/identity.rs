use age::{Identity, IdentityFile, Recipient};
use eyre::{Context, eyre};
use log::debug;
use serde::Deserialize;

use super::super::util::callback::UiCallbacks;

#[derive(Debug, Deserialize, Clone)]
pub struct RawIdentity(String);

pub struct ParsedIdentity {
    pub identity: Box<dyn Identity>,
    pub recipient: Box<dyn Recipient + Send>,
}

impl From<String> for RawIdentity {
    fn from(s: String) -> Self {
        Self(s)
    }
}

impl ParsedIdentity {
    pub fn from_exist(identity: Box<dyn Identity>, recipient: Box<dyn Recipient + Send>) -> Self {
        Self {
            identity,
            recipient,
        }
    }
    pub fn _get_identity(&self) -> &dyn Identity {
        self.identity.as_ref()
    }
    pub fn _get_recipient(&self) -> &dyn Recipient {
        self.recipient.as_ref()
    }
}

use std::io::{self, BufReader, Read};
enum PeekState {
    Peeking { consumed: usize },
    Reading,
}

struct PeekableReader<R: Read> {
    inner: BufReader<R>,
    state: PeekState,
}

/// See <https://github.com/str4d/rage/blob/d7c727aef96cc007e142f5b21c0d19210154b3c7/age/src/cli_common.rs>
/// Same as default buffer size for `BufReader`, but hard-coded so we know exactly what
/// the buffer size is, and therefore can detect if the entire file fits into a single
/// buffer.
///
/// This must be at least 71 bytes to ensure the correct behaviour of
/// `PeekableReader::fill_buf`. See the comments in that method.
const PEEKABLE_SIZE: usize = 8 * 1024;

impl<R: io::Read> PeekableReader<R> {
    fn new(inner: R) -> Self {
        Self {
            inner: BufReader::with_capacity(PEEKABLE_SIZE, inner),
            state: PeekState::Peeking { consumed: 0 },
        }
    }

    fn reset(&mut self) -> io::Result<()> {
        match &mut self.state {
            PeekState::Peeking { consumed } => {
                *consumed = 0;
                Ok(())
            }
            PeekState::Reading => Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "Tried to reset after the underlying buffer was exceeded.",
            )),
        }
    }
}

impl<R: io::Read> io::Read for PeekableReader<R> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        match self.state {
            PeekState::Peeking { .. } => {
                // Perform a read that will never exceed the size of the inner buffer.
                use std::io::BufRead;
                let nread = {
                    let mut rem = self.fill_buf()?;
                    rem.read(buf)?
                };
                self.consume(nread);
                Ok(nread)
            }
            PeekState::Reading => self.inner.read(buf),
        }
    }
}

impl<R: io::Read> io::BufRead for PeekableReader<R> {
    fn fill_buf(&mut self) -> io::Result<&[u8]> {
        match self.state {
            PeekState::Peeking { consumed } => {
                let inner_len = self.inner.fill_buf()?.len();
                if inner_len == 0 {
                    // This state only occurs when the underlying data source is empty.
                    // Don't fall through to change the state to `Reading`, because we can
                    // always reset an empty stream.
                    assert_eq!(consumed, 0);
                    Ok(&[])
                } else if consumed < inner_len {
                    // Re-call so we aren't extending the lifetime of the mutable borrow
                    // on `self.inner` to outside the conditional, which would prevent us
                    // from performing other mutable operations on the other side.
                    Ok(&self.inner.fill_buf()?[consumed..])
                } else if inner_len < PEEKABLE_SIZE {
                    // We have read the entire file into a single buffer and consumed all
                    // of it. Don't fall through to change the state to `Reading`, because
                    // we can always reset a single-buffer stream.
                    //
                    // Note that we cannot distinguish between the file being the exact
                    // same size as our buffer, and the file being larger than it. But
                    // this only becomes relevant if we cannot distinguish between the
                    // kinds of identity files we support parsing, within a single buffer.
                    // We should always be able to distinguish before then, because we
                    // parse in the following order:
                    //
                    // - Encrypted identities, which are parsed incrementally as age
                    //   ciphertexts with optional armor, and can be detected in at most
                    //   70 bytes.
                    // - SSH identities, which are parsed as a PEM encoding and can be
                    //   detected in at most 36 bytes.
                    // - Identity files, which have one identity per line and therefore
                    //   can have arbitrarily long lines. We intentionally try this format
                    //   last.
                    assert_eq!(consumed, inner_len);
                    Ok(&[])
                } else {
                    // We're done peeking.
                    self.inner.consume(consumed);
                    self.state = PeekState::Reading;
                    self.inner.fill_buf()
                }
            }
            PeekState::Reading => self.inner.fill_buf(),
        }
    }

    fn consume(&mut self, amt: usize) {
        match &mut self.state {
            PeekState::Peeking { consumed, .. } => *consumed += amt,
            PeekState::Reading => self.inner.consume(amt),
        }
    }
}

/// A guard that helps to ensure that standard input is only used once.
pub struct StdinGuard {
    stdin_used: bool,
}

/// Wrapper around a [`File`].
pub struct FileReader {
    inner: std::fs::File,
    #[allow(dead_code)]
    filename: String,
}

/// Wrapper around either a file or standard input.
pub enum InputReader {
    /// Wrapper around a file.
    File(FileReader),

    /// Wrapper around standard input.
    Stdin(io::Stdin),
}

impl Read for InputReader {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        match self {
            InputReader::File(f) => f.inner.read(buf),
            InputReader::Stdin(handle) => handle.read(buf),
        }
    }
}

impl InputReader {
    /// Reads input from the given filename, or standard input if `None` or `Some("-")`.
    pub fn new(input: Option<String>) -> io::Result<Self> {
        if let Some(filename) = input {
            // Respect the Unix convention that "-" as an input filename
            // parameter is an explicit request to use standard input.
            return Ok(InputReader::File(FileReader {
                inner: std::fs::File::open(&filename)?,
                filename,
            }));
        }

        Ok(InputReader::Stdin(io::stdin()))
    }
}

use eyre::Result;
impl StdinGuard {
    /// Constructs a new `StdinGuard`.
    ///
    /// `input_is_stdin` should be set to `true` if standard input is being used for
    /// plaintext input during encryption, or ciphertext input during decryption.
    pub fn new(input_is_stdin: bool) -> Self {
        Self {
            stdin_used: input_is_stdin,
        }
    }

    fn open(&mut self, filename: String) -> Result<InputReader> {
        let input = InputReader::new(Some(filename))?;
        if matches!(input, InputReader::Stdin(_)) {
            if self.stdin_used {
                // TODO: refine error type
                return Err(eyre!("multiple stdin"));
            }
            self.stdin_used = true;
        }
        Ok(input)
    }
}

impl TryInto<ParsedIdentity> for RawIdentity {
    type Error = eyre::ErrReport;
    fn try_into(self) -> std::result::Result<ParsedIdentity, Self::Error> {
        let Self(identity_filename) = self;
        if identity_filename.is_empty() {
            Err(eyre!(
                "No identity found, require `vaultix.settings.identity`."
            ))
        } else {
            let identity_file_result = IdentityFile::from_file(identity_filename.clone())
                .map_err(|e| eyre!("import from identity file {identity_filename} error: {e}"));

            #[cfg(feature = "plugin")]
            let identity_file = identity_file_result.map(|i| i.with_callbacks(UiCallbacks));
            #[cfg(not(feature = "plugin"))]
            let identity_file = identity_file_result;

            if let Ok(idf) = identity_file {
                let recip = idf
                    .to_recipients()
                    .wrap_err_with(|| "transform to recipient fail")
                    .map(|o| o.into_iter().next());

                let ident = idf
                    .into_identities()
                    .wrap_err_with(|| "transform to identity fail")
                    .map(|o| o.into_iter().next());

                if let (Ok(Some(r)), Ok(Some(i))) = (recip, ident) {
                    return Ok(ParsedIdentity::from_exist(i, r));
                }
            }

            // single multi-line ssh key handle
            debug!("searching ssh key as identity");
            let mut stdin_guard = StdinGuard::new(false); // no stdin, only from file

            let mut reader = PeekableReader::new(stdin_guard.open(identity_filename.clone())?);

            match age::ssh::Identity::from_buffer(&mut reader, Some(identity_filename.clone())) {
                Ok(age::ssh::Identity::Unsupported(k)) => {
                    return Err(eyre!("unsupported key: {identity_filename}, {k:#?}"));
                }
                Ok(identity) => {
                    let ident = Box::new(identity.clone().with_callbacks(UiCallbacks));
                    match age::ssh::Recipient::try_from(identity).map(Box::new) {
                        Ok(recip) => return Ok(ParsedIdentity::from_exist(ident, recip)),
                        Err(e) => {
                            return Err(eyre!("failed converting ssh key to recipient: {e:#?}"));
                        }
                    }
                }
                Err(_) => (),
            }
            reader.reset()?;
            Err(eyre!("handle ssh key fail"))
        }
    }
}

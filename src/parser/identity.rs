use age::{Identity, IdentityFile, Recipient};
use eyre::{ContextCompat, eyre};
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

impl TryInto<ParsedIdentity> for RawIdentity {
    type Error = eyre::ErrReport;
    fn try_into(self) -> std::result::Result<ParsedIdentity, Self::Error> {
        let Self(identity) = self;
        if identity.is_empty() {
            Err(eyre!(
                "No identity found, require `vaultix.settings.identity`."
            ))
        } else {
            macro_rules! create {
                ($method:ident,  $err_context:expr) => {{
                    let identity_file_result = IdentityFile::from_file(identity.clone())
                        .map_err(|e| eyre!("import from identity file {identity} error: {e}"))?;

                    #[cfg(feature = "plugin")]
                    let processed_identity_file = identity_file_result.with_callbacks(UiCallbacks);
                    #[cfg(not(feature = "plugin"))]
                    let processed_identity_file = identity_file_result;

                    processed_identity_file
                        .$method()
                        .map_err(|e| eyre!("{}", e))?
                        .into_iter()
                        .next()
                        .with_context(|| $err_context)?
                }};
            }
            let ident = create!(into_identities, "into identity fail");

            let recip = create!(to_recipients, "into recip fail");

            Ok(ParsedIdentity::from_exist(ident, recip))
        }
    }
}

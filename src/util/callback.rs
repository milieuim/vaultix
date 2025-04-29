use age::secrecy::{ExposeSecret, SecretString};
use age::Callbacks;
use pinentry::{ConfirmationDialog, PassphraseInput};
use rpassword::prompt_password;
use std::io;
use subtle::ConstantTimeEq;

#[derive(Clone, Copy)]
pub struct UiCallbacks;

impl Callbacks for UiCallbacks {
    fn display_message(&self, message: &str) {
        eprintln!("{message}");
    }

    fn confirm(&self, message: &str, yes_string: &str, no_string: Option<&str>) -> Option<bool> {
        confirm(message, yes_string, no_string).ok()
    }

    fn request_public_string(&self, description: &str) -> Option<String> {
        let term = console::Term::stderr();
        term.write_str(description).ok()?;
        term.read_line().ok().filter(|s| !s.is_empty())
    }

    fn request_passphrase(&self, description: &str) -> Option<SecretString> {
        read_secret(description, "input password:", None).ok()
    }
}
fn confirm(query: &str, ok: &str, cancel: Option<&str>) -> pinentry::Result<bool> {
    if let Some(mut input) = ConfirmationDialog::with_default_binary() {
        // pinentry binary is available!
        input.with_ok(ok).with_timeout(30);
        if let Some(cancel) = cancel {
            input.with_cancel(cancel);
        }
        input.confirm(query)
    } else {
        // Fall back to CLI interface.
        let term = console::Term::stderr();
        let initial = format!("{query}: (y/n) ");
        loop {
            term.write_str(&initial)?;
            let response = term.read_line()?.to_lowercase();
            if ["y", "yes"].contains(&response.as_str()) {
                break Ok(true);
            } else if ["n", "no"].contains(&response.as_str()) {
                break Ok(false);
            }
        }
    }
}

pub fn read_secret(
    description: &str,
    prompt: &str,
    confirm: Option<&str>,
) -> pinentry::Result<SecretString> {
    // Check for the pinentry environment variable. If it's not present try to use the default
    // binary.
    let input = if let Ok(pinentry) = std::env::var("PINENTRY_PROGRAM") {
        PassphraseInput::with_binary(pinentry)
    } else {
        PassphraseInput::with_default_binary()
    };

    if let Some(mut input) = input {
        // User-set or default pinentry binary is available!
        let mismatch_error = "secret mismatch";
        let empty_error = "require secret input";
        input
            .with_description(description)
            .with_prompt(prompt)
            .with_timeout(30);
        if let Some(confirm_prompt) = confirm {
            input.with_confirmation(confirm_prompt, mismatch_error);
        } else {
            input.required(empty_error);
        }
        input.interact()
    } else {
        // Fall back to CLI interface.
        let passphrase = prompt_password(format!("{description}: ")).map(SecretString::from)?;
        if let Some(confirm_prompt) = confirm {
            let confirm_passphrase =
                prompt_password(format!("{confirm_prompt}: ")).map(SecretString::from)?;

            if !bool::from(
                passphrase
                    .expose_secret()
                    .as_bytes()
                    .ct_eq(confirm_passphrase.expose_secret().as_bytes()),
            ) {
                return Err(pinentry::Error::Io(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "Invalid Input",
                )));
            }
        } else if passphrase.expose_secret().is_empty() {
            return Err(pinentry::Error::Cancelled);
        }

        Ok(passphrase)
    }
}

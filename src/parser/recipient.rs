use age::{Callbacks, EncryptError, Recipient, plugin, ssh, x25519};
use eyre::{Context, bail, eyre};
use log::trace;
use serde::Deserialize;

use crate::util::callback::UiCallbacks;

#[derive(Debug, Deserialize, Clone)]
pub struct RecipString(String);

impl From<String> for RecipString {
    fn from(s: String) -> Self {
        Self(s)
    }
}

pub fn ssh_host_key_type(recipient: &str) -> eyre::Result<Option<&str>> {
    let key_type = recipient.split_whitespace().next().unwrap_or_default();
    match key_type {
        "ssh-ed25519" => Ok(Some("ed25519")),
        "ssh-rsa" => Ok(Some("rsa")),
        _ if key_type.starts_with("ssh-") || key_type.starts_with("ecdsa-") => bail!(
            "Unsupported SSH host key type `{key_type}`. Vaultix's age backend supports only \
             Ed25519 and RSA SSH keys. Configure an Ed25519 or RSA host key and set \
             vaultix.settings.hostPubkey to its public key."
        ),
        _ => Ok(None),
    }
}

impl TryInto<Box<dyn Recipient + Send>> for RecipString {
    type Error = eyre::ErrReport;
    fn try_into(self) -> Result<Box<dyn Recipient + Send>, Self::Error> {
        use std::str::FromStr;
        let recip_str = self.0.as_str();
        trace!("parsing recipient(s): {recip_str}");
        ssh_host_key_type(recip_str)?;
        macro_rules! try_recipients {
            ($pub_str:expr, $($type:path),+) => {
                $(
                    if let Ok(o) = <$type>::from_str($pub_str) {
                        return Ok(Box::new(o) as Box<dyn Recipient + Send>);
                    }
                )+
            };
        }
        try_recipients!(recip_str, ssh::Recipient, x25519::Recipient);

        #[cfg(feature = "plugin")]
        {
            let plugin_recip = recip_str
                .parse::<plugin::Recipient>()
                .map_err(|e| eyre!(e))?;
            build_plugin_recip(&plugin_recip, UiCallbacks::new())
        }

        #[cfg(not(feature = "plugin"))]
        Err(eyre!("incompatible recipient type"))
    }
}

fn build_plugin_recip(
    plugin_recip: &plugin::Recipient,
    callbacks: impl Callbacks,
) -> eyre::Result<Box<dyn Recipient + Send>> {
    let plugin_name = plugin_recip.plugin();

    match plugin::RecipientPluginV1::new(
        plugin_name,
        std::slice::from_ref(plugin_recip),
        &[],
        callbacks.clone(),
    ) {
        Ok(o) => Ok(Box::new(o)),
        Err(EncryptError::MissingPlugin { binary_name }) => {
            bail!("age plugin: {binary_name} not found")
        }
        Err(e) => Err(e).wrap_err_with(|| eyre!("unknown fail related to plugin")),
    }
}

#[cfg(test)]
mod tests {
    use super::{RecipString, ssh_host_key_type};
    use age::Recipient;

    #[test]
    fn unsupported_ssh_host_key_has_actionable_error() {
        let result: eyre::Result<Box<dyn Recipient + Send>> =
            RecipString::from("ssh-mldsa44-ed25519 AAAA host".to_owned()).try_into();
        let error = result
            .err()
            .expect("hybrid SSH keys are unsupported")
            .to_string();
        assert!(error.contains("ssh-mldsa44-ed25519"));
        assert!(error.contains("Ed25519 and RSA"));
        assert!(error.contains("Ed25519 or RSA host key"));
        assert!(error.contains("vaultix.settings.hostPubkey"));
    }

    #[test]
    fn distinguishes_supported_ssh_host_key_types() {
        assert_eq!(
            ssh_host_key_type("ssh-ed25519 AAAA host").unwrap(),
            Some("ed25519")
        );
        assert_eq!(ssh_host_key_type("ssh-rsa AAAA host").unwrap(), Some("rsa"));
        assert_eq!(ssh_host_key_type("age1recipient").unwrap(), None);
        assert_eq!(ssh_host_key_type("age1plugin1recipient").unwrap(), None);
    }

    // comment since CI/CD doesn't have plugin installed
    // use super::*;

    // #[test]
    // fn recipient_from_str() {
    //     let plugin_recip =
    //         "age1yubikey1q2aucts9c72rmvnysgczu2zuvtvysmg2hkpvqld9qt5kyvyj6dd8gaujk62"
    //             .parse::<plugin::Recipient>()
    //             .map_err(|e| eyre!(e))
    //             .unwrap();
    //     build_plugin_recip(&plugin_recip, UiCallbacks).unwrap();
    // }
}

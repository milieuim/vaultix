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

impl TryInto<Box<dyn Recipient + Send>> for RecipString {
    type Error = eyre::ErrReport;
    fn try_into(self) -> Result<Box<dyn Recipient + Send>, Self::Error> {
        use std::str::FromStr;
        let recip_str = self.0.as_str();
        trace!("parsing recipient(s): {recip_str}");
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
            build_plugin_recip(&plugin_recip, UiCallbacks)
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

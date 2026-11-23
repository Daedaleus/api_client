//! The app's settings as the settings dialog shows and returns them.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::settings::{
    AppSettings, ClientCertificate, EncryptedPassphrase, HostPattern, NetworkSettings, ProxySetting,
};

/// How requests reach servers.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(
    tag = "mode",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum ProxyView {
    /// The operating system's settings.
    #[default]
    System,
    /// Directly.
    None,
    /// Through a proxy.
    Manual {
        /// Proxy URL.
        url: String,
        /// Hosts reached directly, comma-separated.
        no_proxy: String,
    },
}

/// The app's settings.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct SettingsView {
    /// The proxy.
    pub proxy: ProxyView,
    /// Files with extra trusted CA certificates.
    #[serde(default)]
    pub ca_certificates: Vec<String>,
    /// Client certificates per host.
    #[serde(default)]
    pub client_certificates: Vec<ClientCertificateView>,
}

/// A client certificate as the dialog shows and returns it. The passphrase
/// travels only encrypted; a new one comes in as `new_passphrase` and is
/// encrypted by the app before it is saved.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct ClientCertificateView {
    /// Host pattern, e.g. `api.example.com` or `*.internal`.
    pub host: String,
    /// PEM or PKCS#12 file.
    pub certificate: String,
    /// PEM key file; empty if the key is in the certificate file.
    #[serde(default)]
    pub key: String,
    /// The stored passphrase, encrypted; `null` if there is none.
    #[serde(default)]
    pub encrypted_passphrase: Option<String>,
    /// A new passphrase to store (`""` removes it); `null` keeps the
    /// stored one.
    #[serde(default)]
    pub new_passphrase: Option<String>,
}

impl SettingsView {
    /// The dialog's view of `settings`.
    #[must_use]
    pub fn from_settings(settings: &AppSettings) -> Self {
        let network = &settings.network;
        Self {
            proxy: match &network.proxy {
                ProxySetting::System => ProxyView::System,
                ProxySetting::None => ProxyView::None,
                ProxySetting::Manual { url, no_proxy } => ProxyView::Manual {
                    url: url.clone(),
                    no_proxy: no_proxy.clone(),
                },
            },
            ca_certificates: network
                .ca_certificates
                .iter()
                .map(|path| path.display().to_string())
                .collect(),
            client_certificates: network
                .client_certificates
                .iter()
                .map(|certificate| ClientCertificateView {
                    host: certificate.host.as_str().to_owned(),
                    certificate: certificate.certificate.display().to_string(),
                    key: certificate
                        .key
                        .as_ref()
                        .map_or_default(|key| key.display().to_string()),
                    encrypted_passphrase: certificate
                        .passphrase
                        .as_ref()
                        .map(|passphrase| passphrase.0.clone()),
                    new_passphrase: None,
                })
                .collect(),
        }
    }

    /// The settings the dialog describes (blank entries dropped, spaces
    /// trimmed). Client certificates keep their stored passphrase; new
    /// ones are encrypted by the app.
    #[must_use]
    pub fn to_settings(&self) -> AppSettings {
        AppSettings {
            network: NetworkSettings {
                proxy: match &self.proxy {
                    ProxyView::System => ProxySetting::System,
                    ProxyView::None => ProxySetting::None,
                    ProxyView::Manual { url, no_proxy } => ProxySetting::Manual {
                        url: url.trim().to_owned(),
                        no_proxy: no_proxy.trim().to_owned(),
                    },
                },
                ca_certificates: self
                    .ca_certificates
                    .iter()
                    .map(|path| path.trim())
                    .filter(|path| !path.is_empty())
                    .map(PathBuf::from)
                    .collect(),
                client_certificates: self
                    .client_certificates
                    .iter()
                    .filter(|entry| {
                        !entry.host.trim().is_empty() || !entry.certificate.trim().is_empty()
                    })
                    .map(|entry| ClientCertificate {
                        host: HostPattern::new(&entry.host),
                        certificate: PathBuf::from(entry.certificate.trim()),
                        key: Some(entry.key.trim())
                            .filter(|key| !key.is_empty())
                            .map(PathBuf::from),
                        passphrase: entry.encrypted_passphrase.clone().map(EncryptedPassphrase),
                    })
                    .collect(),
            },
        }
    }
}

#[cfg(test)]
#[path = "../tests/view_settings.rs"]
mod tests;

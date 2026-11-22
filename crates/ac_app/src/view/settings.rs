//! The app's settings as the settings dialog shows and returns them.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::settings::{AppSettings, NetworkSettings, ProxySetting};

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
        }
    }

    /// The settings the dialog describes (blank entries dropped, spaces
    /// trimmed).
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
            },
        }
    }
}

#[cfg(test)]
#[path = "../tests/view_settings.rs"]
mod tests;

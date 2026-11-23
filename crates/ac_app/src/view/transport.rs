//! Redirects, time limit and cookies as the editor shows them (see
//! `ac_domain::transport`).

use ac_domain::transport::{Cookies, Redirects, TransportSettings};
use serde::{Deserialize, Serialize};

/// Whether redirects are followed (serialized in camelCase).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub enum RedirectsView {
    /// As the folder or collection around it.
    #[default]
    Inherit,
    /// Follow them.
    Follow,
    /// Show the redirect itself.
    Stop,
}

/// Whether the cookie jar is used (serialized in camelCase).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub enum CookiesView {
    /// As the folder or collection around it.
    #[default]
    Inherit,
    /// Send and keep cookies.
    On,
    /// Neither send nor keep them.
    Off,
}

/// Redirects, time limit and cookies of a request, folder or collection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct TransportDraft {
    /// Redirects.
    #[serde(default)]
    pub redirects: RedirectsView,
    /// Time limit in seconds; `null` inherits.
    #[serde(default)]
    pub timeout_seconds: Option<u32>,
    /// The cookie jar.
    #[serde(default)]
    pub cookies: CookiesView,
}

impl TransportDraft {
    /// The editor's view of `settings`.
    #[must_use]
    pub fn from_settings(settings: &TransportSettings) -> Self {
        Self {
            redirects: match settings.redirects {
                Redirects::Inherit => RedirectsView::Inherit,
                Redirects::Follow => RedirectsView::Follow,
                Redirects::Stop => RedirectsView::Stop,
            },
            timeout_seconds: settings.timeout_seconds,
            cookies: match settings.cookies {
                Cookies::Inherit => CookiesView::Inherit,
                Cookies::On => CookiesView::On,
                Cookies::Off => CookiesView::Off,
            },
        }
    }

    /// The settings the editor describes (a time limit of 0 inherits).
    #[must_use]
    pub fn to_settings(self) -> TransportSettings {
        TransportSettings {
            redirects: match self.redirects {
                RedirectsView::Inherit => Redirects::Inherit,
                RedirectsView::Follow => Redirects::Follow,
                RedirectsView::Stop => Redirects::Stop,
            },
            timeout_seconds: self.timeout_seconds.filter(|seconds| *seconds > 0),
            cookies: match self.cookies {
                CookiesView::Inherit => Cookies::Inherit,
                CookiesView::On => Cookies::On,
                CookiesView::Off => Cookies::Off,
            },
        }
    }
}

#[cfg(test)]
#[path = "../tests/view_transport.rs"]
mod tests;

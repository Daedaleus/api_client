//! Authentication as the editors show and return it.

use ac_domain::auth::{Auth, KeyPlacement};
use serde::{Deserialize, Serialize};

/// The authentication of a request, folder or collection.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum AuthDraft {
    /// Use that of the folder or collection around it.
    #[default]
    Inherit,
    /// None.
    None,
    /// HTTP Basic.
    Basic {
        /// User name.
        username: String,
        /// Password.
        password: String,
    },
    /// Bearer token.
    Bearer {
        /// The token.
        token: String,
    },
    /// API key.
    ApiKey {
        /// Header or parameter name.
        name: String,
        /// The key.
        value: String,
        /// Where it goes.
        placement: KeyPlacementView,
    },
}

/// Where an API key goes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub enum KeyPlacementView {
    /// As a header.
    #[default]
    Header,
    /// As a query parameter.
    Query,
}

impl AuthDraft {
    /// The editor's view of `auth`.
    #[must_use]
    pub fn from_auth(auth: &Auth) -> Self {
        match auth {
            Auth::Inherit => Self::Inherit,
            Auth::None => Self::None,
            Auth::Basic { username, password } => Self::Basic {
                username: username.clone(),
                password: password.clone(),
            },
            Auth::Bearer { token } => Self::Bearer {
                token: token.clone(),
            },
            Auth::ApiKey {
                name,
                value,
                placement,
            } => Self::ApiKey {
                name: name.clone(),
                value: value.clone(),
                placement: match placement {
                    KeyPlacement::Header => KeyPlacementView::Header,
                    KeyPlacement::Query => KeyPlacementView::Query,
                },
            },
        }
    }

    /// The authentication the editor describes.
    #[must_use]
    pub fn to_auth(&self) -> Auth {
        match self {
            Self::Inherit => Auth::Inherit,
            Self::None => Auth::None,
            Self::Basic { username, password } => Auth::Basic {
                username: username.clone(),
                password: password.clone(),
            },
            Self::Bearer { token } => Auth::Bearer {
                token: token.clone(),
            },
            Self::ApiKey {
                name,
                value,
                placement,
            } => Auth::ApiKey {
                name: name.clone(),
                value: value.clone(),
                placement: match placement {
                    KeyPlacementView::Header => KeyPlacement::Header,
                    KeyPlacementView::Query => KeyPlacement::Query,
                },
            },
        }
    }
}

#[cfg(test)]
#[path = "../tests/view_auth.rs"]
mod tests;

//! Commands from the UI, their acknowledgements and errors.
//!
//! JSON shape (shared with the TypeScript UI):
//! `{"type": "sendRequest", "method": "GET", "url": "…"}` →
//! `{"type": "runStarted", "runId": 1}`.

use ac_domain::protocol::InvalidMethod;
use serde::{Deserialize, Serialize};

use crate::text::{UserText, keys};

/// Something the user wants the application to do.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(
    tag = "type",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum Command {
    /// Send a request; the result arrives as [`crate::AppEvent::RunFinished`].
    SendRequest {
        /// Method as entered, e.g. `GET`.
        method: String,
        /// URL as entered; `http://` is assumed when no scheme is given.
        url: String,
    },
}

/// Identifies one sending of a request, so the UI can match the result.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
#[serde(transparent)]
pub struct RunId(pub u64);

/// Immediate answer to a [`Command`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(
    tag = "type",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum CommandAck {
    /// The request is on its way.
    RunStarted {
        /// Id of this run.
        run_id: RunId,
    },
}

/// A command that cannot be carried out as given.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum AppError {
    /// The method is not a valid HTTP token.
    #[error(transparent)]
    InvalidMethod(#[from] InvalidMethod),
    /// No URL entered.
    #[error("URL is empty")]
    EmptyUrl,
    /// The URL cannot be parsed.
    #[error("invalid URL: {0}")]
    InvalidUrl(String),
    /// The URL uses a scheme other than http or https.
    #[error("unsupported URL scheme {0:?}")]
    UnsupportedScheme(String),
}

impl AppError {
    /// The text to show the user.
    #[must_use]
    pub fn user_text(&self) -> UserText {
        match self {
            Self::InvalidMethod(InvalidMethod(method)) => {
                UserText::new(keys::METHOD_INVALID).with("method", method.clone())
            }
            Self::EmptyUrl => UserText::new(keys::URL_EMPTY),
            Self::InvalidUrl(reason) => {
                UserText::new(keys::URL_INVALID).with("reason", reason.clone())
            }
            Self::UnsupportedScheme(scheme) => {
                UserText::new(keys::URL_SCHEME).with("scheme", scheme.clone())
            }
        }
    }
}

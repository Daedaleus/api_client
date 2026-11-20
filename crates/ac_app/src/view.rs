//! View models: plain data for the UI, serialized as camelCase JSON.

use ac_domain::exchange::Response;
use ac_domain::protocol::{StatusClass, header};
use serde::Serialize;

use crate::body::{BodyView, preview};
use crate::text::UserText;

/// Result of one request run.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum RunOutcomeView {
    /// A response arrived (whatever its status).
    Completed(ResponseView),
    /// No response.
    Failed {
        /// Why.
        error: UserText,
    },
}

/// A response prepared for display.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct ResponseView {
    /// Status code, e.g. 404.
    pub status: u16,
    /// Standard reason phrase, if known.
    pub reason: Option<&'static str>,
    /// Class of the status, for colouring.
    pub status_class: StatusClassView,
    /// Duration in milliseconds.
    pub elapsed_ms: u64,
    /// Body size in bytes.
    pub size_bytes: u64,
    /// Headers in received order.
    pub headers: Vec<HeaderView>,
    /// Body preview.
    pub body: BodyView,
}

/// Class of a status code as the UI sees it (serialized in camelCase).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub enum StatusClassView {
    /// `1xx`
    Informational,
    /// `2xx`
    Success,
    /// `3xx`
    Redirection,
    /// `4xx`
    ClientError,
    /// `5xx`
    ServerError,
    /// `6xx`–`9xx`
    Nonstandard,
}

impl From<StatusClass> for StatusClassView {
    fn from(class: StatusClass) -> Self {
        match class {
            StatusClass::Informational => Self::Informational,
            StatusClass::Success => Self::Success,
            StatusClass::Redirection => Self::Redirection,
            StatusClass::ClientError => Self::ClientError,
            StatusClass::ServerError => Self::ServerError,
            StatusClass::Nonstandard => Self::Nonstandard,
        }
    }
}

/// One header line.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct HeaderView {
    /// Name as received.
    pub name: String,
    /// Value.
    pub value: String,
}

impl ResponseView {
    /// Prepares a response for display.
    #[must_use]
    pub fn from_response(response: &Response) -> Self {
        Self {
            status: response.status.as_u16(),
            reason: response.status.canonical_reason(),
            status_class: response.status.class().into(),
            elapsed_ms: u64::try_from(response.elapsed.as_millis()).unwrap_or(u64::MAX),
            size_bytes: u64::try_from(response.size()).unwrap_or(u64::MAX),
            headers: response
                .headers
                .iter()
                .map(|header| HeaderView {
                    name: header.name.clone(),
                    value: header.value.clone(),
                })
                .collect(),
            body: preview(response.header(header::CONTENT_TYPE), &response.body),
        }
    }
}

#[cfg(test)]
#[path = "tests/view.rs"]
mod tests;

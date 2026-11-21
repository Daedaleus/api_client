//! Responses prepared for display.

use ac_domain::exchange::Response;
use ac_domain::protocol::{StatusClass, header};
use serde::Serialize;

use super::body::{BodyView, preview};
use crate::ports::TestResult;
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
        /// Lines the pre-request script logged before.
        logs: Vec<String>,
        /// Tests the pre-request script ran before.
        tests: Vec<TestResultView>,
    },
}

/// The result of one `test(…)` of a script.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct TestResultView {
    /// The test's name.
    pub name: String,
    /// Whether every check in it held.
    pub passed: bool,
    /// Why it failed (technical, from the script).
    pub message: Option<String>,
}

impl TestResultView {
    /// The view of a script's test result.
    #[must_use]
    pub fn from_result(result: &TestResult) -> Self {
        Self {
            name: result.name.clone(),
            passed: result.passed,
            message: result.message.clone(),
        }
    }
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
    /// Lines the scripts logged, before and after the request.
    pub logs: Vec<String>,
    /// Why the post-response script failed, if it did.
    pub script_error: Option<UserText>,
    /// Tests of both scripts, in order.
    pub tests: Vec<TestResultView>,
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
            logs: Vec::new(),
            script_error: None,
            tests: Vec::new(),
        }
    }
}

#[cfg(test)]
#[path = "../tests/view_response.rs"]
mod tests;

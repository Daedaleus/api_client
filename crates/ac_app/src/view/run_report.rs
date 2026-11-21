//! The result of running requests one after another (command line runner).

use serde::Serialize;

use crate::text::UserText;

/// What a run did, request by request, in order.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RunReport {
    /// One entry per request.
    pub results: Vec<RunResult>,
}

/// The result of one request.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RunResult {
    /// Names of the folders around it and of the request itself.
    pub path: Vec<String>,
    /// Method.
    pub method: String,
    /// What happened.
    pub outcome: RunResultOutcome,
}

/// What happened to one request.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum RunResultOutcome {
    /// A response came back.
    Response {
        /// Status code.
        status: u16,
        /// Time until the whole response was received.
        elapsed_ms: u64,
    },
    /// The request could not be sent or no response arrived.
    Failed {
        /// Why.
        error: UserText,
    },
}

/// Status codes from here on count as failed, unless told otherwise.
const FIRST_ERROR_STATUS: u16 = 400;

impl RunResult {
    /// Whether the request passed: it got a response, and – if
    /// `error_status_fails` – one with a status below 400.
    #[must_use]
    pub fn passed(&self, error_status_fails: bool) -> bool {
        match self.outcome {
            RunResultOutcome::Response { status, .. } => {
                !error_status_fails || status < FIRST_ERROR_STATUS
            }
            RunResultOutcome::Failed { .. } => false,
        }
    }
}

impl RunReport {
    /// How many requests failed (see [`RunResult::passed`]).
    #[must_use]
    pub fn failed(&self, error_status_fails: bool) -> usize {
        self.results
            .iter()
            .filter(|result| !result.passed(error_status_fails))
            .count()
    }
}

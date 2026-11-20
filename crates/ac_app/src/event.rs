//! Events for subscribers of [`crate::App`].
//!
//! JSON shape: `{"type": "runFinished", "runId": 1, "outcome": {…}}`.

use serde::Serialize;

use crate::command::RunId;
use crate::view::RunOutcomeView;

/// Something happened that a UI may want to show.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(
    tag = "type",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum AppEvent {
    /// A request run ended, with a response or an error.
    RunFinished {
        /// The run from [`crate::CommandAck::RunStarted`].
        run_id: RunId,
        /// What happened.
        outcome: RunOutcomeView,
    },
}

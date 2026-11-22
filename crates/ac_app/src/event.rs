//! Events for subscribers of [`crate::App`].
//!
//! JSON shape: `{"type": "runFinished", "runId": 1, "outcome": {…}}`.

use serde::Serialize;

use crate::command::RunId;
use crate::text::UserText;
use crate::view::RunOutcomeView;

/// Something happened that a UI may want to show.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
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
    /// The request history changed (an entry was added or it was
    /// cleared).
    HistoryChanged {
        /// Why the newest request could not be recorded, if so.
        error: Option<UserText>,
    },
}

//! The request history as the UI lists it.

use serde::Serialize;

use crate::history::{HistoryEntry, HistoryOutcome};
use crate::text::UserText;

/// One entry of the history list.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct HistoryItemView {
    /// Entry id, to open it.
    pub id: String,
    /// When it was sent, in milliseconds since 1970.
    pub at_ms: u64,
    /// Method, e.g. `GET`.
    pub method: String,
    /// The URL as sent (secrets masked).
    pub url: String,
    /// Name of its collection, if any.
    pub collection: Option<String>,
    /// Status code of the response.
    pub status: Option<u16>,
    /// Time until the response arrived.
    pub elapsed_ms: Option<u64>,
    /// Why there was no response.
    pub error: Option<UserText>,
}

impl HistoryItemView {
    /// The list's view of `entry`.
    #[must_use]
    pub fn from_entry(entry: &HistoryEntry) -> Self {
        let (status, elapsed_ms) = match &entry.outcome {
            HistoryOutcome::Response {
                status, elapsed_ms, ..
            } => (Some(*status), Some(*elapsed_ms)),
            HistoryOutcome::Failed { .. } => (None, None),
        };
        Self {
            id: entry.id.0.clone(),
            at_ms: entry.at_ms,
            method: entry.request.method.clone(),
            url: entry.request.url.clone(),
            collection: entry.collection.clone(),
            status,
            elapsed_ms,
            error: entry.error_text(),
        }
    }
}

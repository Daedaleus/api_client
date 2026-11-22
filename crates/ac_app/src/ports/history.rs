//! Keeping the request history (see [`crate::history`]) – in the app's data
//! folder, never in a collection.

use std::sync::Arc;

use crate::history::HistoryEntry;
use crate::text::{UserText, keys};

/// Stores history entries, newest first, at most
/// [`crate::history::MAX_ENTRIES`].
pub trait HistoryStore: Send + Sync + 'static {
    /// Adds `entry`, stamped with the current time; drops the oldest
    /// entries beyond the limit.
    ///
    /// # Errors
    /// If the history cannot be written.
    fn record(&self, entry: HistoryEntry) -> Result<(), HistoryError>;

    /// All entries, newest first.
    ///
    /// # Errors
    /// If the history cannot be read.
    fn entries(&self) -> Result<Vec<HistoryEntry>, HistoryError>;

    /// Removes every entry.
    ///
    /// # Errors
    /// If the history cannot be removed.
    fn clear(&self) -> Result<(), HistoryError>;
}

impl<T: HistoryStore> HistoryStore for Arc<T> {
    fn record(&self, entry: HistoryEntry) -> Result<(), HistoryError> {
        T::record(self, entry)
    }

    fn entries(&self) -> Result<Vec<HistoryEntry>, HistoryError> {
        T::entries(self)
    }

    fn clear(&self) -> Result<(), HistoryError> {
        T::clear(self)
    }
}

/// Why the history could not be read or written.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("{0}")]
pub struct HistoryError(pub String);

impl HistoryError {
    /// The text to show the user.
    #[must_use]
    pub fn user_text(&self) -> UserText {
        UserText::new(keys::HISTORY).with("detail", self.0.clone())
    }
}

/// No history (for programs that keep none, like the command line runner).
#[derive(Debug, Clone, Copy, Default)]
pub struct NoHistory;

impl HistoryStore for NoHistory {
    fn record(&self, _entry: HistoryEntry) -> Result<(), HistoryError> {
        Ok(())
    }

    fn entries(&self) -> Result<Vec<HistoryEntry>, HistoryError> {
        Ok(Vec::new())
    }

    fn clear(&self) -> Result<(), HistoryError> {
        Ok(())
    }
}

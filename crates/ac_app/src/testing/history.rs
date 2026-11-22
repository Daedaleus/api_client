//! Test double for the request history.

use std::sync::Mutex;

use super::lock;
use crate::history::{HistoryEntry, MAX_ENTRIES};
use crate::ports::{HistoryError, HistoryStore};

/// A [`HistoryStore`] in memory. Entries are stamped 1, 2, 3 … instead of
/// the time; [`MemoryHistory::failing`] makes every call fail.
#[derive(Debug, Default)]
pub struct MemoryHistory {
    entries: Mutex<Vec<HistoryEntry>>,
    fail: Mutex<Option<String>>,
}

impl MemoryHistory {
    /// An empty history.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// From now on every call fails with `message`.
    pub fn failing(&self, message: &str) {
        *lock(&self.fail) = Some(message.to_owned());
    }

    fn check(&self) -> Result<(), HistoryError> {
        lock(&self.fail)
            .clone()
            .map_or(Ok(()), |message| Err(HistoryError(message)))
    }
}

impl HistoryStore for MemoryHistory {
    fn record(&self, mut entry: HistoryEntry) -> Result<(), HistoryError> {
        self.check()?;
        let mut entries = lock(&self.entries);
        entry.at_ms = entries.first().map_or(1, |newest| newest.at_ms + 1);
        entries.insert(0, entry);
        entries.truncate(MAX_ENTRIES);
        Ok(())
    }

    fn entries(&self) -> Result<Vec<HistoryEntry>, HistoryError> {
        self.check()?;
        Ok(lock(&self.entries).clone())
    }

    fn clear(&self) -> Result<(), HistoryError> {
        self.check()?;
        lock(&self.entries).clear();
        Ok(())
    }
}

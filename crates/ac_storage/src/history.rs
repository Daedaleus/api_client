//! [`FileHistory`]: the request history as a JSON Lines file in the app's
//! data folder (never in a collection).
//!
//! One line per entry, oldest first, each `{"version": 1, "entry": {…}}`, so
//! sending only appends a line. Once the file holds clearly more than
//! [`MAX_ENTRIES`], it is rewritten with the newest ones. Lines that cannot
//! be read (damaged, or from a newer version) are skipped when reading;
//! before such a file is rewritten it is kept as a backup.

use std::fs::OpenOptions;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, PoisonError};
use std::time::{SystemTime, UNIX_EPOCH};

use ac_app::history::{HistoryEntry, MAX_ENTRIES};
use ac_app::ports::{HistoryError, HistoryStore};
use serde::{Deserialize, Serialize};

use crate::document::write_atomically;

/// File name in the data folder.
pub const HISTORY_FILE: &str = "history.jsonl";
/// Version of the line format.
const VERSION: u32 = 1;
/// Lines beyond [`MAX_ENTRIES`] before the file is rewritten (rewriting on
/// every send would cost more than it saves).
const SLACK: usize = 50;

#[derive(Serialize, Deserialize)]
struct Line {
    version: u32,
    entry: HistoryEntry,
}

/// The request history in `<folder>/history.jsonl`.
#[derive(Debug)]
pub struct FileHistory {
    path: PathBuf,
    /// The file as read once and then kept up to date, so neither sending
    /// nor listing reads the whole file again. Also serializes writes.
    cache: Mutex<Option<Cache>>,
}

#[derive(Debug)]
struct Cache {
    /// Lines in the file, readable or not.
    lines: usize,
    /// Readable entries, newest first, at most [`MAX_ENTRIES`].
    entries: Vec<HistoryEntry>,
}

impl FileHistory {
    /// The history in `folder` (created when the first entry is written).
    #[must_use]
    pub fn new(folder: &Path) -> Self {
        Self {
            path: folder.join(HISTORY_FILE),
            cache: Mutex::new(None),
        }
    }

    /// The lines of the file (none if it does not exist yet).
    fn lines(&self) -> Result<Vec<String>, HistoryError> {
        match std::fs::read_to_string(&self.path) {
            Ok(text) => Ok(text
                .lines()
                .filter(|line| !line.trim().is_empty())
                .map(str::to_owned)
                .collect()),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(Vec::new()),
            Err(error) => Err(failed(&self.path, &error)),
        }
    }

    /// Reads the file into `cache` unless it is there already.
    fn load<'a>(&self, cache: &'a mut Option<Cache>) -> Result<&'a mut Cache, HistoryError> {
        if cache.is_none() {
            let lines = self.lines()?;
            let mut entries: Vec<HistoryEntry> =
                lines.iter().filter_map(|line| parse(line)).collect();
            entries.reverse();
            entries.truncate(MAX_ENTRIES);
            *cache = Some(Cache {
                lines: lines.len(),
                entries,
            });
        }
        Ok(cache.as_mut().expect("loaded above"))
    }

    /// Keeps the newest [`MAX_ENTRIES`] readable lines; a file with
    /// unreadable lines is backed up first. Returns the lines kept.
    fn trim(&self) -> Result<usize, HistoryError> {
        let lines = self.lines()?;
        let readable: Vec<&String> = lines.iter().filter(|line| parse(line).is_some()).collect();
        if readable.len() < lines.len() {
            let backup = self.path.with_extension(format!("{}.bak", now_ms()));
            std::fs::copy(&self.path, &backup).map_err(|error| failed(&backup, &error))?;
        }
        let keep = &readable[readable.len().saturating_sub(MAX_ENTRIES)..];
        let mut text = String::new();
        for line in keep {
            text.push_str(line);
            text.push('\n');
        }
        write_atomically(&self.path, &text).map_err(|error| HistoryError(error.to_string()))?;
        Ok(keep.len())
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, Option<Cache>> {
        // The cache is replaced whole or dropped; a panic leaves it usable
        // or empty (then it is read again).
        self.cache.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

impl HistoryStore for FileHistory {
    fn record(&self, mut entry: HistoryEntry) -> Result<(), HistoryError> {
        entry.at_ms = now_ms();
        let line = serde_json::to_string(&Line {
            version: VERSION,
            entry: entry.clone(),
        })
        .map_err(|error| HistoryError(error.to_string()))?;
        let mut guard = self.lock();
        let cache = self.load(&mut guard)?;
        if let Some(folder) = self.path.parent() {
            std::fs::create_dir_all(folder).map_err(|error| failed(folder, &error))?;
        }
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)
            .map_err(|error| failed(&self.path, &error))?;
        writeln!(file, "{line}").map_err(|error| failed(&self.path, &error))?;
        drop(file);
        cache.lines += 1;
        cache.entries.insert(0, entry);
        cache.entries.truncate(MAX_ENTRIES);
        if cache.lines > MAX_ENTRIES + SLACK {
            match self.trim() {
                Ok(kept) => cache.lines = kept,
                Err(error) => {
                    // Read anew next time: the file may be half-way.
                    *guard = None;
                    return Err(error);
                }
            }
        }
        Ok(())
    }

    fn entries(&self) -> Result<Vec<HistoryEntry>, HistoryError> {
        let mut guard = self.lock();
        Ok(self.load(&mut guard)?.entries.clone())
    }

    fn clear(&self) -> Result<(), HistoryError> {
        let mut guard = self.lock();
        *guard = None;
        match std::fs::remove_file(&self.path) {
            Ok(()) => Ok(()),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(error) => Err(failed(&self.path, &error)),
        }
    }
}

/// The entry of a line of the current version.
fn parse(line: &str) -> Option<HistoryEntry> {
    serde_json::from_str::<Line>(line)
        .ok()
        .filter(|line| line.version == VERSION)
        .map(|line| line.entry)
}

fn failed(path: &Path, error: &std::io::Error) -> HistoryError {
    HistoryError(format!("{}: {error}", path.display()))
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |since| {
            u64::try_from(since.as_millis()).unwrap_or(u64::MAX)
        })
}

#[cfg(test)]
#[path = "tests/history.rs"]
mod tests;

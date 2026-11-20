//! Removing requests and folders. Never deletes for good: the caller passes
//! how to discard (the system trash in the app, a test folder in tests).

use std::path::Path;

use crate::error::StorageError;
use crate::names::file_name;
use crate::order;

/// Discards `path` (a request file or folder directory) with `discard`,
/// then removes it from its parent's order. If discarding fails, nothing
/// has changed.
///
/// # Errors
/// If discarding fails or the parent's settings cannot be updated.
pub fn remove_item(
    path: &Path,
    discard: impl FnOnce(&Path) -> std::io::Result<()>,
) -> Result<(), StorageError> {
    discard(path).map_err(|error| StorageError::io(path, error))?;
    if let Some(parent) = path.parent() {
        order::remove(parent, &file_name(path))?;
    }
    Ok(())
}

#[cfg(test)]
#[path = "tests/remove.rs"]
mod tests;

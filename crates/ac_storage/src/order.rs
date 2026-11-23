//! Keeping the `order` list of a collection or folder in step with its
//! entries. Without settings or without `order` nothing changes: unlisted
//! entries are shown anyway (sorted by name), listed but missing ones are
//! skipped. Read-only settings are never written.

use std::path::{Path, PathBuf};

use toml_edit::{Array, Item, Value};

use crate::document;
use crate::error::StorageError;
use crate::format::{COLLECTION_FILE, COLLECTION_FORMAT, FOLDER_FILE, FOLDER_FORMAT};

/// Appends `name` to the order of `dir` unless it is listed already.
pub(crate) fn append(dir: &Path, name: &str) -> Result<(), StorageError> {
    edit(dir, |order| {
        let listed = order.iter().any(|entry| entry.as_str() == Some(name));
        if !listed {
            order.push(name);
        }
        !listed
    })
}

/// Removes `name` from the order of `dir`.
pub(crate) fn remove(dir: &Path, name: &str) -> Result<(), StorageError> {
    edit(dir, |order| {
        let before = order.len();
        order.retain(|entry| entry.as_str() != Some(name));
        order.len() != before
    })
}

/// Applies `change` to the order of `dir`; writes only if it reports a
/// change.
fn edit(dir: &Path, change: impl FnOnce(&mut Array) -> bool) -> Result<(), StorageError> {
    let Some((settings, format)) = settings(dir) else {
        return Ok(());
    };
    let mut parsed = document::read(&settings, format)?;
    if parsed.read_only() {
        return Ok(());
    }
    let Some(Item::Value(Value::Array(order))) = parsed.doc.get_mut("order") else {
        return Ok(());
    };
    if change(order) {
        document::write_atomically(&settings, parsed.doc.to_string())?;
    }
    Ok(())
}

/// The settings file of a collection or folder directory, if it has one.
fn settings(dir: &Path) -> Option<(PathBuf, &'static str)> {
    [
        (COLLECTION_FILE, COLLECTION_FORMAT),
        (FOLDER_FILE, FOLDER_FORMAT),
    ]
    .into_iter()
    .map(|(file, format)| (dir.join(file), format))
    .find(|(path, _)| path.is_file())
}

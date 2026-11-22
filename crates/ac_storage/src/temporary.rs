//! Collections that are not saved yet (imports): they live in a folder of
//! their own below the system's temp folder until the user saves them
//! somewhere, which moves the folder, or closes them, which deletes it.
//! Only folders below that place are ever deleted for good.

use std::path::{Path, PathBuf};

use ac_domain::collection::ItemId;

use crate::error::StorageError;

/// The folder below the temp folder that holds unsaved collections.
pub const TEMPORARY_DIR: &str = "api-client-unsaved";

/// A new, unused folder for an unsaved collection below `base`.
#[must_use]
pub fn temporary_root(base: &Path) -> PathBuf {
    base.join(TEMPORARY_DIR).join(ItemId::new().to_string())
}

/// Moves the collection folder `from` to `to`, which must not exist yet or
/// be an empty folder. Across drives the folder is copied, then removed.
///
/// # Errors
/// If `to` holds something or a file cannot be moved.
pub fn move_collection(from: &Path, to: &Path) -> Result<(), StorageError> {
    if to.exists() {
        let empty = std::fs::read_dir(to)
            .map_err(|error| StorageError::io(to, error))?
            .next()
            .is_none();
        if !empty {
            return Err(StorageError::AlreadyExists {
                path: to.to_owned(),
            });
        }
        std::fs::remove_dir(to).map_err(|error| StorageError::io(to, error))?;
    }
    if let Some(parent) = to.parent() {
        std::fs::create_dir_all(parent).map_err(|error| StorageError::io(parent, error))?;
    }
    if std::fs::rename(from, to).is_ok() {
        return Ok(());
    }
    copy_dir(from, to)?;
    std::fs::remove_dir_all(from).map_err(|error| StorageError::io(from, error))
}

fn copy_dir(from: &Path, to: &Path) -> Result<(), StorageError> {
    std::fs::create_dir_all(to).map_err(|error| StorageError::io(to, error))?;
    for entry in std::fs::read_dir(from).map_err(|error| StorageError::io(from, error))? {
        let entry = entry.map_err(|error| StorageError::io(from, error))?;
        let target = to.join(entry.file_name());
        let kind = entry
            .file_type()
            .map_err(|error| StorageError::io(entry.path(), error))?;
        if kind.is_dir() {
            copy_dir(&entry.path(), &target)?;
        } else {
            std::fs::copy(entry.path(), &target)
                .map_err(|error| StorageError::io(entry.path(), error))?;
        }
    }
    Ok(())
}

/// Deletes the unsaved collection at `root` for good – but only if it lies
/// below `base`'s folder for unsaved collections, so no saved collection
/// can ever be deleted this way.
///
/// # Errors
/// If `root` is somewhere else or cannot be deleted.
pub fn discard(base: &Path, root: &Path) -> Result<(), StorageError> {
    let allowed = base.join(TEMPORARY_DIR);
    let inside = root.starts_with(&allowed)
        && root != allowed
        && !root
            .components()
            .any(|part| matches!(part, std::path::Component::ParentDir));
    if !inside {
        return Err(StorageError::invalid(
            root,
            "collection",
            "not an unsaved collection",
        ));
    }
    match std::fs::remove_dir_all(root) {
        Err(error) if error.kind() != std::io::ErrorKind::NotFound => {
            Err(StorageError::io(root, error))
        }
        _ => Ok(()),
    }
}

#[cfg(test)]
#[path = "tests/temporary.rs"]
mod tests;

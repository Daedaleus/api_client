//! Moving requests and folders, within a folder (reordering) or into
//! another folder of the collection: the file or directory is renamed and
//! the `order` lists of both folders are written.

use std::path::{Path, PathBuf};

use toml_edit::{Array, Item, Value};

use crate::document;
use crate::error::StorageError;
use crate::format::{COLLECTION_FILE, COLLECTION_FORMAT, FOLDER_FILE, FOLDER_FORMAT};
use crate::load::entry_names;
use crate::names::{file_name, file_stem, unique_dir, unique_file};
use crate::settings::save_folder_settings;

/// Moves the request file or folder `path` into `to_dir` (the collection
/// folder or one of its folders) at `position` among its entries, as shown.
/// Returns where it is now.
///
/// A folder without `folder.toml` gets one first, so its id survives the
/// move (ids of such folders are derived from their path). In another
/// folder the item gets a free name if its own is taken.
///
/// # Errors
/// If a folder is moved into itself, a settings file is damaged or
/// read-only, or the file system refuses.
pub fn move_item(path: &Path, to_dir: &Path, position: usize) -> Result<PathBuf, StorageError> {
    let from_dir = path
        .parent()
        .ok_or_else(|| StorageError::invalid(path, "path", "has no folder"))?;
    if to_dir.starts_with(path) {
        return Err(StorageError::invalid(
            path,
            "path",
            "a folder cannot be moved into itself",
        ));
    }
    let moved = if from_dir == to_dir {
        path.to_owned()
    } else {
        if path.is_dir() {
            keep_identity(path)?;
        }
        let target = if path.is_dir() {
            unique_dir(to_dir, &file_name(path))
        } else {
            unique_file(to_dir, &file_stem(path))
        };
        std::fs::rename(path, &target).map_err(|error| StorageError::io(path, error))?;
        // A request's examples follow it.
        let examples = crate::examples_file::path_for(path);
        if !path.is_dir() && examples.is_file() {
            let to = crate::examples_file::path_for(&target);
            std::fs::rename(&examples, &to).map_err(|error| StorageError::io(&examples, error))?;
        }
        let name = file_name(path);
        write_order(from_dir, |order| order.retain(|entry| *entry != name))?;
        target
    };
    let name = file_name(&moved);
    write_order(to_dir, |order| {
        order.retain(|entry| *entry != name);
        order.insert(position.min(order.len()), name.clone());
    })?;
    Ok(moved)
}

/// Gives a folder without settings a `folder.toml` with the id it has now.
fn keep_identity(dir: &Path) -> Result<(), StorageError> {
    if dir.join(FOLDER_FILE).is_file() {
        return Ok(());
    }
    save_folder_settings(dir, document::derived_id(dir), &file_name(dir))
}

/// Applies `change` to the entries of `dir` as shown and writes them as its
/// `order`, creating the settings or the key if needed.
fn write_order(dir: &Path, change: impl FnOnce(&mut Vec<String>)) -> Result<(), StorageError> {
    let is_root = dir.join(COLLECTION_FILE).is_file();
    let (file, format) = if is_root {
        (COLLECTION_FILE, COLLECTION_FORMAT)
    } else {
        keep_identity(dir)?;
        (FOLDER_FILE, FOLDER_FORMAT)
    };
    let settings = dir.join(file);
    let mut parsed = document::read(&settings, format)?;
    if parsed.read_only() {
        return Err(StorageError::ReadOnly {
            path: settings,
            version: parsed.version,
        });
    }
    let listed = document::order(parsed.doc.as_table(), &settings)?;
    let mut order = shown(&entry_names(dir, is_root)?, &listed);
    change(&mut order);
    let array: Array = order.iter().map(String::as_str).collect();
    parsed.doc.insert("order", Item::Value(Value::Array(array)));
    document::write_atomically(&settings, parsed.doc.to_string())
}

/// The entries in the order they are shown: listed ones first (if they
/// exist), then the rest by name (see `load`).
fn shown(names: &[String], listed: &[String]) -> Vec<String> {
    let mut order: Vec<String> = Vec::with_capacity(names.len());
    for name in listed {
        if names.contains(name) && !order.contains(name) {
            order.push(name.clone());
        }
    }
    for name in names {
        if !order.contains(name) {
            order.push(name.clone());
        }
    }
    order
}

#[cfg(test)]
#[path = "tests/arrange.rs"]
mod tests;

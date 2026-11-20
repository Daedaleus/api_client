//! Creating collections, folders and requests: names from display names,
//! new entries appended to the folder's order.

use std::path::{Path, PathBuf};

use ac_domain::collection::{ItemId, Request};
use toml_edit::DocumentMut;

use crate::document;
use crate::error::StorageError;
use crate::format::{COLLECTION_FILE, COLLECTION_FORMAT, CURRENT_VERSION};
use crate::load::{LoadedCollection, load_collection};
use crate::names::{file_name, slug, unique_dir, unique_file};
use crate::order;
use crate::request_file::save_request;
use crate::settings::save_folder_settings;

/// Name stem when a request name has no usable characters.
const REQUEST_FALLBACK: &str = "request";
/// Name stem when a folder name has no usable characters.
const FOLDER_FALLBACK: &str = "folder";

/// Creates a new, empty collection in `root` (the folder is created if
/// needed) and loads it.
///
/// # Errors
/// [`StorageError::AlreadyExists`] if `root` already holds a collection, or
/// the folder or file cannot be written.
pub fn create_collection(root: &Path, name: &str) -> Result<LoadedCollection, StorageError> {
    let settings = root.join(COLLECTION_FILE);
    if settings.exists() {
        return Err(StorageError::AlreadyExists {
            path: root.to_owned(),
        });
    }
    std::fs::create_dir_all(root).map_err(|error| StorageError::io(root, error))?;
    let mut doc = DocumentMut::new();
    doc.insert("format", toml_edit::value(COLLECTION_FORMAT));
    doc.insert("version", toml_edit::value(CURRENT_VERSION));
    doc.insert("id", toml_edit::value(ItemId::new().to_string()));
    doc.insert("name", toml_edit::value(name));
    document::write_atomically(&settings, &doc.to_string())?;
    load_collection(root)
}

/// Creates a file for `request` in `dir` (a collection or folder
/// directory), named after the request. Returns the new file.
///
/// # Errors
/// If the file or the folder settings cannot be written.
pub fn create_request(dir: &Path, request: &Request) -> Result<PathBuf, StorageError> {
    let path = unique_file(dir, &slug(&request.name, REQUEST_FALLBACK));
    save_request(&path, request)?;
    order::append(dir, &file_name(&path))?;
    Ok(path)
}

/// Creates a folder named after `name` in `parent` with its settings file.
/// Returns the new directory and the folder's id.
///
/// # Errors
/// If the directory or a settings file cannot be written.
pub fn create_folder(parent: &Path, name: &str) -> Result<(PathBuf, ItemId), StorageError> {
    let dir = unique_dir(parent, &slug(name, FOLDER_FALLBACK));
    std::fs::create_dir(&dir).map_err(|error| StorageError::io(&dir, error))?;
    let id = ItemId::new();
    save_folder_settings(&dir, id, name)?;
    order::append(parent, &file_name(&dir))?;
    Ok((dir, id))
}

#[cfg(test)]
#[path = "tests/create.rs"]
mod tests;

//! Creating collections, folders and requests: names from display names,
//! new entries appended to the folder's order.

use std::path::{Path, PathBuf};

use ac_domain::collection::{Collection, Item, ItemId, Request, RequestDefaults};
use toml_edit::DocumentMut;

use ac_domain::environment::Environment;

use crate::document;
use crate::environment_file::create_environment;
use crate::error::StorageError;
use crate::format::{COLLECTION_FILE, COLLECTION_FORMAT, CURRENT_VERSION, RESERVED_ROOT_FOLDERS};
use crate::load::{LoadedCollection, load_collection};
use crate::names::{file_name, slug, unique_dir, unique_file};
use crate::order;
use crate::request_file::save_request;
use crate::settings::{save_collection_defaults, save_folder_defaults, save_folder_settings};

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
    document::write_atomically(&settings, doc.to_string())?;
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
    let mut stem = slug(name, FOLDER_FALLBACK);
    // `environments` and `secrets` in the collection root are no request
    // folders: a folder of that name would vanish from the tree.
    if parent.join(COLLECTION_FILE).is_file() && RESERVED_ROOT_FOLDERS.contains(&stem.as_str()) {
        stem.push_str("-folder");
    }
    let dir = unique_dir(parent, &stem);
    std::fs::create_dir(&dir).map_err(|error| StorageError::io(&dir, error))?;
    let id = ItemId::new();
    save_folder_settings(&dir, id, name)?;
    order::append(parent, &file_name(&dir))?;
    Ok((dir, id))
}

/// Writes `collection` as a new collection into `root` (imports): its
/// settings, every folder and request in order, then its environments. Ids of folders and
/// requests are kept; the collection gets a new one.
///
/// # Errors
/// [`StorageError::AlreadyExists`] if `root` already holds a collection, or
/// a file cannot be written (what was written so far stays).
pub fn write_collection(
    root: &Path,
    collection: &Collection,
    environments: &[Environment],
) -> Result<LoadedCollection, StorageError> {
    create_collection(root, &collection.name)?;
    save_collection_defaults(
        root,
        &RequestDefaults {
            transport: collection.transport,
            scripts: collection.scripts.clone(),
            variables: collection.variables.clone(),
            auth: collection.auth.clone(),
            certificate_check: collection.certificate_check,
            docs: String::new(),
        },
    )?;
    write_items(root, &collection.items)?;
    for environment in environments {
        create_environment(root, environment)?;
    }
    load_collection(root)
}

fn write_items(dir: &Path, items: &[Item]) -> Result<(), StorageError> {
    for item in items {
        match item {
            Item::Request(request) => {
                create_request(dir, request)?;
            }
            Item::Folder(folder) => {
                let (path, id) = create_folder(dir, &folder.name)?;
                let defaults = RequestDefaults {
                    transport: folder.transport,
                    scripts: folder.scripts.clone(),
                    variables: folder.variables.clone(),
                    auth: folder.auth.clone(),
                    certificate_check: folder.certificate_check,
                    docs: String::new(),
                };
                save_folder_defaults(&path, id, &folder.name, &defaults)?;
                write_items(&path, &folder.items)?;
            }
        }
    }
    Ok(())
}

#[cfg(test)]
#[path = "tests/create.rs"]
mod tests;

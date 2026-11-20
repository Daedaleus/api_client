//! Settings files written in place: `folder.toml` (name, id, variables)
//! and the variables in `collection.toml`. Other values (order, comments,
//! unknown keys) stay.

use std::path::Path;

use ac_domain::collection::ItemId;
use ac_domain::environment::Variable;
use toml_edit::DocumentMut;

use crate::document;
use crate::error::StorageError;
use crate::format::{
    COLLECTION_FILE, COLLECTION_FORMAT, CURRENT_VERSION, FOLDER_FILE, FOLDER_FORMAT,
};
use crate::variables;

/// Writes name and id of the folder `dir` into its `folder.toml`, creating
/// the file if needed.
///
/// # Errors
/// If the settings are damaged or from a newer version (left untouched), or
/// cannot be written.
pub fn save_folder_settings(dir: &Path, id: ItemId, name: &str) -> Result<(), StorageError> {
    edit_folder(dir, id, name, |_| Ok(()))
}

/// Writes the variables of the folder `dir` (with its name and id, so a new
/// `folder.toml` keeps the folder's identity).
///
/// # Errors
/// As [`save_folder_settings`].
pub fn save_folder_variables(
    dir: &Path,
    id: ItemId,
    name: &str,
    new: &[Variable],
) -> Result<(), StorageError> {
    let settings = dir.join(FOLDER_FILE);
    edit_folder(dir, id, name, |doc| {
        let before = variables::read(doc.get(variables::KEY), &settings, variables::KEY)?;
        variables::write(doc.as_table_mut(), &before, new);
        Ok(())
    })
}

/// Writes the variables of the collection in `root` into its
/// `collection.toml` (which must exist).
///
/// # Errors
/// If the file is missing, damaged, from a newer version or cannot be
/// written.
pub fn save_collection_variables(root: &Path, new: &[Variable]) -> Result<(), StorageError> {
    let settings = root.join(COLLECTION_FILE);
    let text =
        std::fs::read_to_string(&settings).map_err(|error| StorageError::io(&settings, error))?;
    let parsed = document::parse(&settings, &text, COLLECTION_FORMAT)?;
    if parsed.read_only() {
        return Err(StorageError::ReadOnly {
            path: settings,
            version: parsed.version,
        });
    }
    let mut doc = parsed.doc;
    let before = variables::read(doc.get(variables::KEY), &settings, variables::KEY)?;
    variables::write(doc.as_table_mut(), &before, new);
    document::write_atomically(&settings, &doc.to_string())
}

fn edit_folder(
    dir: &Path,
    id: ItemId,
    name: &str,
    change: impl FnOnce(&mut DocumentMut) -> Result<(), StorageError>,
) -> Result<(), StorageError> {
    let settings = dir.join(FOLDER_FILE);
    let mut doc = match std::fs::read_to_string(&settings) {
        Ok(text) => {
            let parsed = document::parse(&settings, &text, FOLDER_FORMAT)?;
            if parsed.read_only() {
                return Err(StorageError::ReadOnly {
                    path: settings,
                    version: parsed.version,
                });
            }
            parsed.doc
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            let mut doc = DocumentMut::new();
            doc.insert("format", toml_edit::value(FOLDER_FORMAT));
            doc
        }
        Err(error) => return Err(StorageError::io(&settings, error)),
    };
    let root = doc.as_table_mut();
    document::set(root, "version", CURRENT_VERSION);
    document::set(root, "id", id.to_string());
    document::set(root, "name", name);
    change(&mut doc)?;
    document::write_atomically(&settings, &doc.to_string())
}

#[cfg(test)]
#[path = "tests/settings.rs"]
mod tests;

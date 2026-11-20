//! Folder settings (`folder.toml`): name and id, written in place.

use std::path::Path;

use ac_domain::collection::ItemId;
use toml_edit::DocumentMut;

use crate::document;
use crate::error::StorageError;
use crate::format::{CURRENT_VERSION, FOLDER_FILE, FOLDER_FORMAT};

/// Writes name and id of the folder `dir` into its `folder.toml`, creating
/// the file if needed. Other values (order, comments) stay.
///
/// # Errors
/// If the settings are damaged or from a newer version (left untouched), or
/// cannot be written.
pub fn save_folder_settings(dir: &Path, id: ItemId, name: &str) -> Result<(), StorageError> {
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
    document::write_atomically(&settings, &doc.to_string())
}

#[cfg(test)]
#[path = "tests/settings.rs"]
mod tests;

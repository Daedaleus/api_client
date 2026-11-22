//! Settings files written in place: `folder.toml` (name, id, what it
//! passes on) and what `collection.toml` passes on. Other values (order,
//! comments, unknown keys) stay.

use std::path::Path;

use ac_domain::collection::{ItemId, RequestDefaults};
use toml_edit::DocumentMut;

use crate::document;
use crate::error::StorageError;
use crate::format::{
    COLLECTION_FILE, COLLECTION_FORMAT, CURRENT_VERSION, FOLDER_FILE, FOLDER_FORMAT,
};
use crate::{auth, scripts, tls, transport, variables};

/// Writes name and id of the folder `dir` into its `folder.toml`, creating
/// the file if needed.
///
/// # Errors
/// If the settings are damaged or from a newer version (left untouched), or
/// cannot be written.
pub fn save_folder_settings(dir: &Path, id: ItemId, name: &str) -> Result<(), StorageError> {
    edit_folder(dir, id, name, |_| Ok(()))
}

/// Writes what the folder `dir` passes on to its requests (with its name
/// and id, so a new `folder.toml` keeps the folder's identity).
///
/// # Errors
/// As [`save_folder_settings`].
pub fn save_folder_defaults(
    dir: &Path,
    id: ItemId,
    name: &str,
    defaults: &RequestDefaults,
) -> Result<(), StorageError> {
    let settings = dir.join(FOLDER_FILE);
    edit_folder(dir, id, name, |doc| {
        write_defaults(doc, &settings, defaults)
    })
}

/// Writes what the collection in `root` passes on to its requests into its
/// `collection.toml` (which must exist).
///
/// # Errors
/// If the file is missing, damaged, from a newer version or cannot be
/// written.
pub fn save_collection_defaults(
    root: &Path,
    defaults: &RequestDefaults,
) -> Result<(), StorageError> {
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
    write_defaults(&mut doc, &settings, defaults)?;
    document::write_atomically(&settings, &doc.to_string())
}

/// Updates variables, authentication, certificate check and scripts in a
/// settings document.
fn write_defaults(
    doc: &mut DocumentMut,
    settings: &Path,
    defaults: &RequestDefaults,
) -> Result<(), StorageError> {
    let before_variables = variables::read(doc.get(variables::KEY), settings, variables::KEY)?;
    let before_auth = auth::read(doc.get(auth::KEY), settings, auth::KEY)?;
    let before_check = tls::read(doc.as_table(), settings)?;
    let before_transport = transport::read(doc.as_table(), settings)?;
    let before_scripts = scripts::read(doc.get(scripts::KEY), settings, scripts::KEY)?;
    let root = doc.as_table_mut();
    variables::write(root, &before_variables, &defaults.variables);
    auth::write(root, &before_auth, &defaults.auth);
    tls::write(root, before_check, defaults.certificate_check);
    transport::write(root, &before_transport, &defaults.transport);
    scripts::write(root, &before_scripts, &defaults.scripts);
    Ok(())
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

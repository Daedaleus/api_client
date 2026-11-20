//! Environment files (`environments/<name>.toml`): reading, writing in place
//! and creating them.

use std::path::{Path, PathBuf};

use ac_domain::environment::{Environment, Variable};
use toml_edit::DocumentMut;

use crate::document;
use crate::error::StorageError;
use crate::format::{CURRENT_VERSION, ENVIRONMENT_FORMAT, ENVIRONMENTS_FOLDER};
use crate::names::{file_stem, slug, unique_file};
use crate::rows::{self, Row};

const VARIABLES: &str = "variables";
/// Name stem when an environment name has no usable characters.
const FALLBACK: &str = "environment";

/// Reads an environment file; the flag says whether it is read-only.
///
/// # Errors
/// If the file cannot be read or is not a valid environment file.
pub fn read_environment(path: &Path) -> Result<(Environment, bool), StorageError> {
    let parsed = document::read(path, ENVIRONMENT_FORMAT)?;
    let environment = environment_from(&parsed.doc, path)?;
    Ok((environment, parsed.read_only()))
}

fn environment_from(doc: &DocumentMut, path: &Path) -> Result<Environment, StorageError> {
    let root = doc.as_table();
    Ok(Environment {
        id: document::id(root, path, path)?,
        name: document::opt_str(root, "name", path)?.unwrap_or_else(|| file_stem(path)),
        variables: rows::read(root.get(VARIABLES), path, VARIABLES)?
            .into_iter()
            .map(|row| Variable {
                name: row.name,
                value: row.value,
                enabled: row.enabled,
            })
            .collect(),
    })
}

/// Writes `environment` to `path`, changing only what differs (comments and
/// unknown keys stay). Atomic, like every write.
///
/// # Errors
/// If the existing file is damaged or from a newer version (left untouched),
/// or the file cannot be written.
pub fn save_environment(path: &Path, environment: &Environment) -> Result<(), StorageError> {
    let mut doc = match std::fs::read_to_string(path) {
        Ok(text) => {
            let parsed = document::parse(path, &text, ENVIRONMENT_FORMAT)?;
            if parsed.read_only() {
                return Err(StorageError::ReadOnly {
                    path: path.to_owned(),
                    version: parsed.version,
                });
            }
            parsed.doc
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            let mut doc = DocumentMut::new();
            doc.insert("format", toml_edit::value(ENVIRONMENT_FORMAT));
            doc
        }
        Err(error) => return Err(StorageError::io(path, error)),
    };
    let before = environment_from(&doc, path)?;
    let root = doc.as_table_mut();
    document::set(root, "version", CURRENT_VERSION);
    document::set(root, "id", environment.id.to_string());
    document::set(root, "name", environment.name.as_str());
    rows::write(
        root,
        VARIABLES,
        &variable_rows(&before.variables),
        &variable_rows(&environment.variables),
    );
    document::write_atomically(path, &doc.to_string())
}

/// Creates a file for `environment` in the collection's `environments`
/// folder (created if needed). Returns the new file.
///
/// # Errors
/// If the folder or file cannot be written.
pub fn create_environment(root: &Path, environment: &Environment) -> Result<PathBuf, StorageError> {
    let dir = root.join(ENVIRONMENTS_FOLDER);
    std::fs::create_dir_all(&dir).map_err(|error| StorageError::io(&dir, error))?;
    let path = unique_file(&dir, &slug(&environment.name, FALLBACK));
    save_environment(&path, environment)?;
    Ok(path)
}

fn variable_rows(variables: &[Variable]) -> Vec<Row> {
    variables
        .iter()
        .map(|variable| Row {
            name: variable.name.clone(),
            value: variable.value.clone(),
            enabled: variable.enabled,
        })
        .collect()
}

#[cfg(test)]
#[path = "tests/environment_file.rs"]
mod tests;

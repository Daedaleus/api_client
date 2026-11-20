//! Creating collections and requests: file names from display names, order
//! kept in the folder settings.

use std::path::{Path, PathBuf};

use ac_domain::collection::{ItemId, Request};
use toml_edit::{DocumentMut, Item, Value};

use crate::document;
use crate::error::StorageError;
use crate::format::{
    COLLECTION_FILE, COLLECTION_FORMAT, CURRENT_VERSION, EXTENSION, FOLDER_FILE, FOLDER_FORMAT,
};
use crate::load::{LoadedCollection, load_collection};
use crate::request_file::{save_request, write_atomically};

/// Longest file name stem created from a display name.
const MAX_SLUG: usize = 60;
/// Stem used when a name has no usable characters.
const FALLBACK_SLUG: &str = "request";
/// Names Windows does not allow as file names.
const WINDOWS_RESERVED: [&str; 22] = [
    "con", "prn", "aux", "nul", "com1", "com2", "com3", "com4", "com5", "com6", "com7", "com8",
    "com9", "lpt1", "lpt2", "lpt3", "lpt4", "lpt5", "lpt6", "lpt7", "lpt8", "lpt9",
];

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
    write_atomically(&settings, &doc.to_string())?;
    load_collection(root)
}

/// Creates a file for `request` in `dir` (a collection or folder
/// directory), named after the request, and appends it to the folder's
/// `order` if it has one. Returns the new file.
///
/// # Errors
/// If the file or the folder settings cannot be written.
pub fn create_request(dir: &Path, request: &Request) -> Result<PathBuf, StorageError> {
    let path = unique_path(dir, &slug(&request.name));
    save_request(&path, request)?;
    if let Some(name) = path.file_name() {
        append_to_order(dir, &name.to_string_lossy())?;
    }
    Ok(path)
}

/// File name stem for a display name: lower case ASCII letters and digits,
/// `-` between words, German umlauts transliterated.
#[must_use]
pub fn slug(name: &str) -> String {
    let mut slug = String::new();
    for c in name.chars().flat_map(char::to_lowercase) {
        match c {
            'ä' => slug.push_str("ae"),
            'ö' => slug.push_str("oe"),
            'ü' => slug.push_str("ue"),
            'ß' => slug.push_str("ss"),
            c if c.is_ascii_alphanumeric() => slug.push(c),
            _ if !slug.ends_with('-') => slug.push('-'),
            _ => {}
        }
    }
    let mut slug = slug.trim_matches('-').to_owned();
    // Only ASCII is left, so cutting at a byte index is safe.
    slug.truncate(MAX_SLUG);
    let slug = slug.trim_end_matches('-').to_owned();
    if slug.is_empty() {
        FALLBACK_SLUG.to_owned()
    } else if WINDOWS_RESERVED.contains(&slug.as_str()) {
        format!("{FALLBACK_SLUG}-{slug}")
    } else {
        slug
    }
}

/// `stem.toml`, or `stem-2.toml`, `stem-3.toml` … if taken.
fn unique_path(dir: &Path, stem: &str) -> PathBuf {
    let mut path = dir.join(format!("{stem}.{EXTENSION}"));
    let mut number = 2;
    while path.exists() {
        path = dir.join(format!("{stem}-{number}.{EXTENSION}"));
        number += 1;
    }
    path
}

/// Appends `name` to the `order` of the collection or folder settings in
/// `dir`. Without settings or without `order`, nothing changes: unlisted
/// entries are shown anyway.
fn append_to_order(dir: &Path, name: &str) -> Result<(), StorageError> {
    let (settings, format) = if dir.join(COLLECTION_FILE).is_file() {
        (dir.join(COLLECTION_FILE), COLLECTION_FORMAT)
    } else if dir.join(FOLDER_FILE).is_file() {
        (dir.join(FOLDER_FILE), FOLDER_FORMAT)
    } else {
        return Ok(());
    };
    let mut parsed = document::read(&settings, format)?;
    if parsed.read_only() {
        return Ok(());
    }
    let Some(Item::Value(Value::Array(order))) = parsed.doc.get_mut("order") else {
        return Ok(());
    };
    if !order.iter().any(|entry| entry.as_str() == Some(name)) {
        order.push(name);
        write_atomically(&settings, &parsed.doc.to_string())?;
    }
    Ok(())
}

#[cfg(test)]
#[path = "tests/create.rs"]
mod tests;

//! Reading and updating TOML values in place, so that comments, key order
//! and unknown keys of hand-edited files survive saving.

use std::path::Path;

use ac_domain::collection::ItemId;
use toml_edit::{DocumentMut, Item, TableLike, Value};

use crate::error::StorageError;
use crate::format::CURRENT_VERSION;

/// A parsed file with its format version.
pub(crate) struct Parsed {
    pub doc: DocumentMut,
    pub version: i64,
}

impl Parsed {
    /// Written by a newer program version: show, but never write.
    pub fn read_only(&self) -> bool {
        self.version > CURRENT_VERSION
    }
}

/// Reads `path`, checks `format` and `version` and migrates older versions
/// in memory (version 1 is the first, so there is nothing to migrate yet).
pub(crate) fn read(path: &Path, expected_format: &'static str) -> Result<Parsed, StorageError> {
    let text = std::fs::read_to_string(path).map_err(|error| StorageError::io(path, error))?;
    parse(path, &text, expected_format)
}

pub(crate) fn parse(
    path: &Path,
    text: &str,
    expected_format: &'static str,
) -> Result<Parsed, StorageError> {
    let doc: DocumentMut =
        text.parse()
            .map_err(|error: toml_edit::TomlError| StorageError::Toml {
                path: path.to_owned(),
                message: error.to_string().trim().to_owned(),
            })?;
    let found = doc.get("format").and_then(Item::as_str);
    if found != Some(expected_format) {
        return Err(StorageError::WrongFormat {
            path: path.to_owned(),
            expected: expected_format,
            found: found.map(str::to_owned),
        });
    }
    let version = doc
        .get("version")
        .and_then(Item::as_integer)
        .filter(|version| *version >= 1)
        .ok_or_else(|| StorageError::invalid(path, "version", "must be a whole number ≥ 1"))?;
    Ok(Parsed { doc, version })
}

/// Optional string value.
pub(crate) fn opt_str(
    table: &dyn TableLike,
    key: &'static str,
    path: &Path,
) -> Result<Option<String>, StorageError> {
    match table.get(key) {
        None => Ok(None),
        Some(item) => item
            .as_str()
            .map(|text| Some(text.to_owned()))
            .ok_or_else(|| StorageError::invalid(path, key, "must be a string")),
    }
}

/// Optional boolean value.
pub(crate) fn opt_bool(
    table: &dyn TableLike,
    key: &'static str,
    path: &Path,
) -> Result<Option<bool>, StorageError> {
    match table.get(key) {
        None => Ok(None),
        Some(item) => item
            .as_bool()
            .map(Some)
            .ok_or_else(|| StorageError::invalid(path, key, "must be true or false")),
    }
}

/// The `id`; a new one if the file has none (written on the next save).
pub(crate) fn id(table: &dyn TableLike, path: &Path) -> Result<ItemId, StorageError> {
    match opt_str(table, "id", path)? {
        None => Ok(ItemId::new()),
        Some(text) => text
            .parse()
            .map_err(|error: ac_domain::collection::InvalidItemId| {
                StorageError::invalid(path, "id", error.to_string())
            }),
    }
}

/// The `order` list of a collection or folder file.
pub(crate) fn order(table: &dyn TableLike, path: &Path) -> Result<Vec<String>, StorageError> {
    let Some(item) = table.get("order") else {
        return Ok(Vec::new());
    };
    let invalid = || StorageError::invalid(path, "order", "must be an array of names");
    item.as_array()
        .ok_or_else(invalid)?
        .iter()
        .map(|value| value.as_str().map(str::to_owned).ok_or_else(invalid))
        .collect()
}

/// Sets a value only if it differs; keeps the comment after an existing
/// value.
pub(crate) fn set(table: &mut dyn TableLike, key: &str, new: impl Into<Value>) {
    let mut new = new.into();
    match table.get_mut(key) {
        Some(Item::Value(old)) => {
            if !same_value(old, &new) {
                *new.decor_mut() = old.decor().clone();
                *old = new;
            }
        }
        _ => {
            table.insert(key, Item::Value(new));
        }
    }
}

fn same_value(old: &Value, new: &Value) -> bool {
    match (old, new) {
        (Value::String(a), Value::String(b)) => a.value() == b.value(),
        (Value::Integer(a), Value::Integer(b)) => a.value() == b.value(),
        (Value::Boolean(a), Value::Boolean(b)) => a.value() == b.value(),
        _ => false,
    }
}

/// A string value; multi-line text as a literal block (`'''`) so it stays
/// readable in the file, otherwise a normal string.
pub(crate) fn text_value(text: &str) -> Value {
    if text.contains('\n')
        && !text.contains("'''")
        && let Ok(value) = format!("'''\n{text}'''").parse::<Value>()
        && value.as_str() == Some(text)
    {
        return value;
    }
    Value::from(text)
}

#[cfg(test)]
#[path = "tests/document.rs"]
mod tests;

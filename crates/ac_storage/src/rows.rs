//! Lists of `name`/`value`/`enabled` rows, as used by request headers and
//! environment variables:
//!
//! ```toml
//! [[request.headers]]
//! name = "Accept"
//! value = "application/json"
//! enabled = false        # optional, default true
//! ```
//!
//! Environment variables may be secret (`secret = true`); they have no
//! `value` in the file – it is stored encrypted elsewhere.
//!
//! Inline arrays (`headers = [{ name = "A", value = "b" }]`) are read too;
//! written are always arrays of tables, which diff best.

use std::path::Path;

use toml_edit::{ArrayOfTables, Item, Table, TableLike, Value};

use crate::document;
use crate::error::StorageError;

/// One row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Row {
    pub name: String,
    pub value: String,
    pub enabled: bool,
    /// Only variables: the value is stored encrypted, not in this file.
    pub secret: bool,
}

/// Reads the rows in `item` (missing = none). `field` names the list in
/// error messages, e.g. `request.headers`.
pub(crate) fn read(
    item: Option<&Item>,
    path: &Path,
    field: &'static str,
) -> Result<Vec<Row>, StorageError> {
    let invalid = || StorageError::invalid(path, field, "must be an array of tables");
    match item {
        None => Ok(Vec::new()),
        Some(Item::ArrayOfTables(tables)) => tables.iter().map(|table| row(table, path)).collect(),
        Some(Item::Value(Value::Array(values))) => values
            .iter()
            .map(|value| row(value.as_inline_table().ok_or_else(invalid)?, path))
            .collect(),
        Some(_) => Err(invalid()),
    }
}

fn row(table: &dyn TableLike, path: &Path) -> Result<Row, StorageError> {
    Ok(Row {
        name: document::opt_str(table, "name", path)?
            .ok_or_else(|| StorageError::invalid(path, "name", "every row needs a name"))?,
        value: document::opt_str(table, "value", path)?.unwrap_or_default(),
        enabled: document::opt_bool(table, "enabled", path)?.unwrap_or(true),
        secret: document::opt_bool(table, "secret", path)?.unwrap_or(false),
    })
}

/// Replaces the rows under `key` in `table`, but only if they changed, so an
/// unchanged list keeps its formatting and comments. No rows remove the key.
pub(crate) fn write(table: &mut dyn TableLike, key: &str, before: &[Row], rows: &[Row]) {
    if before == rows {
        return;
    }
    table.remove(key);
    if rows.is_empty() {
        return;
    }
    let mut tables = ArrayOfTables::new();
    for row in rows {
        let mut entry = Table::new();
        entry.insert("name", toml_edit::value(row.name.as_str()));
        if row.secret {
            entry.insert("secret", toml_edit::value(true));
        } else {
            entry.insert("value", toml_edit::value(row.value.as_str()));
        }
        entry.insert("enabled", toml_edit::value(row.enabled));
        tables.push(entry);
    }
    table.insert(key, Item::ArrayOfTables(tables));
}

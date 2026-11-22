//! The scripts table of a request file (`[request.scripts]`) and of
//! `folder.toml` / `collection.toml` (`[scripts]`):
//!
//! ```toml
//! [request.scripts]
//! pre_request = '''
//! req.setHeader("X-Trace", "abc");
//! '''
//! post_response = '''
//! env.set("token", res.json().token);
//! '''
//! ```
//!
//! Missing table or key = no script.

use std::path::Path;

use ac_domain::script::Scripts;
use toml_edit::{Item, Table, TableLike};

use crate::document;
use crate::error::StorageError;

/// Key of the table.
pub(crate) const KEY: &str = "scripts";
const PRE_REQUEST: &str = "pre_request";
const POST_RESPONSE: &str = "post_response";

/// Reads the scripts in `item` (missing = none); `name` is the table's
/// name in error messages.
pub(crate) fn read(
    item: Option<&Item>,
    path: &Path,
    name: &'static str,
) -> Result<Scripts, StorageError> {
    let Some(item) = item else {
        return Ok(Scripts::default());
    };
    let table = item
        .as_table_like()
        .ok_or_else(|| StorageError::invalid(path, name, "must be a table"))?;
    Ok(Scripts {
        pre_request: document::opt_str(table, PRE_REQUEST, path)?.unwrap_or_default(),
        post_response: document::opt_str(table, POST_RESPONSE, path)?.unwrap_or_default(),
    })
}

/// Replaces the scripts table in `table` if the scripts changed; no table
/// without scripts.
pub(crate) fn write(table: &mut dyn TableLike, before: &Scripts, after: &Scripts) {
    if before == after {
        return;
    }
    table.remove(KEY);
    if after.is_empty() {
        return;
    }
    let mut scripts = Table::new();
    for (key, text) in [
        (PRE_REQUEST, &after.pre_request),
        (POST_RESPONSE, &after.post_response),
    ] {
        if !text.trim().is_empty() {
            scripts.insert(key, toml_edit::value(text.as_str()));
        }
    }
    table.insert(KEY, Item::Table(scripts));
}

#[cfg(test)]
#[path = "tests/scripts.rs"]
mod tests;

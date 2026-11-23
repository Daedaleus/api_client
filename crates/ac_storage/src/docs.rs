//! The documentation of a request (`request.docs`), a folder or the
//! collection (`docs` in `folder.toml` / `collection.toml`): Markdown as a
//! multi-line string, so it diffs line by line.
//!
//! ```toml
//! [request]
//! docs = """
//! Creates an order. Needs `{{token}}` from *Login*.
//! """
//! ```
//!
//! Missing key = no documentation.

use std::path::Path;

use toml_edit::TableLike;

use crate::document;
use crate::error::StorageError;

/// Key of the text.
pub(crate) const KEY: &str = "docs";

/// The documentation in `table` (missing = empty).
pub(crate) fn read(table: &dyn TableLike, path: &Path) -> Result<String, StorageError> {
    Ok(document::opt_str(table, KEY, path)?.unwrap_or_default())
}

/// Writes the documentation into `table` if it changed; no key when empty.
pub(crate) fn write(table: &mut dyn TableLike, before: &str, after: &str) {
    if before == after {
        return;
    }
    if after.trim().is_empty() {
        table.remove(KEY);
    } else {
        document::set(table, KEY, after);
    }
}

#[cfg(test)]
#[path = "tests/docs.rs"]
mod tests;

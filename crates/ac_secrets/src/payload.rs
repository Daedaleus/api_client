//! The document inside an encrypted secrets file:
//!
//! ```toml
//! format = "api-client/secrets"
//! version = 1
//! [values]
//! token = "…"
//! ```

use ac_app::ports::{SecretError, SecretValues};
use toml_edit::{DocumentMut, Item, Table};

const FORMAT: &str = "api-client/secrets";
const VERSION: i64 = 1;
const VALUES: &str = "values";

/// The document for `values`.
pub(crate) fn encode(values: &SecretValues) -> String {
    let mut doc = DocumentMut::new();
    doc.insert("format", toml_edit::value(FORMAT));
    doc.insert("version", toml_edit::value(VERSION));
    let mut table = Table::new();
    for (name, value) in values {
        table.insert(name, toml_edit::value(value.as_str()));
    }
    doc.insert(VALUES, Item::Table(table));
    doc.to_string()
}

/// The values in a decrypted document.
///
/// # Errors
/// [`SecretError::Damaged`] if it is no secrets document of a known version.
pub(crate) fn decode(text: &str) -> Result<SecretValues, SecretError> {
    let damaged = |detail: &str| SecretError::Damaged(detail.to_owned());
    let doc: DocumentMut = text.parse().map_err(|_| damaged("not valid TOML"))?;
    if doc.get("format").and_then(Item::as_str) != Some(FORMAT) {
        return Err(damaged("not a secrets document"));
    }
    match doc.get("version").and_then(Item::as_integer) {
        Some(VERSION) => {}
        Some(version) if version > VERSION => return Err(damaged("written by a newer version")),
        _ => return Err(damaged("invalid version")),
    }
    let Some(table) = doc.get(VALUES) else {
        return Ok(SecretValues::new());
    };
    let table = table
        .as_table_like()
        .ok_or_else(|| damaged("values must be a table"))?;
    table
        .iter()
        .map(|(name, value)| {
            let value = value
                .as_str()
                .ok_or_else(|| damaged("values must be strings"))?;
            Ok((name.to_owned(), value.to_owned()))
        })
        .collect()
}

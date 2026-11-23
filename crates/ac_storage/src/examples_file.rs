//! The response examples of a request, in their own file next to it:
//! `login.toml` keeps its examples in `login.examples.toml`, so the request
//! file stays short however large the bodies are.
//!
//! ```toml
//! format = "api-client/examples"
//! version = 1
//!
//! [[example]]
//! name = "OK"
//! status = 200
//! body = """
//! {"token": "abc"}
//! """
//!
//! [[example.headers]]
//! name = "Content-Type"
//! value = "application/json"
//! ```
//!
//! No file means no examples. A file that cannot be read is moved aside
//! (`login.examples.toml.<time>.bak`) before new examples are written, never
//! overwritten.

use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use ac_domain::example::ResponseExample;
use ac_domain::exchange::Header;
use ac_domain::protocol::StatusCode;
use toml_edit::{ArrayOfTables, DocumentMut, Item, Table, value};

use crate::document;
use crate::error::StorageError;
use crate::format::{CURRENT_VERSION, EXAMPLES_FORMAT, EXAMPLES_SUFFIX, EXTENSION};

const EXAMPLE: &str = "example";
const NAME: &str = "name";
const STATUS: &str = "status";
const HEADERS: &str = "headers";
const VALUE: &str = "value";
const BODY: &str = "body";

/// The examples file of the request file `request` (`x.toml` →
/// `x.examples.toml`).
#[must_use]
pub fn path_for(request: &Path) -> PathBuf {
    let stem = request
        .file_stem()
        .map_or_default(|stem| stem.to_string_lossy().into_owned());
    request.with_file_name(format!("{stem}{EXAMPLES_SUFFIX}.{EXTENSION}"))
}

/// Whether `name` is an examples file (not a request).
#[must_use]
pub fn is_examples_file(name: &str) -> bool {
    name.ends_with(&format!("{EXAMPLES_SUFFIX}.{EXTENSION}"))
}

/// The examples of the request file `request` (none without a file).
///
/// # Errors
/// If the file exists but cannot be read or is damaged.
pub fn read(request: &Path) -> Result<Vec<ResponseExample>, StorageError> {
    let path = path_for(request);
    if !path.is_file() {
        return Ok(Vec::new());
    }
    let parsed = document::read(&path, EXAMPLES_FORMAT)?;
    examples_from(&parsed.doc, &path)
}

/// Writes the examples of the request file `request`; no examples remove
/// the file. A damaged file is moved aside first.
///
/// # Errors
/// If a file cannot be written, moved or removed.
pub fn write(request: &Path, examples: &[ResponseExample]) -> Result<(), StorageError> {
    let path = path_for(request);
    if path.is_file() && read(request).is_err() {
        let backup = path.with_extension(format!("{EXTENSION}.{}.bak", now_ms()));
        std::fs::rename(&path, &backup).map_err(|error| StorageError::io(&path, error))?;
    }
    if examples.is_empty() {
        if path.is_file() {
            std::fs::remove_file(&path).map_err(|error| StorageError::io(&path, error))?;
        }
        return Ok(());
    }
    document::write_atomically(&path, document_of(examples).to_string())
}

fn examples_from(doc: &DocumentMut, path: &Path) -> Result<Vec<ResponseExample>, StorageError> {
    let Some(item) = doc.get(EXAMPLE) else {
        return Ok(Vec::new());
    };
    let tables = item
        .as_array_of_tables()
        .ok_or_else(|| StorageError::invalid(path, EXAMPLE, "must be a list of tables"))?;
    tables
        .iter()
        .map(|table| {
            let status = table
                .get(STATUS)
                .and_then(Item::as_integer)
                .and_then(|code| u16::try_from(code).ok())
                .and_then(|code| StatusCode::new(code).ok())
                .ok_or_else(|| StorageError::invalid(path, STATUS, "must be a status code"))?;
            let headers = match table.get(HEADERS) {
                None => Vec::new(),
                Some(item) => item
                    .as_array_of_tables()
                    .ok_or_else(|| {
                        StorageError::invalid(path, HEADERS, "must be a list of tables")
                    })?
                    .iter()
                    .map(|header| {
                        Ok(Header::new(
                            document::opt_str(header, NAME, path)?.unwrap_or_default(),
                            document::opt_str(header, VALUE, path)?.unwrap_or_default(),
                        ))
                    })
                    .collect::<Result<_, StorageError>>()?,
            };
            Ok(ResponseExample {
                name: document::opt_str(table, NAME, path)?.unwrap_or_default(),
                status,
                headers,
                body: document::opt_str(table, BODY, path)?.unwrap_or_default(),
            })
        })
        .collect()
}

fn document_of(examples: &[ResponseExample]) -> DocumentMut {
    let mut doc = DocumentMut::new();
    doc.insert("format", value(EXAMPLES_FORMAT));
    doc.insert("version", value(CURRENT_VERSION));
    let mut list = ArrayOfTables::new();
    for example in examples {
        let mut table = Table::new();
        table.insert(NAME, value(example.name.as_str()));
        table.insert(STATUS, value(i64::from(example.status.as_u16())));
        table.insert(BODY, value(example.body.as_str()));
        let mut headers = ArrayOfTables::new();
        for header in &example.headers {
            let mut row = Table::new();
            row.insert(NAME, value(header.name.as_str()));
            row.insert(VALUE, value(header.value.as_str()));
            headers.push(row);
        }
        if !headers.is_empty() {
            table.insert(HEADERS, Item::ArrayOfTables(headers));
        }
        list.push(table);
    }
    doc.insert(EXAMPLE, Item::ArrayOfTables(list));
    doc
}

fn now_ms() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |since| since.as_millis())
}

#[cfg(test)]
#[path = "tests/examples_file.rs"]
mod tests;

//! One request file: reading it into a [`Request`] and writing a request
//! back, changing only what differs.

use std::path::Path;

use ac_domain::collection::{Body, HeaderEntry, Request};
use ac_domain::protocol::HttpMethod;
use toml_edit::{DocumentMut, Item, Table, TableLike};

use crate::document;
use crate::error::StorageError;
use crate::format::{CURRENT_VERSION, REQUEST_FORMAT, body_type};
use crate::rows::{self, Row};
use crate::{auth, variables};

const REQUEST_TABLE: &str = "request";
const HEADERS: &str = "headers";
const BODY: &str = "body";

/// Reads a request file. Missing optional values get defaults (name: file
/// name, method: `GET`, empty URL, no headers, no body).
///
/// # Errors
/// If the file cannot be read or is not a valid request file.
pub fn read_request(path: &Path) -> Result<(Request, bool), StorageError> {
    let parsed = document::read(path, REQUEST_FORMAT)?;
    let request = request_from(&parsed.doc, path)?;
    Ok((request, parsed.read_only()))
}

pub(crate) fn request_from(doc: &DocumentMut, path: &Path) -> Result<Request, StorageError> {
    let root = doc.as_table();
    let name = match document::opt_str(root, "name", path)? {
        Some(name) => name,
        None => crate::names::file_stem(path),
    };
    let empty = Table::new();
    let table: &dyn TableLike = match root.get(REQUEST_TABLE) {
        None => &empty,
        Some(item) => item
            .as_table_like()
            .ok_or_else(|| StorageError::invalid(path, "request", "must be a table"))?,
    };
    let method = match document::opt_str(table, "method", path)? {
        None => HttpMethod::Get,
        Some(text) => text
            .parse()
            .map_err(|error: ac_domain::protocol::InvalidMethod| {
                StorageError::invalid(path, "request.method", error.to_string())
            })?,
    };
    Ok(Request {
        variables: variables::read(table.get(variables::KEY), path, "request.variables")?,
        auth: auth::read(table.get(auth::KEY), path, "request.auth")?,
        id: document::id(root, path, path)?,
        name,
        method,
        url: document::opt_str(table, "url", path)?.unwrap_or_default(),
        headers: headers_from(table.get(HEADERS), path)?,
        body: body_from(table.get(BODY), path)?,
    })
}

fn headers_from(item: Option<&Item>, path: &Path) -> Result<Vec<HeaderEntry>, StorageError> {
    Ok(rows::read(item, path, "request.headers")?
        .into_iter()
        .map(|row| HeaderEntry {
            name: row.name,
            value: row.value,
            enabled: row.enabled,
        })
        .collect())
}

fn header_rows(headers: &[HeaderEntry]) -> Vec<Row> {
    headers
        .iter()
        .map(|header| Row {
            name: header.name.clone(),
            value: header.value.clone(),
            enabled: header.enabled,
            secret: false,
        })
        .collect()
}

fn body_from(item: Option<&Item>, path: &Path) -> Result<Body, StorageError> {
    let Some(item) = item else {
        return Ok(Body::None);
    };
    let table = item
        .as_table_like()
        .ok_or_else(|| StorageError::invalid(path, "request.body", "must be a table"))?;
    let content = document::opt_str(table, "content", path)?.unwrap_or_default();
    match document::opt_str(table, "type", path)?.as_deref() {
        Some(body_type::JSON) => Ok(Body::Json(content)),
        Some(body_type::TEXT) => Ok(Body::Text(content)),
        other => Err(StorageError::invalid(
            path,
            "request.body.type",
            format!("must be \"json\" or \"text\", found {other:?}"),
        )),
    }
}

/// Writes `request` to `path`. An existing file keeps its comments, key
/// order and unknown keys; only changed values are rewritten. The write is
/// atomic: a crash leaves either the old or the new file, never half of it.
///
/// # Errors
/// If the existing file is damaged or from a newer version (it is then left
/// untouched), or the file cannot be written.
pub fn save_request(path: &Path, request: &Request) -> Result<(), StorageError> {
    let mut doc = match std::fs::read_to_string(path) {
        Ok(text) => {
            let parsed = document::parse(path, &text, REQUEST_FORMAT)?;
            if parsed.read_only() {
                return Err(StorageError::ReadOnly {
                    path: path.to_owned(),
                    version: parsed.version,
                });
            }
            parsed.doc
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => new_document(),
        Err(error) => return Err(StorageError::io(path, error)),
    };
    apply(&mut doc, request, path)?;
    document::write_atomically(path, &doc.to_string())
}

fn new_document() -> DocumentMut {
    let mut doc = DocumentMut::new();
    doc.insert("format", toml_edit::value(REQUEST_FORMAT));
    doc.insert("version", toml_edit::value(CURRENT_VERSION));
    doc
}

/// Updates `doc` to describe `request`, touching only what differs.
fn apply(doc: &mut DocumentMut, request: &Request, path: &Path) -> Result<(), StorageError> {
    let before = request_from(doc, path)?;
    let root = doc.as_table_mut();
    document::set(root, "version", CURRENT_VERSION);
    document::set(root, "id", request.id.to_string());
    document::set(root, "name", request.name.as_str());

    if !root.contains_key(REQUEST_TABLE) {
        root.insert(REQUEST_TABLE, Item::Table(Table::new()));
    }
    let table = root
        .get_mut(REQUEST_TABLE)
        .and_then(Item::as_table_like_mut)
        .ok_or_else(|| StorageError::invalid(path, "request", "must be a table"))?;
    document::set(table, "method", request.method.as_str());
    document::set(table, "url", request.url.as_str());

    rows::write(
        table,
        HEADERS,
        &header_rows(&before.headers),
        &header_rows(&request.headers),
    );
    variables::write(table, &before.variables, &request.variables);
    auth::write(table, &before.auth, &request.auth);
    if before.body != request.body {
        table.remove(BODY);
        if let Some(item) = body_item(&request.body) {
            table.insert(BODY, item);
        }
    }
    Ok(())
}

fn body_item(body: &Body) -> Option<Item> {
    let (kind, content) = match body {
        Body::None => return None,
        Body::Json(content) => (body_type::JSON, content),
        Body::Text(content) => (body_type::TEXT, content),
    };
    let mut table = Table::new();
    table.insert("type", toml_edit::value(kind));
    table.insert("content", Item::Value(document::text_value(content)));
    Some(Item::Table(table))
}

#[cfg(test)]
#[path = "tests/request_file.rs"]
mod tests;

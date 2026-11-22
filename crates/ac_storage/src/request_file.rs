//! One request file: reading it into a [`Request`] and writing a request
//! back, changing only what differs.

use std::path::Path;

use ac_domain::collection::{HeaderEntry, Request};
use ac_domain::params::{PathParam, QueryParam};
use ac_domain::protocol::HttpMethod;
use toml_edit::{DocumentMut, Item, Table, TableLike};

use crate::document;
use crate::error::StorageError;
use crate::format::{CURRENT_VERSION, REQUEST_FORMAT};
use crate::rows::{self, Row};
use crate::{auth, body, scripts, tls, variables};

const REQUEST_TABLE: &str = "request";
const HEADERS: &str = "headers";
const BODY: &str = "body";
/// Values of the path variables (`:name`) in the URL.
const PATH_PARAMS: &str = "path_params";
/// Query parameters switched off (the others are part of the URL).
const DISABLED_QUERY: &str = "disabled_query";

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
        certificate_check: tls::read(table, path)?,
        scripts: scripts::read(table.get(scripts::KEY), path, "request.scripts")?,
        id: document::id(root, path, path)?,
        name,
        method,
        url: document::opt_str(table, "url", path)?.unwrap_or_default(),
        headers: headers_from(table.get(HEADERS), path)?,
        body: body::read(table.get(BODY), path)?,
        path_params: rows::read(table.get(PATH_PARAMS), path, "request.path_params")?
            .into_iter()
            .map(|row| PathParam {
                name: row.name,
                value: row.value,
            })
            .collect(),
        disabled_query: rows::read(table.get(DISABLED_QUERY), path, "request.disabled_query")?
            .into_iter()
            .map(|row| QueryParam {
                name: row.name,
                value: row.value,
            })
            .collect(),
    })
}

/// Rows of name/value pairs; `enabled` says whether they are sent.
fn pair_rows<'a>(pairs: impl Iterator<Item = (&'a String, &'a String)>, enabled: bool) -> Vec<Row> {
    pairs
        .map(|(name, value)| Row {
            name: name.clone(),
            value: value.clone(),
            enabled,
            secret: false,
        })
        .collect()
}

fn path_rows(params: &[PathParam]) -> Vec<Row> {
    pair_rows(params.iter().map(|param| (&param.name, &param.value)), true)
}

fn disabled_rows(params: &[QueryParam]) -> Vec<Row> {
    pair_rows(
        params.iter().map(|param| (&param.name, &param.value)),
        false,
    )
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
    rows::write(
        table,
        PATH_PARAMS,
        &path_rows(&before.path_params),
        &path_rows(&request.path_params),
    );
    rows::write(
        table,
        DISABLED_QUERY,
        &disabled_rows(&before.disabled_query),
        &disabled_rows(&request.disabled_query),
    );
    variables::write(table, &before.variables, &request.variables);
    auth::write(table, &before.auth, &request.auth);
    tls::write(table, before.certificate_check, request.certificate_check);
    scripts::write(table, &before.scripts, &request.scripts);
    if before.body != request.body {
        table.remove(BODY);
        if let Some(item) = body::item(&request.body) {
            table.insert(BODY, item);
        }
    }
    Ok(())
}

#[cfg(test)]
#[path = "tests/request_file.rs"]
mod tests;

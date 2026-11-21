//! The body table of a request file:
//!
//! ```toml
//! [request.body]
//! type = "json"                 # json | text: `content` is the text
//! content = '{"name": "Teapot"}'
//!
//! [request.body]
//! type = "form"                 # form | multipart: a list of fields
//! [[request.body.fields]]
//! name = "q"
//! value = "tea"
//! enabled = false               # optional, default true
//! [[request.body.fields]]      # multipart only: a file instead of a value
//! name = "avatar"
//! file = "files/avatar.png"
//!
//! [request.body]
//! type = "binary"               # the contents of `file`
//! file = "data/payload.bin"
//! ```
//!
//! File paths are relative to the collection folder unless absolute.

use std::path::Path;

use ac_domain::collection::{Body, FormField, MultipartField};
use toml_edit::{Item, Table, TableLike, Value};

use crate::document;
use crate::error::StorageError;
use crate::format::body_type;

const FIELD: &str = "request.body";
const FIELDS: &str = "fields";
const FILE: &str = "file";

/// Reads the body table (missing = no body).
pub(crate) fn read(item: Option<&Item>, path: &Path) -> Result<Body, StorageError> {
    let Some(item) = item else {
        return Ok(Body::None);
    };
    let table = item
        .as_table_like()
        .ok_or_else(|| StorageError::invalid(path, FIELD, "must be a table"))?;
    let text = |key| -> Result<String, StorageError> {
        Ok(document::opt_str(table, key, path)?.unwrap_or_default())
    };
    match document::opt_str(table, "type", path)?.as_deref() {
        Some(body_type::JSON) => Ok(Body::Json(text("content")?)),
        Some(body_type::TEXT) => Ok(Body::Text(text("content")?)),
        Some(body_type::BINARY) => Ok(Body::Binary(text(FILE)?)),
        Some(body_type::FORM) => Ok(Body::Form(
            fields(table, path)?
                .into_iter()
                .map(|field| {
                    Ok(FormField {
                        name: name(field, path)?,
                        value: document::opt_str(field, "value", path)?.unwrap_or_default(),
                        enabled: enabled(field, path)?,
                    })
                })
                .collect::<Result<_, StorageError>>()?,
        )),
        Some(body_type::MULTIPART) => Ok(Body::Multipart(
            fields(table, path)?
                .into_iter()
                .map(|field| multipart_field(field, path))
                .collect::<Result<_, StorageError>>()?,
        )),
        other => Err(StorageError::invalid(
            path,
            "request.body.type",
            format!("must be json, text, form, multipart or binary, found {other:?}"),
        )),
    }
}

fn fields<'a>(
    table: &'a dyn TableLike,
    path: &Path,
) -> Result<Vec<&'a dyn TableLike>, StorageError> {
    let invalid =
        || StorageError::invalid(path, "request.body.fields", "must be an array of tables");
    match table.get(FIELDS) {
        None => Ok(Vec::new()),
        Some(Item::ArrayOfTables(tables)) => {
            Ok(tables.iter().map(|table| table as &dyn TableLike).collect())
        }
        Some(Item::Value(Value::Array(values))) => values
            .iter()
            .map(|value| {
                value
                    .as_inline_table()
                    .map(|table| table as &dyn TableLike)
                    .ok_or_else(invalid)
            })
            .collect(),
        Some(_) => Err(invalid()),
    }
}

fn multipart_field(field: &dyn TableLike, path: &Path) -> Result<MultipartField, StorageError> {
    let file = document::opt_str(field, FILE, path)?;
    Ok(MultipartField {
        name: name(field, path)?,
        file: file.is_some(),
        value: match file {
            Some(file) => file,
            None => document::opt_str(field, "value", path)?.unwrap_or_default(),
        },
        enabled: enabled(field, path)?,
    })
}

fn name(field: &dyn TableLike, path: &Path) -> Result<String, StorageError> {
    document::opt_str(field, "name", path)?
        .ok_or_else(|| StorageError::invalid(path, "name", "every field needs a name"))
}

fn enabled(field: &dyn TableLike, path: &Path) -> Result<bool, StorageError> {
    Ok(document::opt_bool(field, "enabled", path)?.unwrap_or(true))
}

/// The body table for `body`; `None` for no body.
pub(crate) fn item(body: &Body) -> Option<Item> {
    let kind = match body {
        Body::None => return None,
        Body::Json(_) => body_type::JSON,
        Body::Text(_) => body_type::TEXT,
        Body::Binary(_) => body_type::BINARY,
        Body::Form(_) => body_type::FORM,
        Body::Multipart(_) => body_type::MULTIPART,
    };
    let mut table = Table::new();
    table.insert("type", toml_edit::value(kind));
    match body {
        Body::None => {}
        Body::Json(content) | Body::Text(content) => {
            table.insert("content", Item::Value(document::text_value(content)));
        }
        Body::Binary(file) => {
            table.insert(FILE, toml_edit::value(file.as_str()));
        }
        Body::Form(fields) => {
            let rows = fields
                .iter()
                .map(|field| row(&field.name, "value", &field.value, field.enabled));
            table.insert(FIELDS, Item::ArrayOfTables(rows.collect()));
        }
        Body::Multipart(fields) => {
            let rows = fields.iter().map(|field| {
                let key = if field.file { FILE } else { "value" };
                row(&field.name, key, &field.value, field.enabled)
            });
            table.insert(FIELDS, Item::ArrayOfTables(rows.collect()));
        }
    }
    Some(Item::Table(table))
}

fn row(name: &str, key: &str, value: &str, enabled: bool) -> Table {
    let mut row = Table::new();
    row.insert("name", toml_edit::value(name));
    row.insert(key, toml_edit::value(value));
    if !enabled {
        row.insert("enabled", toml_edit::value(false));
    }
    row
}

#[cfg(test)]
#[path = "tests/body.rs"]
mod tests;

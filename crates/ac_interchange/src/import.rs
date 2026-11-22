//! Importing a collection from another tool's file: the format is detected
//! from the content, so the user picks a file and nothing else.

use ac_domain::collection::Collection;
use ac_domain::environment::Environment;
use serde_json::Value;
use url::Url;

use crate::{openapi, postman};

/// A converted collection.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Imported {
    /// The collection.
    pub collection: Collection,
    /// Environments that came with it (Bruno).
    pub environments: Vec<Environment>,
    /// What could not be taken over, in the order it was met.
    pub skipped: Vec<Skipped>,
}

/// Something that was not taken over.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Skipped {
    /// Name of the collection, folder or request.
    pub item: String,
    /// What.
    pub what: SkippedKind,
}

/// Kinds of [`Skipped`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SkippedKind {
    /// A declarative assertion without counterpart (Bruno `assert`), as
    /// written, e.g. `res.status: between 200 299`.
    Assertion(String),
    /// An auth type without counterpart, e.g. `oauth2`.
    Auth(String),
    /// A body type without counterpart, e.g. `graphql` or `application/xml`.
    Body(String),
    /// Headers of a folder or the collection (only requests have headers).
    Headers,
    /// An `OpenAPI` server given relative to where the description is
    /// served, e.g. `/api/v3`: the host is unknown, so `baseUrl` got
    /// `http://localhost` in front.
    RelativeServer(String),
}

/// Why a file cannot be imported.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ImportError {
    /// Neither JSON nor YAML.
    #[error("neither JSON nor YAML: {0}")]
    Parse(String),
    /// A format this program does not read.
    #[error("not a Postman collection or OpenAPI 3 description")]
    Unknown,
    /// A folder without `bruno.json`: no Bruno collection.
    #[error("no Bruno collection here (bruno.json is missing)")]
    NotBruno,
    /// Swagger 2.0, the predecessor of `OpenAPI` 3.
    #[error("Swagger 2.0 is not supported; convert it to OpenAPI 3 first")]
    Swagger2,
}

/// Converts a Postman collection or an `OpenAPI` 3 description (JSON or
/// YAML), whichever `text` is.
///
/// # Errors
/// [`ImportError`] if the text is none of them.
pub fn import_collection(text: &str) -> Result<Imported, ImportError> {
    import_collection_from(text, None)
}

/// Like [`import_collection`] for a description downloaded from `origin`:
/// relative `OpenAPI` servers (`/api/v3`) are resolved against it, as a
/// browser would.
///
/// # Errors
/// [`ImportError`] if the text is none of the formats.
pub fn import_collection_from(text: &str, origin: Option<&Url>) -> Result<Imported, ImportError> {
    let document = parse(text)?;
    if document.get("swagger").is_some() {
        return Err(ImportError::Swagger2);
    }
    if let Some(imported) = openapi::from_value(&document, origin) {
        return Ok(imported);
    }
    postman::from_value(&document).ok_or(ImportError::Unknown)
}

/// An imported OAuth 2 redirect URL if the app can receive it; empty (any
/// free port) otherwise, e.g. for the web callbacks of other tools.
pub(crate) fn loopback_redirect(url: &str) -> String {
    match url::Url::parse(url.trim()) {
        Ok(parsed) if ac_domain::auth::is_loopback_redirect(&parsed) => url.trim().to_owned(),
        _ => String::new(),
    }
}

/// The document in `text`: JSON, else YAML (which also covers most JSON).
pub(crate) fn parse(text: &str) -> Result<Value, ImportError> {
    if let Ok(value) = serde_json::from_str(text) {
        return Ok(value);
    }
    let documents = yaml_rust2::YamlLoader::load_from_str(text)
        .map_err(|error| ImportError::Parse(error.to_string()))?;
    documents
        .into_iter()
        .next()
        .map(yaml_to_json)
        .ok_or_else(|| ImportError::Parse("empty document".into()))
}

fn yaml_to_json(yaml: yaml_rust2::Yaml) -> Value {
    use yaml_rust2::Yaml;
    match yaml {
        Yaml::Real(text) => text
            .parse::<f64>()
            .ok()
            .and_then(serde_json::Number::from_f64)
            .map_or(Value::String(text), Value::Number),
        Yaml::Integer(number) => Value::Number(number.into()),
        Yaml::String(text) => Value::String(text),
        Yaml::Boolean(flag) => Value::Bool(flag),
        Yaml::Array(items) => Value::Array(items.into_iter().map(yaml_to_json).collect()),
        Yaml::Hash(entries) => Value::Object(
            entries
                .into_iter()
                .map(|(key, value)| (yaml_key(key), yaml_to_json(value)))
                .collect(),
        ),
        Yaml::Alias(_) | Yaml::Null | Yaml::BadValue => Value::Null,
    }
}

/// Map keys as text (`200:` in YAML is a number, but a status in `OpenAPI`).
fn yaml_key(key: yaml_rust2::Yaml) -> String {
    match yaml_to_json(key) {
        Value::String(text) => text,
        other => other.to_string(),
    }
}

#[cfg(test)]
#[path = "tests/import.rs"]
mod tests;

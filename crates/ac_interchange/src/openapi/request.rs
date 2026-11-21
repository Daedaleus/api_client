//! One operation of a description as a request: name, URL with variables,
//! headers, body and auth.

use ac_domain::auth::{Auth, KeyPlacement, OAuth2};
use ac_domain::collection::{Body, FormField, HeaderEntry, MultipartField, Request};
use ac_domain::environment::Variable;
use ac_domain::protocol::HttpMethod;
use serde_json::Value;

use super::schema::{Document, text};
use super::{BASE_URL, skipped};
use crate::import::{Skipped, SkippedKind};

/// What an operation is made of.
pub(super) struct Operation<'a> {
    pub method: &'a str,
    pub path: &'a str,
    /// The path item, for parameters shared by its operations.
    pub path_item: &'a Value,
    /// The operation object itself.
    pub spec: &'a Value,
    /// The collection's auth (an operation with the same one inherits it).
    pub global_auth: &'a Auth,
}

/// The request for an operation.
pub(super) fn operation(
    document: &Document<'_>,
    operation: &Operation<'_>,
    skipped: &mut Vec<Skipped>,
) -> Request {
    let op = operation.spec;
    let name = op
        .get("summary")
        .or_else(|| op.get("operationId"))
        .and_then(Value::as_str)
        .map_or_else(
            || format!("{} {}", operation.method.to_uppercase(), operation.path),
            str::to_owned,
        );
    let mut request = Request::new(name);
    request.method = operation
        .method
        .to_uppercase()
        .parse()
        .unwrap_or(HttpMethod::Get);

    let mut url = format!("{{{{{BASE_URL}}}}}{}", path_with_variables(operation.path));
    let mut query = Vec::new();
    for parameter in parameters(document, operation) {
        let Some(name) = parameter.get("name").and_then(Value::as_str) else {
            continue;
        };
        let required = parameter.get("required").and_then(Value::as_bool) == Some(true);
        let value = parameter_example(document, parameter);
        match parameter.get("in").and_then(Value::as_str) {
            Some("path") => request.variables.push(Variable::plain(name, value)),
            Some("query") if required => {
                query.push(format!("{name}={{{{{name}}}}}"));
                request.variables.push(Variable::plain(name, value));
            }
            Some("header") => request.headers.push(HeaderEntry {
                name: name.to_owned(),
                value,
                enabled: required,
            }),
            _ => {}
        }
    }
    if !query.is_empty() {
        url.push('?');
        url.push_str(&query.join("&"));
    }
    request.url = url;
    if let Some(body) = op.get("requestBody") {
        request.body = body_of(document, document.resolve(body), &request.name, skipped);
    }
    if let Some(auth) = security(document, op.get("security"), &request.name, skipped)
        && auth != *operation.global_auth
    {
        request.auth = auth;
    }
    request
}

/// `/pets/{id}` as `/pets/{{id}}`.
fn path_with_variables(path: &str) -> String {
    path.replace('{', "{{").replace('}', "}}")
}

/// The parameters of the path item and the operation (the operation's win).
fn parameters<'v>(document: &'v Document<'_>, operation: &'v Operation<'_>) -> Vec<&'v Value> {
    let mut all: Vec<&Value> = Vec::new();
    for source in [operation.path_item, operation.spec] {
        for parameter in source
            .get("parameters")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
        {
            let parameter = document.resolve(parameter);
            let key = |p: &Value| (p.get("name").cloned(), p.get("in").cloned());
            all.retain(|known| key(known) != key(parameter));
            all.push(parameter);
        }
    }
    all
}

/// The example of a parameter, if the description gives one.
fn parameter_example(document: &Document<'_>, parameter: &Value) -> String {
    if let Some(example) = parameter.get("example") {
        return text(example);
    }
    let Some(schema) = parameter
        .get("schema")
        .map(|schema| document.resolve(schema))
    else {
        return String::new();
    };
    let given = schema
        .get("example")
        .or_else(|| schema.get("default"))
        .or_else(|| schema.get("enum").and_then(|values| values.get(0)));
    given.map_or_default(text)
}

/// The body for a request body, by its first usable media type.
fn body_of(document: &Document<'_>, body: &Value, item: &str, skipped: &mut Vec<Skipped>) -> Body {
    let Some(content) = body.get("content").and_then(Value::as_object) else {
        return Body::None;
    };
    let Some((media_type, media)) = content.iter().next() else {
        return Body::None;
    };
    let schema = media.get("schema").map(|schema| document.resolve(schema));
    let example = || -> Value {
        if let Some(example) = media.get("example") {
            return example.clone();
        }
        if let Some(first) = media
            .get("examples")
            .and_then(Value::as_object)
            .and_then(|examples| examples.values().next())
        {
            return document
                .resolve(first)
                .get("value")
                .cloned()
                .unwrap_or(Value::Null);
        }
        schema.map_or(Value::Null, |schema| document.example(schema))
    };
    let kind = media_type.to_ascii_lowercase();
    if kind.contains("json") {
        let value = example();
        let text = serde_json::to_string_pretty(&value).unwrap_or_default();
        return Body::Json(if value.is_null() { String::new() } else { text });
    }
    if kind == "application/x-www-form-urlencoded" {
        return Body::Form(
            fields(document, schema)
                .into_iter()
                .map(|(name, value, _)| FormField {
                    name,
                    value,
                    enabled: true,
                })
                .collect(),
        );
    }
    if kind == "multipart/form-data" {
        return Body::Multipart(
            fields(document, schema)
                .into_iter()
                .map(|(name, value, file)| MultipartField {
                    name,
                    value: if file { String::new() } else { value },
                    enabled: true,
                    file,
                })
                .collect(),
        );
    }
    if kind == "application/octet-stream" {
        return Body::Binary(String::new());
    }
    if kind.starts_with("text/") || kind.contains("xml") {
        return Body::Text(text(&example()));
    }
    skipped.push(super::skipped(item, SkippedKind::Body(media_type.clone())));
    Body::None
}

/// The fields of a form schema: name, example value, whether it is a file.
fn fields(document: &Document<'_>, schema: Option<&Value>) -> Vec<(String, String, bool)> {
    let Some(schema) = schema else {
        return Vec::new();
    };
    document
        .properties_of(schema)
        .into_iter()
        .map(|(name, property)| {
            let property = document.resolve(property);
            let file = matches!(
                property.get("format").and_then(Value::as_str),
                Some("binary" | "base64")
            );
            (name.clone(), text(&document.example(property)), file)
        })
        .collect()
}

/// The auth of a `security` list: `Some(None)` for an empty list (no auth),
/// `None` if there is no list. Only the first scheme of the first
/// requirement is used.
pub(super) fn security(
    document: &Document<'_>,
    security: Option<&Value>,
    item: &str,
    skipped: &mut Vec<Skipped>,
) -> Option<Auth> {
    let requirements = security?.as_array()?;
    let Some((name, scopes)) = requirements
        .first()
        .and_then(Value::as_object)
        .and_then(|requirement| requirement.iter().next())
    else {
        return Some(Auth::None);
    };
    let scheme = document
        .scheme(name)
        .map(|scheme| document.resolve(scheme))?;
    let kind = scheme
        .get("type")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let http = scheme
        .get("scheme")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_ascii_lowercase();
    match (kind, http.as_str()) {
        ("http", "basic") => Some(Auth::Basic {
            username: "{{username}}".into(),
            password: "{{password}}".into(),
        }),
        ("http", "bearer") => Some(Auth::Bearer {
            token: "{{token}}".into(),
        }),
        ("apiKey", _) => {
            let placement = match scheme.get("in").and_then(Value::as_str) {
                Some("query") => KeyPlacement::Query,
                Some("header") => KeyPlacement::Header,
                other => {
                    skipped.push(super::skipped(
                        item,
                        SkippedKind::Auth(format!("apiKey in {}", other.unwrap_or("?"))),
                    ));
                    return None;
                }
            };
            Some(Auth::ApiKey {
                name: scheme
                    .get("name")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_owned(),
                value: "{{apiKey}}".into(),
                placement,
            })
        }
        ("oauth2", _) => {
            let token_url = scheme
                .get("flows")
                .and_then(|flows| flows.get("clientCredentials"))
                .and_then(|flow| flow.get("tokenUrl"))
                .and_then(Value::as_str);
            let Some(token_url) = token_url else {
                skipped.push(skipped_auth(item, "oauth2".into()));
                return None;
            };
            let scope: Vec<&str> = scopes
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(Value::as_str)
                .collect();
            Some(Auth::OAuth2(OAuth2 {
                token_url: token_url.to_owned(),
                client_id: "{{clientId}}".into(),
                client_secret: "{{clientSecret}}".into(),
                scope: scope.join(" "),
            }))
        }
        (other, scheme_name) => {
            let label = if scheme_name.is_empty() {
                other.to_owned()
            } else {
                format!("{other} {scheme_name}")
            };
            skipped.push(skipped_auth(item, label));
            None
        }
    }
}

fn skipped_auth(item: &str, label: String) -> Skipped {
    skipped(item, SkippedKind::Auth(label))
}

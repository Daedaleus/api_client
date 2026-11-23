//! A collection as a Postman collection (format v2.1).
//!
//! Folders and requests in order, methods, URLs with path variables and
//! switched-off query parameters, headers, bodies, auth, variables,
//! scripts (as written – Postman runs them with its `pm` object) and the
//! redirect, certificate and cookie settings of folders and requests
//! (`protocolProfileBehavior`). Secret values are never written: secret
//! variables are exported without a value. What Postman has no place for –
//! time limits, the collection's own redirect, certificate and cookie
//! settings – is reported.

use ac_domain::auth::{Auth, KeyPlacement, OAuth2Grant};
use ac_domain::collection::{Body, Collection, Folder, Item, Request};
use ac_domain::environment::Variable;
use ac_domain::protocol::grant_type::{AUTHORIZATION_CODE, CLIENT_CREDENTIALS};
use ac_domain::script::Scripts;
use ac_domain::tls::CertificateCheck;
use ac_domain::transport::{Cookies, Redirects, TransportSettings};
use serde_json::{Map, Value, json};

use crate::import::{ItemSetting, Skipped, SkippedKind};

/// The schema Postman recognises a v2.1 collection by.
const SCHEMA: &str = "https://schema.getpostman.com/json/collection/v2.1.0/collection.json";
/// Script events.
const PREREQUEST_EVENT: &str = "prerequest";
const TEST_EVENT: &str = "test";

/// A collection in another tool's format.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Exported {
    /// The file's text.
    pub text: String,
    /// What was not written, in the order it was met.
    pub left_out: Vec<Skipped>,
}

/// `collection` as a Postman collection v2.1 (pretty-printed JSON).
#[must_use]
pub fn to_postman(collection: &Collection) -> Exported {
    let mut left_out = Vec::new();
    let mut root = Map::new();
    let mut info = json!({ "name": collection.name, "schema": SCHEMA });
    if !collection.docs.trim().is_empty() {
        info["description"] = json!(collection.docs);
    }
    root.insert("info".into(), info);
    let items = items(&collection.items, &mut left_out);
    root.insert("item".into(), Value::Array(items));
    insert_auth(&mut root, &collection.auth);
    insert_events(&mut root, &collection.scripts);
    insert_variables(
        &mut root,
        &collection.name,
        &collection.variables,
        &mut left_out,
    );
    // Postman knows these settings only on folders and requests.
    report_settings(
        &collection.name,
        &collection.transport,
        collection.certificate_check,
        true,
        &mut left_out,
    );
    let mut text = serde_json::to_string_pretty(&Value::Object(root)).unwrap_or_default();
    text.push('\n');
    Exported { text, left_out }
}

fn items(items: &[Item], left_out: &mut Vec<Skipped>) -> Vec<Value> {
    items
        .iter()
        .map(|item| match item {
            Item::Folder(folder) => folder_of(folder, left_out),
            Item::Request(request) => request_of(request, left_out),
        })
        .collect()
}

fn folder_of(folder: &Folder, left_out: &mut Vec<Skipped>) -> Value {
    let mut object = Map::new();
    object.insert("name".into(), json!(folder.name));
    if !folder.docs.trim().is_empty() {
        object.insert("description".into(), json!(folder.docs));
    }
    object.insert("item".into(), Value::Array(items(&folder.items, left_out)));
    insert_auth(&mut object, &folder.auth);
    insert_events(&mut object, &folder.scripts);
    insert_variables(&mut object, &folder.name, &folder.variables, left_out);
    insert_behavior(&mut object, &folder.transport, folder.certificate_check);
    report_settings(
        &folder.name,
        &folder.transport,
        folder.certificate_check,
        false,
        left_out,
    );
    Value::Object(object)
}

fn request_of(request: &Request, left_out: &mut Vec<Skipped>) -> Value {
    let mut fields = Map::new();
    fields.insert("method".into(), json!(request.method.as_str()));
    fields.insert(
        "header".into(),
        Value::Array(
            request
                .headers
                .iter()
                .map(|header| row(&header.name, &header.value, header.enabled))
                .collect(),
        ),
    );
    if let Some(body) = body_of(&request.body) {
        fields.insert("body".into(), body);
    }
    fields.insert("url".into(), url_of(request));
    if !request.docs.trim().is_empty() {
        fields.insert("description".into(), json!(request.docs));
    }
    insert_auth(&mut fields, &request.auth);

    let mut object = Map::new();
    object.insert("name".into(), json!(request.name));
    insert_events(&mut object, &request.scripts);
    insert_variables(&mut object, &request.name, &request.variables, left_out);
    insert_behavior(&mut object, &request.transport, request.certificate_check);
    report_settings(
        &request.name,
        &request.transport,
        request.certificate_check,
        false,
        left_out,
    );
    object.insert("request".into(), Value::Object(fields));
    if !request.examples.is_empty() {
        let responses = request
            .examples
            .iter()
            .map(|example| {
                json!({
                    "name": example.name,
                    "code": example.status.as_u16(),
                    "header": example
                        .headers
                        .iter()
                        .map(|header| json!({ "key": header.name, "value": header.value }))
                        .collect::<Vec<_>>(),
                    "body": example.body,
                })
            })
            .collect();
        object.insert("response".into(), Value::Array(responses));
    }
    Value::Object(object)
}

/// The URL as written, with its query parameters (also the switched-off
/// ones) and path variables listed as Postman shows them.
fn url_of(request: &Request) -> Value {
    let mut url = Map::new();
    url.insert("raw".into(), json!(request.url));
    let mut query: Vec<Value> = request
        .url
        .split_once('?')
        .map(|(_, query)| query.split_once('#').map_or(query, |(query, _)| query))
        .filter(|query| !query.is_empty())
        .map_or_default(|query| {
            query
                .split('&')
                .map(|pair| {
                    let (name, value) = pair.split_once('=').unwrap_or((pair, ""));
                    row(name, value, true)
                })
                .collect()
        });
    query.extend(
        request
            .disabled_query
            .iter()
            .map(|param| row(&param.name, &param.value, false)),
    );
    if !query.is_empty() {
        url.insert("query".into(), Value::Array(query));
    }
    if !request.path_params.is_empty() {
        url.insert(
            "variable".into(),
            Value::Array(
                request
                    .path_params
                    .iter()
                    .map(|param| json!({ "key": param.name, "value": param.value }))
                    .collect(),
            ),
        );
    }
    Value::Object(url)
}

fn body_of(body: &Body) -> Option<Value> {
    Some(match body {
        Body::None => return None,
        Body::Json(text) => json!({
            "mode": "raw",
            "raw": text,
            "options": { "raw": { "language": "json" } },
        }),
        Body::Text(text) => json!({
            "mode": "raw",
            "raw": text,
            "options": { "raw": { "language": "text" } },
        }),
        Body::Form(fields) => json!({
            "mode": "urlencoded",
            "urlencoded": fields
                .iter()
                .map(|field| row(&field.name, &field.value, field.enabled))
                .collect::<Vec<_>>(),
        }),
        Body::Multipart(fields) => json!({
            "mode": "formdata",
            "formdata": fields
                .iter()
                .map(|field| {
                    let mut part = Map::new();
                    part.insert("key".into(), json!(field.name));
                    if field.file {
                        part.insert("type".into(), json!("file"));
                        part.insert("src".into(), json!(field.value));
                    } else {
                        part.insert("type".into(), json!("text"));
                        part.insert("value".into(), json!(field.value));
                    }
                    if !field.enabled {
                        part.insert("disabled".into(), json!(true));
                    }
                    Value::Object(part)
                })
                .collect::<Vec<_>>(),
        }),
        Body::Binary(path) => json!({ "mode": "file", "file": { "src": path } }),
    })
}

fn insert_auth(object: &mut Map<String, Value>, auth: &Auth) {
    let pairs = |kind: &str, fields: &[(&str, &str)]| {
        let mut auth = Map::new();
        auth.insert("type".into(), json!(kind));
        auth.insert(
            kind.to_owned(),
            fields
                .iter()
                .map(|(key, value)| json!({ "key": key, "value": value, "type": "string" }))
                .collect(),
        );
        Value::Object(auth)
    };
    let value = match auth {
        Auth::Inherit => return,
        Auth::None => json!({ "type": "noauth" }),
        Auth::Digest { username, password } => pairs(
            "digest",
            &[
                ("username", username.as_str()),
                ("password", password.as_str()),
            ],
        ),
        Auth::AwsSigV4(aws) => pairs(
            "awsv4",
            &[
                ("accessKey", aws.access_key_id.as_str()),
                ("secretKey", aws.secret_access_key.as_str()),
                ("sessionToken", aws.session_token.as_str()),
                ("region", aws.region.as_str()),
                ("service", aws.service.as_str()),
            ],
        ),
        Auth::Basic { username, password } => pairs(
            "basic",
            &[
                ("username", username.as_str()),
                ("password", password.as_str()),
            ],
        ),
        Auth::Bearer { token } => pairs("bearer", &[("token", token.as_str())]),
        Auth::ApiKey {
            name,
            value,
            placement,
        } => pairs(
            "apikey",
            &[
                ("key", name.as_str()),
                ("value", value.as_str()),
                (
                    "in",
                    match placement {
                        KeyPlacement::Header => "header",
                        KeyPlacement::Query => "query",
                    },
                ),
            ],
        ),
        Auth::OAuth2(oauth) => pairs(
            "oauth2",
            &[
                (
                    "grant_type",
                    match oauth.grant {
                        OAuth2Grant::ClientCredentials => CLIENT_CREDENTIALS,
                        OAuth2Grant::AuthorizationCode => AUTHORIZATION_CODE,
                    },
                ),
                ("accessTokenUrl", oauth.token_url.as_str()),
                ("authUrl", oauth.auth_url.as_str()),
                ("redirect_uri", oauth.redirect_url.as_str()),
                ("clientId", oauth.client_id.as_str()),
                ("clientSecret", oauth.client_secret.as_str()),
                ("scope", oauth.scope.as_str()),
            ],
        ),
    };
    object.insert("auth".into(), value);
}

fn insert_events(object: &mut Map<String, Value>, scripts: &Scripts) {
    let events: Vec<Value> = [
        (PREREQUEST_EVENT, &scripts.pre_request),
        (TEST_EVENT, &scripts.post_response),
    ]
    .into_iter()
    .filter(|(_, code)| !code.trim().is_empty())
    .map(|(listen, code)| {
        json!({
            "listen": listen,
            "script": { "type": "text/javascript", "exec": code.lines().collect::<Vec<_>>() },
        })
    })
    .collect();
    if !events.is_empty() {
        object.insert("event".into(), Value::Array(events));
    }
}

/// Variables; secret ones without their value (reported).
fn insert_variables(
    object: &mut Map<String, Value>,
    item: &str,
    variables: &[Variable],
    left_out: &mut Vec<Skipped>,
) {
    if variables.is_empty() {
        return;
    }
    let rows = variables
        .iter()
        .map(|variable| {
            if variable.secret {
                left_out.push(Skipped {
                    item: item.to_owned(),
                    what: SkippedKind::SecretValue(variable.name.clone()),
                });
            }
            let value = if variable.secret { "" } else { &variable.value };
            let mut row = row(&variable.name, value, variable.enabled);
            if let Value::Object(row) = &mut row {
                row.insert(
                    "type".into(),
                    json!(if variable.secret { "secret" } else { "string" }),
                );
            }
            row
        })
        .collect();
    object.insert("variable".into(), Value::Array(rows));
}

/// Redirect, certificate and cookie settings of a folder or request.
fn insert_behavior(
    object: &mut Map<String, Value>,
    transport: &TransportSettings,
    certificate_check: CertificateCheck,
) {
    let mut behavior = Map::new();
    match transport.redirects {
        Redirects::Inherit => {}
        Redirects::Follow => {
            behavior.insert("followRedirects".into(), json!(true));
        }
        Redirects::Stop => {
            behavior.insert("followRedirects".into(), json!(false));
        }
    }
    match certificate_check {
        CertificateCheck::Inherit => {}
        CertificateCheck::On => {
            behavior.insert("strictSSL".into(), json!(true));
        }
        CertificateCheck::Off => {
            behavior.insert("strictSSL".into(), json!(false));
        }
    }
    match transport.cookies {
        Cookies::Inherit => {}
        Cookies::On => {
            behavior.insert("disableCookies".into(), json!(false));
        }
        Cookies::Off => {
            behavior.insert("disableCookies".into(), json!(true));
        }
    }
    if !behavior.is_empty() {
        object.insert("protocolProfileBehavior".into(), Value::Object(behavior));
    }
}

/// Reports a time limit (Postman has none per item) and, for the
/// collection, redirect, certificate and cookie settings.
fn report_settings(
    item: &str,
    transport: &TransportSettings,
    certificate_check: CertificateCheck,
    collection: bool,
    left_out: &mut Vec<Skipped>,
) {
    let mut report = |setting: ItemSetting| {
        left_out.push(Skipped {
            item: item.to_owned(),
            what: SkippedKind::Setting(setting),
        });
    };
    if transport.timeout_seconds.is_some() {
        report(ItemSetting::Timeout);
    }
    if collection && transport.redirects != Redirects::Inherit {
        report(ItemSetting::Redirects);
    }
    if collection && certificate_check != CertificateCheck::Inherit {
        report(ItemSetting::CertificateCheck);
    }
    if collection && transport.cookies != Cookies::Inherit {
        report(ItemSetting::Cookies);
    }
}

/// A `{key, value}` row, `disabled` only when switched off.
fn row(key: &str, value: &str, enabled: bool) -> Value {
    let mut row = Map::new();
    row.insert("key".into(), json!(key));
    row.insert("value".into(), json!(value));
    if !enabled {
        row.insert("disabled".into(), json!(true));
    }
    Value::Object(row)
}

#[cfg(test)]
#[path = "../tests/postman_export.rs"]
mod tests;

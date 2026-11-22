//! Postman collections (format v2.1, also v2.0) as collections.
//!
//! Taken over: folders and requests in order, methods, URLs, headers,
//! bodies (raw JSON/text, URL-encoded, form data with files, binary file),
//! auth (Basic, Bearer, API key, none, inherit) and variables of the
//! collection and of folders. Postman's `{{variable}}` syntax is the same.
//! Scripts are taken over as written (they run with the `pm` object of the
//! script API). What has no counterpart – other auth types, GraphQL bodies – is
//! reported in [`Imported::skipped`](crate::import::Imported::skipped).
//!
//! Read leniently from JSON values: exports differ between Postman versions,
//! and a missing optional field should never make an import fail.

use ac_domain::auth::{Auth, KeyPlacement, OAuth2, OAuth2Grant};
use ac_domain::collection::{
    Body, Collection, Folder, FormField, HeaderEntry, Item, ItemId, MultipartField, Request,
};
use ac_domain::environment::Variable;
use ac_domain::params::{PathParam, QueryParam};
use ac_domain::protocol::grant_type::{AUTHORIZATION_CODE, CLIENT_CREDENTIALS};
use ac_domain::script::Scripts;
use ac_domain::tls::CertificateCheck;
use serde_json::Value;

use crate::import::{Imported, Skipped, SkippedKind, loopback_redirect};

/// Name of a collection without one.
const UNNAMED: &str = "Postman";
/// Event of a script that runs before the request.
const PREREQUEST_EVENT: &str = "prerequest";
/// Event of a script that runs after the response (Postman's "tests").
const TEST_EVENT: &str = "test";

/// Converts an exported Postman collection; `None` if `root` is none
/// (`info` and `item` missing).
pub(crate) fn from_value(root: &Value) -> Option<Imported> {
    let (Some(info), Some(items)) = (root.get("info"), root.get("item")) else {
        return None;
    };
    let name = text_of(info.get("name")).unwrap_or_else(|| UNNAMED.to_owned());
    let mut skipped = Vec::new();
    let auth = auth(root.get("auth"), &name, &mut skipped);
    let collection = Collection {
        transport: ac_domain::transport::TransportSettings::default(),
        scripts: scripts(root),
        id: ItemId::new(),
        items: items_of(items, &mut skipped),
        variables: variables(root.get("variable")),
        // Inherit on the collection means none, as in Postman.
        auth,
        certificate_check: CertificateCheck::Inherit,
        name,
    };
    Some(Imported {
        collection,
        environments: Vec::new(),
        skipped,
    })
}

fn items_of(items: &Value, skipped: &mut Vec<Skipped>) -> Vec<Item> {
    items
        .as_array()
        .map_or_default(|items| items.iter().map(|item| item_of(item, skipped)).collect())
}

fn item_of(item: &Value, skipped: &mut Vec<Skipped>) -> Item {
    let name = text_of(item.get("name")).unwrap_or_default();
    if let Some(children) = item.get("item") {
        return Item::Folder(Folder {
            transport: ac_domain::transport::TransportSettings::default(),
            scripts: scripts(item),
            id: ItemId::new(),
            items: items_of(children, skipped),
            variables: variables(item.get("variable")),
            auth: auth(item.get("auth"), &name, skipped),
            certificate_check: CertificateCheck::Inherit,
            name,
        });
    }
    let mut request = request_of(item.get("request"), name, skipped);
    request.scripts = scripts(item);
    Item::Request(request)
}

fn request_of(request: Option<&Value>, name: String, skipped: &mut Vec<Skipped>) -> Request {
    let mut converted = Request::new(name);
    let Some(request) = request else {
        return converted;
    };
    // A request may be just its URL.
    if let Some(url) = request.as_str() {
        url.clone_into(&mut converted.url);
        return converted;
    }
    if let Some(method) = text_of(request.get("method")).and_then(|m| m.parse().ok()) {
        converted.method = method;
    }
    converted.url = url_of(request.get("url"));
    if let Some(url) = request.get("url").filter(|url| url.is_object()) {
        // Path variables (`:id`) with their values; switched-off query
        // parameters (the others are in the URL already).
        converted.path_params = list(url.get("variable"))
            .map(|(name, value, _)| PathParam { name, value })
            .collect();
        converted.disabled_query = list(url.get("query"))
            .filter(|(_, _, enabled)| !enabled)
            .map(|(name, value, _)| QueryParam { name, value })
            .collect();
    }
    converted.headers = list(request.get("header"))
        .map(|(key, value, enabled)| HeaderEntry {
            name: key,
            value,
            enabled,
        })
        .collect();
    converted.body = body_of(request.get("body"), &converted.name, skipped);
    converted.auth = auth(request.get("auth"), &converted.name, skipped);
    converted
}

fn url_of(url: Option<&Value>) -> String {
    match url {
        Some(Value::String(raw)) => raw.clone(),
        Some(url) => text_of(url.get("raw")).unwrap_or_default(),
        None => String::new(),
    }
}

fn body_of(body: Option<&Value>, item: &str, skipped: &mut Vec<Skipped>) -> Body {
    let Some(body) = body else {
        return Body::None;
    };
    let mode = text_of(body.get("mode")).unwrap_or_default();
    match mode.as_str() {
        "raw" => {
            let content = text_of(body.get("raw")).unwrap_or_default();
            let language = body
                .pointer("/options/raw/language")
                .and_then(Value::as_str)
                .unwrap_or_default();
            if language == "json" {
                Body::Json(content)
            } else {
                Body::Text(content)
            }
        }
        "urlencoded" => Body::Form(
            list(body.get("urlencoded"))
                .map(|(name, value, enabled)| FormField {
                    name,
                    value,
                    enabled,
                })
                .collect(),
        ),
        "formdata" => Body::Multipart(form_data(body.get("formdata"))),
        "file" => Body::Binary(source(body.pointer("/file/src"))),
        "" => Body::None,
        other => {
            skipped.push(Skipped {
                item: item.to_owned(),
                what: SkippedKind::Body(other.to_owned()),
            });
            Body::None
        }
    }
}

fn form_data(fields: Option<&Value>) -> Vec<MultipartField> {
    fields.and_then(Value::as_array).map_or_default(|fields| {
        fields
            .iter()
            .map(|field| {
                let file = field.get("type").and_then(Value::as_str) == Some("file");
                MultipartField {
                    name: text_of(field.get("key")).unwrap_or_default(),
                    value: if file {
                        source(field.get("src"))
                    } else {
                        text_of(field.get("value")).unwrap_or_default()
                    },
                    enabled: !disabled(field),
                    file,
                }
            })
            .collect()
    })
}

/// A file source: a path, or the first of several.
fn source(src: Option<&Value>) -> String {
    match src {
        Some(Value::Array(paths)) => paths.first().and_then(Value::as_str).unwrap_or_default(),
        Some(path) => path.as_str().unwrap_or_default(),
        None => "",
    }
    .to_owned()
}

fn auth(auth: Option<&Value>, item: &str, skipped: &mut Vec<Skipped>) -> Auth {
    let Some(auth) = auth else {
        return Auth::Inherit;
    };
    let kind = text_of(auth.get("type")).unwrap_or_default();
    let field = |name: &str| -> String {
        auth.get(&kind)
            .and_then(Value::as_array)
            .and_then(|fields| {
                fields
                    .iter()
                    .find(|field| field.get("key").and_then(Value::as_str) == Some(name))
            })
            .and_then(|field| text_of(field.get("value")))
            .unwrap_or_default()
    };
    match kind.as_str() {
        "noauth" => Auth::None,
        "inherit" | "" => Auth::Inherit,
        "basic" => Auth::Basic {
            username: field("username"),
            password: field("password"),
        },
        "bearer" => Auth::Bearer {
            token: field("token"),
        },
        "oauth2" if field("grant_type") == CLIENT_CREDENTIALS => Auth::OAuth2(OAuth2 {
            token_url: field("accessTokenUrl"),
            client_id: field("clientId"),
            client_secret: field("clientSecret"),
            scope: field("scope"),
            ..Default::default()
        }),
        // With or without PKCE in Postman: the app always uses it.
        "oauth2" if AUTHORIZATION_CODE_GRANTS.contains(&field("grant_type").as_str()) => {
            Auth::OAuth2(OAuth2 {
                grant: OAuth2Grant::AuthorizationCode,
                token_url: field("accessTokenUrl"),
                auth_url: field("authUrl"),
                redirect_url: loopback_redirect(&field("redirect_uri")),
                client_id: field("clientId"),
                client_secret: field("clientSecret"),
                scope: field("scope"),
            })
        }
        "apikey" => Auth::ApiKey {
            name: field("key"),
            value: field("value"),
            placement: if field("in") == "query" {
                KeyPlacement::Query
            } else {
                KeyPlacement::Header
            },
        },
        other => {
            skipped.push(Skipped {
                item: item.to_owned(),
                what: SkippedKind::Auth(other.to_owned()),
            });
            Auth::Inherit
        }
    }
}

/// Postman's names of the authorization code grant.
const AUTHORIZATION_CODE_GRANTS: [&str; 2] = [AUTHORIZATION_CODE, "authorization_code_with_pkce"];

fn variables(variables: Option<&Value>) -> Vec<Variable> {
    list(variables)
        .map(|(name, value, enabled)| Variable {
            enabled,
            ..Variable::plain(name, value)
        })
        .collect()
}

/// `[{key, value, disabled}]` rows as (key, value, enabled).
fn list(rows: Option<&Value>) -> impl Iterator<Item = (String, String, bool)> + '_ {
    rows.and_then(Value::as_array)
        .into_iter()
        .flatten()
        .map(|row| {
            (
                text_of(row.get("key")).unwrap_or_default(),
                text_of(row.get("value")).unwrap_or_default(),
                !disabled(row),
            )
        })
}

fn disabled(row: &Value) -> bool {
    row.get("disabled").and_then(Value::as_bool) == Some(true)
}

/// Text of a string value; numbers and booleans as written.
fn text_of(value: Option<&Value>) -> Option<String> {
    match value? {
        Value::String(text) => Some(text.clone()),
        Value::Number(number) => Some(number.to_string()),
        Value::Bool(flag) => Some(flag.to_string()),
        _ => None,
    }
}

/// The scripts of an item (`event` with `prerequest` or `test` code), as
/// written: they run with the `pm` object of the script API.
fn scripts(item: &Value) -> Scripts {
    let code = |listen: &str| {
        item.get("event")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter(|event| event.get("listen").and_then(Value::as_str) == Some(listen))
            .filter_map(|event| match event.pointer("/script/exec")? {
                Value::String(text) => Some(text.clone()),
                Value::Array(lines) => Some(
                    lines
                        .iter()
                        .filter_map(Value::as_str)
                        .collect::<Vec<_>>()
                        .join("\n"),
                ),
                _ => None,
            })
            .map(|code| code.trim().to_owned())
            .filter(|code| !code.is_empty())
            .collect::<Vec<_>>()
            .join("\n\n")
    };
    Scripts {
        pre_request: code(PREREQUEST_EVENT),
        post_response: code(TEST_EVENT),
    }
}

#[cfg(test)]
#[path = "tests/postman.rs"]
mod tests;

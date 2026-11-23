//! HAR files (HTTP Archive 1.2, as browsers' developer tools save them) as
//! collections: one request per entry, in the order recorded.
//!
//! When the file marks what each entry was (`_resourceType`, written by
//! Chromium-based browsers), only XHR and fetch requests are taken over –
//! the API calls; images, scripts and style sheets are counted in
//! [`Imported::skipped`](crate::import::Imported::skipped). Headers the
//! client sets itself (HTTP/2 pseudo headers, `Host`, `Content-Length` …)
//! are left out.

use std::collections::BTreeMap;

use ac_domain::collection::{
    Body, Collection, FormField, HeaderEntry, Item, ItemId, MultipartField, Request,
};
use ac_domain::protocol::header;
use ac_domain::tls::CertificateCheck;
use serde_json::Value;

use crate::import::{Imported, Skipped, SkippedKind, text_of};

/// Name of the collection when the file has no page title.
const UNNAMED: &str = "HAR";
/// Resource types that are API calls.
const API_TYPES: [&str; 2] = ["xhr", "fetch"];
/// Headers the HTTP client sets itself, in lower case.
const CLIENT_HEADERS: [&str; 5] = [
    "host",
    "content-length",
    "connection",
    "accept-encoding",
    "transfer-encoding",
];

/// Converts a HAR file; `None` if `root` is none (`log.entries` missing).
pub(crate) fn from_value(root: &Value) -> Option<Imported> {
    let log = root.get("log")?;
    let entries = log.get("entries")?.as_array()?;
    let name = log
        .pointer("/pages/0/title")
        .and_then(Value::as_str)
        .filter(|title| !title.trim().is_empty())
        .unwrap_or(UNNAMED)
        .to_owned();
    let mut other_types = BTreeMap::<String, usize>::new();
    let mut items = Vec::new();
    for entry in entries {
        match entry.get("_resourceType").and_then(Value::as_str) {
            Some(kind) if !API_TYPES.contains(&kind) => {
                *other_types.entry(kind.to_owned()).or_default() += 1;
            }
            _ => {
                if let Some(request) = entry.get("request").map(request_of) {
                    items.push(Item::Request(request));
                }
            }
        }
    }
    let skipped = other_types
        .into_iter()
        .map(|(kind, count)| Skipped {
            item: name.clone(),
            what: SkippedKind::Resources { kind, count },
        })
        .collect();
    Some(Imported {
        collection: Collection {
            id: ItemId::new(),
            name,
            items,
            variables: Vec::new(),
            auth: ac_domain::auth::Auth::Inherit,
            certificate_check: CertificateCheck::Inherit,
            transport: ac_domain::transport::TransportSettings::default(),
            scripts: ac_domain::script::Scripts::default(),
            docs: String::new(),
        },
        environments: Vec::new(),
        skipped,
    })
}

fn request_of(entry: &Value) -> Request {
    let url = text_of(entry.get("url")).unwrap_or_default();
    let method = text_of(entry.get("method")).unwrap_or_default();
    let mut request = Request::new(name_of(&method, &url));
    if let Ok(method) = method.parse() {
        request.method = method;
    }
    request.body = body_of(entry.get("postData"));
    let multipart = matches!(request.body, Body::Multipart(_));
    request.headers = pairs(entry.get("headers"))
        .filter(|(name, _)| !set_by_client(name))
        // The recorded multipart boundary would not match the new body.
        .filter(|(name, _)| !(multipart && name.eq_ignore_ascii_case(header::CONTENT_TYPE)))
        .map(|(name, value)| HeaderEntry {
            name,
            value,
            enabled: true,
        })
        .collect();
    request.url = url;
    request
}

/// `GET /path` – what developer tools show for an entry.
fn name_of(method: &str, url: &str) -> String {
    let path = url::Url::parse(url).map_or_else(|_| url.to_owned(), |url| url.path().to_owned());
    format!("{method} {path}")
}

fn set_by_client(name: &str) -> bool {
    name.starts_with(':') || CLIENT_HEADERS.contains(&name.to_ascii_lowercase().as_str())
}

fn body_of(post: Option<&Value>) -> Body {
    let Some(post) = post else {
        return Body::None;
    };
    let mime = text_of(post.get("mimeType"))
        .unwrap_or_default()
        .to_ascii_lowercase();
    let text = text_of(post.get("text")).unwrap_or_default();
    let params = post.get("params").and_then(Value::as_array);
    if mime.starts_with("multipart/form-data") {
        return Body::Multipart(params.map_or_default(|params| {
            params
                .iter()
                .map(|param| {
                    let file = text_of(param.get("fileName"));
                    MultipartField {
                        name: text_of(param.get("name")).unwrap_or_default(),
                        value: file
                            .clone()
                            .or_else(|| text_of(param.get("value")))
                            .unwrap_or_default(),
                        enabled: true,
                        file: file.is_some(),
                    }
                })
                .collect()
        }));
    }
    if mime.starts_with("application/x-www-form-urlencoded") {
        // Browsers give the fields as `params`, other recorders only `text`.
        let fields: Vec<(String, String)> = if params.is_some_and(|params| !params.is_empty()) {
            pairs(post.get("params")).collect()
        } else {
            url::form_urlencoded::parse(text.as_bytes())
                .map(|(name, value)| (name.into_owned(), value.into_owned()))
                .collect()
        };
        return Body::Form(
            fields
                .into_iter()
                .map(|(name, value)| FormField {
                    name,
                    value,
                    enabled: true,
                })
                .collect(),
        );
    }
    if text.is_empty() {
        Body::None
    } else if mime.contains("json") {
        Body::Json(text)
    } else {
        Body::Text(text)
    }
}

/// `[{name, value}]` rows as (name, value).
fn pairs(rows: Option<&Value>) -> impl Iterator<Item = (String, String)> + '_ {
    rows.and_then(Value::as_array)
        .into_iter()
        .flatten()
        .map(|row| {
            (
                text_of(row.get("name")).unwrap_or_default(),
                text_of(row.get("value")).unwrap_or_default(),
            )
        })
}

#[cfg(test)]
#[path = "tests/har.rs"]
mod tests;

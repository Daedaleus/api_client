//! `OpenAPI` 3.0 and 3.1 descriptions as collections.
//!
//! Every operation becomes a request, grouped in folders by its first tag
//! (in the order of the description's `tags`, then of appearance);
//! operations without a tag stay at the top. The first server becomes the
//! collection variable `baseUrl`; a relative one (`/api/v3`, relative to
//! where the description is served) is resolved against the URL it was
//! downloaded from; from a file it gets `http://localhost` in front and is
//! pointed out. Path parameters and required query
//! parameters become `{{variables}}` of the request with their example as
//! value; header parameters become headers. Bodies are filled with the
//! example of the description, or one built from its schema. Security
//! schemes become auth with `{{token}}`, `{{username}}`, `{{password}}` or
//! `{{apiKey}}` – their values belong in an environment.
//!
//! - this file: the tree.
//! - `request`: one operation as a request.
//! - `schema`: `$ref`s and examples from schemas.

mod request;
mod schema;

use ac_domain::auth::Auth;
use ac_domain::collection::{Collection, Folder, Item, ItemId};
use ac_domain::environment::Variable;
use ac_domain::tls::CertificateCheck;
use serde_json::Value;
use url::Url;

use crate::import::{Imported, Skipped, SkippedKind};
use schema::Document;

/// HTTP methods of a path item, in a stable order.
const METHODS: [&str; 8] = [
    "get", "post", "put", "patch", "delete", "head", "options", "trace",
];

/// Name of a description without a title.
const UNNAMED: &str = "OpenAPI";
/// `baseUrl` when the description names no server.
const NO_SERVER: &str = "http://localhost";
/// The variable the server becomes.
const BASE_URL: &str = "baseUrl";

/// Converts an `OpenAPI` 3 description; `None` if `root` is none.
pub(crate) fn from_value(root: &Value, origin: Option<&Url>) -> Option<Imported> {
    let version = root.get("openapi")?.as_str()?;
    if !version.starts_with('3') {
        return None;
    }
    let document = Document::new(root);
    let name = root
        .pointer("/info/title")
        .and_then(Value::as_str)
        .unwrap_or(UNNAMED)
        .to_owned();
    let mut skipped = Vec::new();
    let auth = request::security(&document, root.get("security"), &name, &mut skipped)
        .unwrap_or(Auth::Inherit);
    let mut folders = Folders::new(root);
    for (path, path_item) in root
        .get("paths")
        .and_then(Value::as_object)
        .into_iter()
        .flatten()
    {
        let path_item = document.resolve(path_item);
        for method in METHODS {
            let Some(operation) = path_item.get(method) else {
                continue;
            };
            let operation = document.resolve(operation);
            let converted = request::operation(
                &document,
                &request::Operation {
                    method,
                    path,
                    path_item,
                    spec: operation,
                    global_auth: &auth,
                },
                &mut skipped,
            );
            let tag = operation
                .get("tags")
                .and_then(Value::as_array)
                .and_then(|tags| tags.first())
                .and_then(Value::as_str);
            folders.add(tag, Item::Request(converted));
        }
    }
    let (base_url, relative) = server_url(root, origin);
    if let Some(url) = relative {
        skipped.push(Skipped {
            item: name.clone(),
            what: SkippedKind::RelativeServer(url),
        });
    }
    Some(Imported {
        collection: Collection {
            transport: ac_domain::transport::TransportSettings::default(),
            scripts: ac_domain::script::Scripts::default(),
            id: ItemId::new(),
            items: folders.into_items(),
            variables: vec![Variable::plain(BASE_URL, base_url)],
            auth,
            certificate_check: CertificateCheck::Inherit,
            name,
        },
        environments: Vec::new(),
        skipped,
    })
}

/// The first server's URL with its variables' defaults filled in, and the
/// URL as given if it was relative and no `origin` resolves it (then it is
/// put below [`NO_SERVER`]).
fn server_url(root: &Value, origin: Option<&Url>) -> (String, Option<String>) {
    let Some(server) = root.pointer("/servers/0") else {
        return (NO_SERVER.to_owned(), None);
    };
    let mut url = server
        .get("url")
        .and_then(Value::as_str)
        .unwrap_or(NO_SERVER)
        .to_owned();
    for (name, variable) in server
        .get("variables")
        .and_then(Value::as_object)
        .into_iter()
        .flatten()
    {
        if let Some(default) = variable.get("default").and_then(Value::as_str) {
            url = url.replace(&format!("{{{name}}}"), default);
        }
    }
    if !url.contains("://")
        && let Some(resolved) = origin.and_then(|origin| origin.join(&url).ok())
    {
        return (resolved.as_str().trim_end_matches('/').to_owned(), None);
    }
    let url = url.trim_end_matches('/');
    if url.contains("://") {
        return (url.to_owned(), None);
    }
    let path = url.trim_start_matches('/');
    let absolute = if path.is_empty() {
        NO_SERVER.to_owned()
    } else {
        format!("{NO_SERVER}/{path}")
    };
    (absolute, Some(url.to_owned()))
}

/// Requests sorted into folders by tag, keeping the order of `tags`.
struct Folders {
    /// Folder names with their requests, in order.
    folders: Vec<(String, Vec<Item>)>,
    /// Requests without a tag.
    untagged: Vec<Item>,
}

impl Folders {
    fn new(root: &Value) -> Self {
        let declared = root
            .get("tags")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(|tag| tag.get("name").and_then(Value::as_str))
            .map(|name| (name.to_owned(), Vec::new()))
            .collect();
        Self {
            folders: declared,
            untagged: Vec::new(),
        }
    }

    fn add(&mut self, tag: Option<&str>, item: Item) {
        let Some(tag) = tag else {
            self.untagged.push(item);
            return;
        };
        match self.folders.iter_mut().find(|(name, _)| name == tag) {
            Some((_, items)) => items.push(item),
            None => self.folders.push((tag.to_owned(), vec![item])),
        }
    }

    /// Folders with requests first, then the untagged requests.
    fn into_items(self) -> Vec<Item> {
        self.folders
            .into_iter()
            .filter(|(_, items)| !items.is_empty())
            .map(|(name, items)| {
                Item::Folder(Folder {
                    transport: ac_domain::transport::TransportSettings::default(),
                    scripts: ac_domain::script::Scripts::default(),
                    id: ItemId::new(),
                    name,
                    items,
                    variables: Vec::new(),
                    auth: Auth::Inherit,
                    certificate_check: CertificateCheck::Inherit,
                })
            })
            .chain(self.untagged)
            .collect()
    }
}

/// For reporting: what an unnamed item is called.
fn skipped(item: &str, what: crate::import::SkippedKind) -> Skipped {
    Skipped {
        item: item.to_owned(),
        what,
    }
}

#[cfg(test)]
#[path = "../tests/openapi.rs"]
mod tests;

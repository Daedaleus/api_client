//! Collections: requests organised in folders, as the user sees and edits
//! them (before variables are resolved).

use std::fmt;
use std::str::FromStr;

use uuid::Uuid;

use crate::auth::Auth;
use crate::environment::Variable;
use crate::params::{PathParam, QueryParam};
use crate::protocol::HttpMethod;
use crate::script::Scripts;
use crate::tls::CertificateCheck;

/// Stable identity of a collection, folder or request. Survives renaming and
/// moving, so references (open tabs, history) stay valid.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ItemId(Uuid);

impl ItemId {
    /// A new, unique id (UUID version 7: sortable by creation time).
    #[must_use]
    pub fn new() -> Self {
        Self(Uuid::now_v7())
    }

    /// An id derived from `key` (UUID version 5): the same key always gives
    /// the same id. For items whose file has no id yet, so they keep one
    /// identity across loads until it is written on the next save.
    #[must_use]
    pub fn derived(key: &str) -> Self {
        Self(Uuid::new_v5(&DERIVED_NAMESPACE, key.as_bytes()))
    }
}

/// Namespace of [`ItemId::derived`] (a fixed, random UUID).
const DERIVED_NAMESPACE: Uuid = Uuid::from_u128(0x6f0c_2d4e_8b1a_4c7e_9f53_2a6d_1e8b_7c40);

impl Default for ItemId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for ItemId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.hyphenated().fmt(f)
    }
}

/// Text that is not a UUID.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("invalid id {0:?}: expected a UUID")]
pub struct InvalidItemId(pub String);

impl FromStr for ItemId {
    type Err = InvalidItemId;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        Uuid::parse_str(text)
            .map(Self)
            .map_err(|_| InvalidItemId(text.to_owned()))
    }
}

/// A collection: the root of a request tree.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Collection {
    /// Identity.
    pub id: ItemId,
    /// Display name.
    pub name: String,
    /// Top-level folders and requests in display order.
    pub items: Vec<Item>,
    /// Variables for every request of the collection (weakest, see
    /// [`crate::scope`]).
    pub variables: Vec<Variable>,
    /// Authentication for its requests (`Inherit` here means none).
    pub auth: Auth,
    /// Certificate check for its requests (`Inherit` here means on).
    pub certificate_check: CertificateCheck,
    /// Scripts around every request of the collection.
    pub scripts: Scripts,
}

impl Collection {
    /// The folder or request with `id` anywhere in the tree.
    #[must_use]
    pub fn find(&self, id: ItemId) -> Option<&Item> {
        find_in(&self.items, id)
    }

    /// The authentication that applies to the request `id`: its own, or
    /// that of the innermost folder or the collection that sets one.
    #[must_use]
    pub fn auth_for(&self, id: ItemId, own: &'_ Auth) -> Auth {
        let folders = self.folders_around(id);
        let chain = std::iter::once(own)
            .chain(folders.iter().rev().map(|folder| &folder.auth))
            .chain(std::iter::once(&self.auth));
        crate::auth::effective(chain).clone()
    }

    /// Whether the certificate is checked for the request `id`: its own
    /// setting, or that of the innermost folder or the collection that has
    /// one; on by default.
    #[must_use]
    pub fn checks_certificates_for(&self, id: ItemId, own: CertificateCheck) -> bool {
        let folders = self.folders_around(id);
        let chain = std::iter::once(own)
            .chain(folders.iter().rev().map(|folder| folder.certificate_check))
            .chain(std::iter::once(self.certificate_check));
        crate::tls::checks_certificates(chain)
    }

    /// The scripts around the request `id`, outermost first: the
    /// collection's, then those of its folders, each with its owner's name.
    /// They run in this order before the request's own pre-request script
    /// and in reverse after its post-response script.
    #[must_use]
    pub fn scripts_around(&self, id: ItemId) -> Vec<(&str, &Scripts)> {
        std::iter::once((self.name.as_str(), &self.scripts))
            .chain(
                self.folders_around(id)
                    .into_iter()
                    .map(|folder| (folder.name.as_str(), &folder.scripts)),
            )
            .filter(|(_, scripts)| !scripts.is_empty())
            .collect()
    }

    /// The folders around the item `id`, outermost first; empty for a
    /// top-level item or an unknown id.
    #[must_use]
    pub fn folders_around(&self, id: ItemId) -> Vec<&Folder> {
        let mut path = Vec::new();
        folders_to(&self.items, id, &mut path);
        path
    }
}

/// Pushes the folders leading to `id` onto `path`; true if found.
fn folders_to<'a>(items: &'a [Item], id: ItemId, path: &mut Vec<&'a Folder>) -> bool {
    for item in items {
        if item.id() == id {
            return true;
        }
        if let Item::Folder(folder) = item {
            path.push(folder);
            if folders_to(&folder.items, id, path) {
                return true;
            }
            path.pop();
        }
    }
    false
}

fn find_in(items: &[Item], id: ItemId) -> Option<&Item> {
    items.iter().find_map(|item| match item {
        _ if item.id() == id => Some(item),
        Item::Folder(folder) => find_in(&folder.items, id),
        Item::Request(_) => None,
    })
}

/// What a collection or folder passes on to the requests in it.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct RequestDefaults {
    /// Variables (see [`crate::scope`]).
    pub variables: Vec<Variable>,
    /// Authentication (see [`crate::auth`]).
    pub auth: Auth,
    /// Certificate check (see [`crate::tls`]).
    pub certificate_check: CertificateCheck,
    /// Scripts around the requests (see [`Collection::scripts_around`]).
    pub scripts: Scripts,
}

/// A folder of a collection.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Folder {
    /// Identity.
    pub id: ItemId,
    /// Display name.
    pub name: String,
    /// Folders and requests in display order.
    pub items: Vec<Item>,
    /// Variables for the requests in this folder and its subfolders.
    pub variables: Vec<Variable>,
    /// Authentication for the requests in it, or inherited.
    pub auth: Auth,
    /// Certificate check for the requests in it, or inherited.
    pub certificate_check: CertificateCheck,
    /// Scripts around the requests in it and its subfolders.
    pub scripts: Scripts,
}

/// An entry of the request tree.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Item {
    /// A folder with further items.
    Folder(Folder),
    /// A request.
    Request(Request),
}

impl Item {
    /// Identity of the folder or request.
    #[must_use]
    pub fn id(&self) -> ItemId {
        match self {
            Self::Folder(folder) => folder.id,
            Self::Request(request) => request.id,
        }
    }

    /// Display name of the folder or request.
    #[must_use]
    pub fn name(&self) -> &str {
        match self {
            Self::Folder(folder) => &folder.name,
            Self::Request(request) => &request.name,
        }
    }
}

/// A saved request as the user edits it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Request {
    /// Identity.
    pub id: ItemId,
    /// Display name.
    pub name: String,
    /// Method.
    pub method: HttpMethod,
    /// URL as entered; may contain `{{variables}}`.
    pub url: String,
    /// Headers in the order they are sent.
    pub headers: Vec<HeaderEntry>,
    /// Body.
    pub body: Body,
    /// Variables of this request only (strongest, see [`crate::scope`]).
    pub variables: Vec<Variable>,
    /// Authentication, or inherited from the folders and collection.
    pub auth: Auth,
    /// Certificate check, or inherited from the folders and collection.
    pub certificate_check: CertificateCheck,
    /// Scripts before sending and after the response.
    pub scripts: Scripts,
    /// Values of the path variables (`:name`) in the URL.
    pub path_params: Vec<PathParam>,
    /// Query parameters switched off (the others are in the URL).
    pub disabled_query: Vec<QueryParam>,
}

impl Request {
    /// A new `GET` request without URL, headers or body.
    #[must_use]
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            id: ItemId::new(),
            name: name.into(),
            method: HttpMethod::Get,
            url: String::new(),
            headers: Vec::new(),
            body: Body::None,
            variables: Vec::new(),
            auth: Auth::Inherit,
            certificate_check: CertificateCheck::Inherit,
            scripts: Scripts::default(),
            path_params: Vec::new(),
            disabled_query: Vec::new(),
        }
    }
}

/// A header of a saved request; disabled headers are kept but not sent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HeaderEntry {
    /// Name.
    pub name: String,
    /// Value; may contain `{{variables}}`.
    pub value: String,
    /// Sent with the request.
    pub enabled: bool,
}

/// Body of a saved request. Text values may contain `{{variables}}`.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum Body {
    /// No body.
    #[default]
    None,
    /// JSON text.
    Json(String),
    /// Plain text.
    Text(String),
    /// URL-encoded form fields (`application/x-www-form-urlencoded`).
    Form(Vec<FormField>),
    /// Multipart form data: text fields and files.
    Multipart(Vec<MultipartField>),
    /// The contents of a file, sent as they are. A relative path is relative
    /// to the collection folder.
    Binary(String),
}

/// A field of a URL-encoded form; disabled fields are kept but not sent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FormField {
    /// Name.
    pub name: String,
    /// Value.
    pub value: String,
    /// Sent with the request.
    pub enabled: bool,
}

/// A field of a multipart body: text, or a file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MultipartField {
    /// Name.
    pub name: String,
    /// The text, or the path of the file (relative = to the collection
    /// folder).
    pub value: String,
    /// Sent with the request.
    pub enabled: bool,
    /// `value` is a file path.
    pub file: bool,
}

#[cfg(test)]
#[path = "tests/collection.rs"]
mod tests;

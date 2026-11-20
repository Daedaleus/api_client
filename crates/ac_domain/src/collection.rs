//! Collections: requests organised in folders, as the user sees and edits
//! them (before variables are resolved).

use std::fmt;
use std::str::FromStr;

use uuid::Uuid;

use crate::protocol::HttpMethod;

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
}

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

/// Body of a saved request. Form, multipart and binary bodies follow with
/// milestone M5.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum Body {
    /// No body.
    #[default]
    None,
    /// JSON text.
    Json(String),
    /// Plain text.
    Text(String),
}

#[cfg(test)]
#[path = "tests/collection.rs"]
mod tests;

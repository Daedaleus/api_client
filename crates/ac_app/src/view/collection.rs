//! Collections and saved requests, prepared for the tree and the editor.
//! Ids travel as text (UUIDs); paths as text in the platform's notation.

use ac_domain::collection::{Body, HeaderEntry, Item, Request};
use serde::{Deserialize, Serialize};

use crate::error::AppError;
use crate::ports::{Location, OpenedCollection, StoreError};
use crate::text::UserText;
use crate::view::{AuthDraft, EnvironmentDraft, VariableDraft};

/// An open collection.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct CollectionView {
    /// Collection id.
    pub id: String,
    /// Display name.
    pub name: String,
    /// Collection folder.
    pub root: String,
    /// Top-level folders and requests in display order.
    pub items: Vec<TreeItemView>,
    /// Environments, sorted by name.
    pub environments: Vec<EnvironmentDraft>,
    /// Variables for every request of the collection.
    pub variables: Vec<VariableDraft>,
    /// Authentication for its requests.
    pub auth: AuthDraft,
    /// Files that could not be loaded.
    pub problems: Vec<UserText>,
}

/// An entry of the request tree.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum TreeItemView {
    /// A folder.
    Folder {
        /// Folder id.
        id: String,
        /// Display name.
        name: String,
        /// Its settings are never written.
        read_only: bool,
        /// Contents in display order.
        items: Vec<TreeItemView>,
        /// Variables for the requests in the folder.
        variables: Vec<VariableDraft>,
        /// Authentication for the requests in the folder.
        auth: AuthDraft,
    },
    /// A request.
    Request {
        /// Request id.
        id: String,
        /// Display name.
        name: String,
        /// Method, e.g. `GET`, for the label in the tree.
        method: String,
        /// The file is never written.
        read_only: bool,
    },
}

impl CollectionView {
    /// The tree of an opened collection.
    #[must_use]
    pub fn from_opened(opened: &OpenedCollection) -> Self {
        let collection = &opened.collection;
        Self {
            id: collection.id.to_string(),
            name: collection.name.clone(),
            root: opened.root.display().to_string(),
            items: tree(&collection.items, opened),
            environments: opened
                .environments
                .iter()
                .map(|environment| {
                    let read_only = opened
                        .locations
                        .get(&environment.id)
                        .is_some_and(|location| location.read_only);
                    EnvironmentDraft::from_environment(environment, read_only)
                })
                .collect(),
            variables: VariableDraft::list(&collection.variables),
            auth: AuthDraft::from_auth(&collection.auth),
            problems: opened.problems.iter().map(StoreError::user_text).collect(),
        }
    }
}

fn tree(items: &[Item], opened: &OpenedCollection) -> Vec<TreeItemView> {
    let read_only = |id| {
        opened
            .locations
            .get(&id)
            .is_some_and(|location: &Location| location.read_only)
    };
    items
        .iter()
        .map(|item| match item {
            Item::Folder(folder) => TreeItemView::Folder {
                id: folder.id.to_string(),
                name: folder.name.clone(),
                read_only: read_only(folder.id),
                items: tree(&folder.items, opened),
                variables: VariableDraft::list(&folder.variables),
                auth: AuthDraft::from_auth(&folder.auth),
            },
            Item::Request(request) => TreeItemView::Request {
                id: request.id.to_string(),
                name: request.name.clone(),
                method: request.method.to_string(),
                read_only: read_only(request.id),
            },
        })
        .collect()
}

/// A saved request as the editor shows and returns it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct RequestDraft {
    /// Request id.
    pub id: String,
    /// Display name.
    pub name: String,
    /// Method.
    pub method: String,
    /// URL as entered; may contain `{{variables}}`.
    pub url: String,
    /// Headers in the order they are sent.
    pub headers: Vec<HeaderDraft>,
    /// Body.
    pub body: BodyDraft,
    /// Variables of this request only.
    #[serde(default)]
    pub variables: Vec<VariableDraft>,
    /// Authentication, or inherited.
    #[serde(default)]
    pub auth: AuthDraft,
    /// The file is never written (set by the app, ignored when saving).
    #[serde(default)]
    pub read_only: bool,
}

/// A header of a [`RequestDraft`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct HeaderDraft {
    /// Name.
    pub name: String,
    /// Value.
    pub value: String,
    /// Sent with the request.
    pub enabled: bool,
}

/// The body of a [`RequestDraft`].
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum BodyDraft {
    /// No body.
    #[default]
    None,
    /// JSON text.
    Json {
        /// The text.
        content: String,
    },
    /// Plain text.
    Text {
        /// The text.
        content: String,
    },
}

impl HeaderDraft {
    /// The editor's view of a header.
    #[must_use]
    pub fn from_entry(header: &HeaderEntry) -> Self {
        Self {
            name: header.name.clone(),
            value: header.value.clone(),
            enabled: header.enabled,
        }
    }

    /// The header this row describes.
    #[must_use]
    pub fn to_entry(&self) -> HeaderEntry {
        HeaderEntry {
            name: self.name.clone(),
            value: self.value.clone(),
            enabled: self.enabled,
        }
    }
}

impl BodyDraft {
    /// The editor's view of a body.
    #[must_use]
    pub fn from_body(body: &Body) -> Self {
        match body {
            Body::None => Self::None,
            Body::Json(content) => Self::Json {
                content: content.clone(),
            },
            Body::Text(content) => Self::Text {
                content: content.clone(),
            },
        }
    }

    /// The body this draft describes.
    #[must_use]
    pub fn to_body(&self) -> Body {
        match self {
            Self::None => Body::None,
            Self::Json { content } => Body::Json(content.clone()),
            Self::Text { content } => Body::Text(content.clone()),
        }
    }
}

impl RequestDraft {
    /// The editor's view of `request`.
    #[must_use]
    pub fn from_request(request: &Request, read_only: bool) -> Self {
        Self {
            id: request.id.to_string(),
            name: request.name.clone(),
            method: request.method.to_string(),
            url: request.url.clone(),
            headers: request
                .headers
                .iter()
                .map(HeaderDraft::from_entry)
                .collect(),
            body: BodyDraft::from_body(&request.body),
            variables: VariableDraft::list(&request.variables),
            auth: AuthDraft::from_auth(&request.auth),
            read_only,
        }
    }

    /// The request the editor describes.
    ///
    /// # Errors
    /// If the id, name or method is not valid.
    pub fn to_request(&self) -> Result<Request, AppError> {
        Ok(Request {
            variables: VariableDraft::to_plain_variables(&self.variables),
            auth: self.auth.to_auth(),
            id: self.id.parse().map_err(|_| AppError::UnknownItem)?,
            name: non_empty(&self.name)?.to_owned(),
            method: self.method.trim().parse()?,
            url: self.url.clone(),
            headers: self.headers.iter().map(HeaderDraft::to_entry).collect(),
            body: self.body.to_body(),
        })
    }
}

/// `name` without surrounding spaces; empty names are refused.
///
/// # Errors
/// [`AppError::EmptyName`] if nothing but spaces is left.
pub fn non_empty(name: &str) -> Result<&str, AppError> {
    let name = name.trim();
    if name.is_empty() {
        Err(AppError::EmptyName)
    } else {
        Ok(name)
    }
}

#[cfg(test)]
#[path = "../tests/view_collection.rs"]
mod tests;

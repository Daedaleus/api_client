//! Commands and queries from the UI and their answers.
//!
//! JSON shape (shared with the TypeScript UI, generated into `bindings.ts`):
//! `{"type": "sendRequest", "method": "GET", "url": "…"}` →
//! `{"type": "runStarted", "runId": 1}`. Ids are UUIDs as text, paths are
//! text in the platform's notation.

use serde::{Deserialize, Serialize};

use crate::view::{
    BodyDraft, CollectionView, EnvironmentDraft, EnvironmentRef, HeaderDraft, RecipientsView,
    RequestDraft,
};

/// Something the user wants the application to do.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(
    tag = "type",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum Command {
    /// Send a request; the result arrives as [`crate::AppEvent::RunFinished`].
    SendRequest {
        /// Method as entered, e.g. `GET`.
        method: String,
        /// URL as entered; `http://` is assumed when no scheme is given.
        url: String,
        /// Header rows; disabled rows and rows without a name are not sent.
        #[serde(default)]
        headers: Vec<HeaderDraft>,
        /// Body; JSON and text get a `Content-Type` unless a header sets one.
        #[serde(default)]
        body: BodyDraft,
        /// Environment for `{{variables}}`; without one, placeholders are
        /// an error.
        #[serde(default)]
        environment: Option<EnvironmentRef>,
    },
    /// Open the collection in a folder.
    OpenCollection {
        /// The collection folder.
        path: String,
    },
    /// Create a new collection in a folder and open it.
    CreateCollection {
        /// The folder (created if needed).
        path: String,
        /// Display name.
        name: String,
    },
    /// Close an open collection (nothing is written).
    CloseCollection {
        /// The collection.
        collection_id: String,
    },
    /// Save a request edited in the editor.
    SaveRequest {
        /// The collection it belongs to.
        collection_id: String,
        /// The edited request (identified by its id).
        request: RequestDraft,
    },
    /// Create a new, empty request.
    CreateRequest {
        /// The collection.
        collection_id: String,
        /// The collection itself or a folder in it.
        parent_id: String,
        /// Display name.
        name: String,
    },
    /// Create a new folder.
    CreateFolder {
        /// The collection.
        collection_id: String,
        /// The collection itself or a folder in it.
        parent_id: String,
        /// Display name.
        name: String,
    },
    /// Rename a folder or request (its file keeps its name).
    RenameItem {
        /// The collection.
        collection_id: String,
        /// The folder or request.
        item_id: String,
        /// New display name.
        name: String,
    },
    /// Move a folder, request or environment to the trash.
    DeleteItem {
        /// The collection.
        collection_id: String,
        /// The folder, request or environment.
        item_id: String,
    },
    /// Create a new, empty environment.
    CreateEnvironment {
        /// The collection.
        collection_id: String,
        /// Display name.
        name: String,
    },
    /// Save an environment edited in the editor (name and variables).
    SaveEnvironment {
        /// The collection.
        collection_id: String,
        /// The edited environment (identified by its id).
        environment: EnvironmentDraft,
    },
    /// Set who the collection's secrets are encrypted for and re-encrypt
    /// the secrets of every environment. Nothing changes if any of them
    /// cannot be decrypted.
    SetRecipients {
        /// The collection.
        collection_id: String,
        /// Public keys (`age1…`, `ssh-ed25519 …`, `ssh-rsa …`).
        recipients: Vec<String>,
    },
}

/// Identifies one sending of a request, so the UI can match the result.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(transparent)]
pub struct RunId(pub u64);

/// Immediate answer to a [`Command`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(
    tag = "type",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum CommandAck {
    /// The request is on its way.
    RunStarted {
        /// Id of this run.
        run_id: RunId,
    },
    /// A collection was opened or changed; here is its current tree.
    CollectionChanged {
        /// The collection as it is now on disk.
        collection: CollectionView,
        /// The item a create command made, if any.
        created_id: Option<String>,
    },
    /// A collection was closed.
    CollectionClosed {
        /// The collection.
        collection_id: String,
    },
}

/// Something the UI wants to read.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(
    tag = "type",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum Query {
    /// The current tree of an open collection, read anew from disk.
    Collection {
        /// The collection.
        collection_id: String,
    },
    /// A saved request for the editor.
    Request {
        /// The collection.
        collection_id: String,
        /// The request.
        request_id: String,
    },
    /// Who the collection's secrets are encrypted for, and the user's own
    /// public key (created on first use).
    Recipients {
        /// The collection.
        collection_id: String,
    },
}

/// Answer to a [`Query`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(
    tag = "type",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum QueryResult {
    /// The collection tree.
    Collection {
        /// The collection.
        collection: CollectionView,
    },
    /// The request.
    Request {
        /// The request.
        request: RequestDraft,
    },
    /// The recipients.
    Recipients {
        /// The recipients and the user's own key.
        recipients: RecipientsView,
    },
}

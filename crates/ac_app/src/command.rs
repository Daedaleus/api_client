//! Commands and queries from the UI and their answers.
//!
//! JSON shape (shared with the TypeScript UI, generated into `bindings.ts`):
//! `{"type": "sendRequest", "method": "GET", "url": "…"}` →
//! `{"type": "runStarted", "runId": 1}`. Ids are UUIDs as text, paths are
//! text in the platform's notation.

use serde::{Deserialize, Serialize};

use crate::text::UserText;

use crate::view::{
    AuthDraft, CertificateCheckView, CollectionView, CookieKeyView, CookieView, EnvironmentDraft,
    ExportFormatView, HistoryItemView, OutgoingRequest, RecipientsView, RequestDraft,
    RunOutcomeView, ScriptsDraft, SettingsView, SnippetLanguageView, TransportDraft, VariableDraft,
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
        /// The request as entered.
        #[serde(flatten)]
        request: OutgoingRequest,
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
    /// Copy a folder (with everything in it) or request as `name`, placed
    /// right after it; the copy gets new ids.
    DuplicateItem {
        /// The collection.
        collection_id: String,
        /// The folder or request.
        item_id: String,
        /// Name of the copy.
        name: String,
    },
    /// Move a folder or request into the collection or one of its folders,
    /// or to another place in the same folder.
    MoveItem {
        /// The collection.
        collection_id: String,
        /// The folder or request.
        item_id: String,
        /// The collection or folder it goes into.
        parent_id: String,
        /// Its position there among the other items (0 = first).
        index: usize,
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
    /// Save what the collection or one of its folders passes on to its
    /// requests: variables and authentication.
    SaveDefaults {
        /// The collection.
        collection_id: String,
        /// The collection itself or a folder in it.
        item_id: String,
        /// The variables in display order (no secrets).
        variables: Vec<VariableDraft>,
        /// The authentication (`inherit` on the collection means none).
        auth: AuthDraft,
        /// The certificate check (`inherit` on the collection means on).
        certificate_check: CertificateCheckView,
        /// Scripts around the requests in it.
        #[serde(default)]
        scripts: ScriptsDraft,
        /// Redirects and time limit (`inherit` on the collection means the
        /// defaults).
        #[serde(default)]
        transport: TransportDraft,
    },
    /// Import another tool's collection as a new collection and open it:
    /// a Postman collection (v2.1, v2.0) or an `OpenAPI` 3 description (JSON
    /// or YAML), detected from the file.
    ImportCollection {
        /// The file to import.
        file: String,
        /// The folder for the new collection (created if needed); without
        /// one the collection stays unsaved until `SaveCollectionAs`.
        #[serde(default)]
        path: Option<String>,
    },
    /// Write a collection in another tool's format to `file` (replacing
    /// it); what the format has no place for is listed.
    ExportCollection {
        /// The collection.
        collection_id: String,
        /// The format.
        format: ExportFormatView,
        /// The file to write.
        file: String,
    },
    /// Save an unsaved collection: move it to `path` (which must not exist
    /// or be an empty folder).
    SaveCollectionAs {
        /// The collection.
        collection_id: String,
        /// Where it goes.
        path: String,
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
    /// Add a cookie to a jar, or replace `original` with it.
    SaveCookie {
        /// The collection; `null` for requests outside a collection.
        #[serde(default)]
        collection_id: Option<String>,
        /// The cookie being edited; `null` to add one.
        #[serde(default)]
        original: Option<CookieKeyView>,
        /// The cookie.
        cookie: CookieView,
    },
    /// Remove one cookie from a jar.
    DeleteCookie {
        /// The collection; `null` for requests outside a collection.
        #[serde(default)]
        collection_id: Option<String>,
        /// The cookie.
        cookie: CookieKeyView,
    },
    /// Remove all cookies of a jar.
    ClearCookies {
        /// The collection; `null` for requests outside a collection.
        #[serde(default)]
        collection_id: Option<String>,
    },
    /// Remove every entry of the request history.
    ClearHistory,
    /// Apply and save the app's settings (checked first: nothing is saved
    /// if they cannot be applied).
    SaveSettings {
        /// The settings.
        settings: SettingsView,
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
        collection: Box<CollectionView>,
        /// The item a create command made, if any.
        created_id: Option<String>,
    },
    /// A collection was imported and opened.
    CollectionImported {
        /// The new collection.
        collection: Box<CollectionView>,
        /// What could not be taken over, to show the user.
        skipped: Vec<UserText>,
    },
    /// A cookie jar changed.
    CookiesChanged {
        /// Its cookies now.
        cookies: Vec<CookieView>,
    },
    /// A collection was exported.
    CollectionExported {
        /// What the format has no place for, to show the user.
        left_out: Vec<UserText>,
    },
    /// A collection was closed.
    CollectionClosed {
        /// The collection.
        collection_id: String,
    },
    /// The request history was cleared.
    HistoryCleared,
    /// The app's settings were applied and saved.
    SettingsSaved,
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
    /// The cURL command for a request as entered (variables and inherited
    /// auth applied, as when sending).
    Curl {
        /// The request as entered.
        #[serde(flatten)]
        request: Box<OutgoingRequest>,
    },
    /// Code that sends a request as entered, in `language` (variables and
    /// inherited auth applied, as when sending).
    Snippet {
        /// The language.
        language: SnippetLanguageView,
        /// The request as entered.
        #[serde(flatten)]
        request: Box<OutgoingRequest>,
    },
    /// The request history, newest first.
    History,
    /// The app's settings.
    Settings,
    /// One entry of the request history, to open as a new tab.
    HistoryEntry {
        /// The entry.
        id: String,
    },
    /// A pasted cURL command as a new, unsaved request.
    ParseCurl {
        /// The command.
        text: String,
    },
    /// Who the collection's secrets are encrypted for, and the user's own
    /// public key (created on first use).
    Recipients {
        /// The collection.
        collection_id: String,
    },
    /// The cookies of a jar that have not expired.
    Cookies {
        /// The collection; `null` for requests outside a collection.
        #[serde(default)]
        collection_id: Option<String>,
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
    /// The cURL command.
    Curl {
        /// For a POSIX shell, one option per line.
        command: String,
    },
    /// Code that sends the request.
    Snippet {
        /// The code.
        code: String,
    },
    /// The app's settings.
    Settings {
        /// The settings.
        settings: SettingsView,
        /// Why the saved settings could not be applied at start, if so.
        problem: Option<UserText>,
    },
    /// The request history, newest first.
    History {
        /// The entries.
        entries: Vec<HistoryItemView>,
    },
    /// One entry of the history, to open as a new tab.
    HistoryEntry {
        /// Its request (not saved, no id).
        request: Box<RequestDraft>,
        /// Its response or error.
        outcome: Box<RunOutcomeView>,
    },
    /// The request a cURL command describes.
    ParsedCurl {
        /// The request (not saved, no id).
        request: RequestDraft,
        /// Options that were not taken over, as written.
        ignored: Vec<String>,
    },
    /// The recipients.
    Recipients {
        /// The recipients and the user's own key.
        recipients: RecipientsView,
    },
    /// The cookies of a jar.
    Cookies {
        /// In the order they were first set.
        cookies: Vec<CookieView>,
    },
}

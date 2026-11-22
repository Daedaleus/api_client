//! The facade every user interface talks to.
//!
//! - this file: [`App`], its adapters ([`Ports`]) and the dispatch of
//!   commands and queries.
//! - `runs`: sending requests in the background.
//! - `script_runs`: a run with the request's scripts; variables they set.
//! - `collections`: open collections and changes to them.
//! - `environments`: environments of open collections.
//! - `secrets`: secret variable values (decrypt for sending, encrypt on save).
//! - `recipients`: who secrets are encrypted for, re-encrypting on change.
//! - `runner`: running a collection's requests one after another (CLI).
//! - `interchange`: cURL export and import.
//! - `defaults`: what the collection and its folders pass on to requests.
//! - `arrange`: duplicating and moving requests and folders.
//! - `assist`: checking and completing scripts in the editor.
//! - `history`: recording sent requests; listing, opening, clearing them.

mod arrange;
mod assist;
mod collections;
mod defaults;
mod environments;
mod history;
mod interchange;
mod recipients;
mod runner;
mod runs;
mod script_runs;
mod secrets;

use defaults::DefaultsDraft;
pub use runner::RunPlan;

use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::atomic::AtomicU64;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use ac_domain::collection::ItemId;
use tokio::runtime::Handle;
use tokio::sync::broadcast;

use crate::command::{Command, CommandAck, Query, QueryResult};
use crate::error::AppError;
use crate::event::AppEvent;
use crate::oauth::TokenCache;
use crate::ports::{BrowserLogin, CollectionStore, HttpSender, ScriptRunner, SecretVault};
use crate::ports::{HistoryStore, ScriptAssist};
use script_runs::RuntimeVariables;

/// Events kept for slow subscribers before they start missing some.
const EVENT_CAPACITY: usize = 64;

/// The adapters the application works with.
pub struct Ports<H, S, V, B, R, A, Y> {
    /// Sends HTTP requests.
    pub http: H,
    /// Reads and writes collections.
    pub store: S,
    /// Encrypts and decrypts secrets.
    pub vault: V,
    /// Signs in in the browser (OAuth 2 authorization code).
    pub login: B,
    /// Runs request scripts.
    pub scripts: R,
    /// Checks and completes scripts in the editor.
    pub assist: A,
    /// Keeps the request history.
    pub history: Y,
}

/// The application: send [`Command`]s and [`Query`]s, receive
/// [`AppEvent`]s.
///
/// Cheap to clone; clones share state.
pub struct App<H, S, V, B, R, A, Y> {
    inner: Arc<Inner<H, S, V, B, R, A, Y>>,
}

struct Inner<H, S, V, B, R, A, Y> {
    http: H,
    store: S,
    vault: V,
    login: B,
    scripts: R,
    assist: A,
    history: Y,
    /// Variables set by scripts, in memory only.
    runtime_variables: Mutex<RuntimeVariables>,
    /// Background work runs here, whichever thread calls `execute` – UIs
    /// may call from threads without a runtime (Tauri: the main thread).
    runtime: Handle,
    events: broadcast::Sender<AppEvent>,
    next_run: AtomicU64,
    /// OAuth 2 access tokens, in memory only.
    tokens: TokenCache,
    /// Open collections (id and folder) in the order they were opened. Their
    /// contents are read anew for every command, so changes made outside the
    /// app are picked up.
    open: Mutex<Vec<(ItemId, PathBuf)>>,
    /// Open collections that are not saved yet (in a temporary folder).
    temporary: Mutex<HashSet<ItemId>>,
}

impl<H, S, V, B, R, A, Y> Clone for App<H, S, V, B, R, A, Y> {
    fn clone(&self) -> Self {
        Self {
            inner: Arc::clone(&self.inner),
        }
    }
}

impl<
    H: HttpSender,
    S: CollectionStore,
    V: SecretVault,
    B: BrowserLogin,
    R: ScriptRunner,
    A: ScriptAssist,
    Y: HistoryStore,
> App<H, S, V, B, R, A, Y>
{
    /// Creates the application with its adapters; background work runs on
    /// `runtime`.
    #[must_use]
    pub fn new(ports: Ports<H, S, V, B, R, A, Y>, runtime: Handle) -> Self {
        let (events, _) = broadcast::channel(EVENT_CAPACITY);
        Self {
            inner: Arc::new(Inner {
                http: ports.http,
                store: ports.store,
                vault: ports.vault,
                login: ports.login,
                scripts: ports.scripts,
                assist: ports.assist,
                history: ports.history,
                runtime_variables: Mutex::default(),
                runtime,
                events,
                next_run: AtomicU64::new(1),
                tokens: TokenCache::default(),
                open: Mutex::new(Vec::new()),
                temporary: Mutex::default(),
            }),
        }
    }

    /// Receives all events from now on.
    #[must_use]
    pub fn subscribe(&self) -> broadcast::Receiver<AppEvent> {
        self.inner.events.subscribe()
    }

    /// Carries out a command. Long work continues in the background and
    /// reports through [`App::subscribe`]. May be called from any thread.
    ///
    /// # Errors
    /// [`AppError`] if the command cannot be carried out as given.
    pub fn execute(&self, command: Command) -> Result<CommandAck, AppError> {
        match command {
            Command::SendRequest { request } => self.send_request(&request),
            Command::OpenCollection { path } => self.open_collection(&path),
            Command::CreateCollection { path, name } => self.create_collection(&path, &name),
            Command::CloseCollection { collection_id } => self.close_collection(&collection_id),
            Command::SaveRequest {
                collection_id,
                request,
            } => self.save_request(&collection_id, &request),
            Command::CreateRequest {
                collection_id,
                parent_id,
                name,
            } => self.create_request(&collection_id, &parent_id, &name),
            Command::CreateFolder {
                collection_id,
                parent_id,
                name,
            } => self.create_folder(&collection_id, &parent_id, &name),
            Command::DuplicateItem {
                collection_id,
                item_id,
                name,
            } => self.duplicate_item(&collection_id, &item_id, &name),
            Command::MoveItem {
                collection_id,
                item_id,
                parent_id,
                index,
            } => self.move_item(&collection_id, &item_id, &parent_id, index),
            Command::RenameItem {
                collection_id,
                item_id,
                name,
            } => self.rename_item(&collection_id, &item_id, &name),
            Command::DeleteItem {
                collection_id,
                item_id,
            } => self.delete_item(&collection_id, &item_id),
            Command::CreateEnvironment {
                collection_id,
                name,
            } => self.create_environment(&collection_id, &name),
            Command::SaveEnvironment {
                collection_id,
                environment,
            } => self.save_environment(&collection_id, &environment),
            Command::SaveDefaults {
                collection_id,
                item_id,
                variables,
                auth,
                certificate_check,
                scripts,
                transport,
            } => self.save_defaults(
                &collection_id,
                &item_id,
                &DefaultsDraft {
                    variables: &variables,
                    auth: &auth,
                    certificate_check,
                    scripts: &scripts,
                    transport,
                },
            ),
            Command::ImportCollection { file, path } => {
                self.import_collection(&file, path.as_deref())
            }
            Command::SaveCollectionAs {
                collection_id,
                path,
            } => self.save_collection_as(&collection_id, &path),
            Command::SetRecipients {
                collection_id,
                recipients,
            } => self.set_recipients(&collection_id, &recipients),
            Command::ClearHistory => self.clear_history(),
        }
    }

    /// Answers a query. May be called from any thread.
    ///
    /// # Errors
    /// [`AppError`] if the collection or item is unknown or cannot be read.
    pub fn query(&self, query: Query) -> Result<QueryResult, AppError> {
        match query {
            Query::Collection { collection_id } => self.query_collection(&collection_id),
            Query::Request {
                collection_id,
                request_id,
            } => self.query_request(&collection_id, &request_id),
            Query::Recipients { collection_id } => self.query_recipients(&collection_id),
            Query::Curl { request } => self.query_curl(&request),
            Query::ParseCurl { text } => interchange::parse_curl(&text),
            Query::History => self.query_history(),
            Query::HistoryEntry { id } => self.query_history_entry(&id),
        }
    }

    fn temporary(&self) -> MutexGuard<'_, HashSet<ItemId>> {
        // Ids are inserted or removed whole; a panic leaves the set usable.
        self.inner
            .temporary
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
    }

    fn open_collections(&self) -> MutexGuard<'_, Vec<(ItemId, PathBuf)>> {
        // A panic while holding the lock leaves the list consistent (it is
        // only ever pushed to or removed from).
        self.inner
            .open
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
    }
}

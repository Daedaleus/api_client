//! The facade every user interface talks to.
//!
//! - this file: [`App`], its adapters ([`Ports`]) and the dispatch of
//!   commands and queries.
//! - `runs`: sending requests in the background.
//! - `collections`: open collections and changes to them.
//! - `environments`: environments of open collections.
//! - `secrets`: secret variable values (decrypt for sending, encrypt on save).
//! - `recipients`: who secrets are encrypted for, re-encrypting on change.
//! - `defaults`: what the collection and its folders pass on to requests.

mod collections;
mod defaults;
mod environments;
mod recipients;
mod runs;
mod secrets;

use std::path::PathBuf;
use std::sync::atomic::AtomicU64;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use ac_domain::collection::ItemId;
use tokio::runtime::Handle;
use tokio::sync::broadcast;

use crate::command::{Command, CommandAck, Query, QueryResult};
use crate::error::AppError;
use crate::event::AppEvent;
use crate::ports::{CollectionStore, HttpSender, SecretVault};
use runs::Outgoing;

/// Events kept for slow subscribers before they start missing some.
const EVENT_CAPACITY: usize = 64;

/// The adapters the application works with.
pub struct Ports<H, S, V> {
    /// Sends HTTP requests.
    pub http: H,
    /// Reads and writes collections.
    pub store: S,
    /// Encrypts and decrypts secrets.
    pub vault: V,
}

/// The application: send [`Command`]s and [`Query`]s, receive
/// [`AppEvent`]s.
///
/// Cheap to clone; clones share state.
pub struct App<H, S, V> {
    inner: Arc<Inner<H, S, V>>,
}

struct Inner<H, S, V> {
    http: H,
    store: S,
    vault: V,
    /// Background work runs here, whichever thread calls `execute` – UIs
    /// may call from threads without a runtime (Tauri: the main thread).
    runtime: Handle,
    events: broadcast::Sender<AppEvent>,
    next_run: AtomicU64,
    /// Open collections (id and folder) in the order they were opened. Their
    /// contents are read anew for every command, so changes made outside the
    /// app are picked up.
    open: Mutex<Vec<(ItemId, PathBuf)>>,
}

impl<H, S, V> Clone for App<H, S, V> {
    fn clone(&self) -> Self {
        Self {
            inner: Arc::clone(&self.inner),
        }
    }
}

impl<H: HttpSender, S: CollectionStore, V: SecretVault> App<H, S, V> {
    /// Creates the application with its adapters; background work runs on
    /// `runtime`.
    #[must_use]
    pub fn new(ports: Ports<H, S, V>, runtime: Handle) -> Self {
        let (events, _) = broadcast::channel(EVENT_CAPACITY);
        Self {
            inner: Arc::new(Inner {
                http: ports.http,
                store: ports.store,
                vault: ports.vault,
                runtime,
                events,
                next_run: AtomicU64::new(1),
                open: Mutex::new(Vec::new()),
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
            Command::SendRequest {
                method,
                url,
                headers,
                body,
                variables,
                auth,
                certificate_check,
                context,
            } => self.send_request(
                &Outgoing {
                    method: &method,
                    url: &url,
                    headers: &headers,
                    body: &body,
                    variables: &variables,
                    auth: &auth,
                    certificate_check,
                },
                context.as_ref(),
            ),
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
            } => self.save_defaults(
                &collection_id,
                &item_id,
                &variables,
                &auth,
                certificate_check,
            ),
            Command::SetRecipients {
                collection_id,
                recipients,
            } => self.set_recipients(&collection_id, &recipients),
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
        }
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

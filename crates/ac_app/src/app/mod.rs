//! The facade every user interface talks to.
//!
//! - this file: [`App`], its adapters ([`Ports`]) and the dispatch of
//!   commands and queries.
//! - `runs`: sending requests in the background.
//! - `collections`: open collections and changes to them.

mod collections;
mod runs;

use std::path::PathBuf;
use std::sync::atomic::AtomicU64;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use ac_domain::collection::ItemId;
use tokio::runtime::Handle;
use tokio::sync::broadcast;

use crate::command::{Command, CommandAck, Query, QueryResult};
use crate::error::AppError;
use crate::event::AppEvent;
use crate::ports::{CollectionStore, HttpSender};

/// Events kept for slow subscribers before they start missing some.
const EVENT_CAPACITY: usize = 64;

/// The adapters the application works with.
pub struct Ports<H, S> {
    /// Sends HTTP requests.
    pub http: H,
    /// Reads and writes collections.
    pub store: S,
}

/// The application: send [`Command`]s and [`Query`]s, receive
/// [`AppEvent`]s.
///
/// Cheap to clone; clones share state.
pub struct App<H, S> {
    inner: Arc<Inner<H, S>>,
}

struct Inner<H, S> {
    http: H,
    store: S,
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

impl<H, S> Clone for App<H, S> {
    fn clone(&self) -> Self {
        Self {
            inner: Arc::clone(&self.inner),
        }
    }
}

impl<H: HttpSender, S: CollectionStore> App<H, S> {
    /// Creates the application with its adapters; background work runs on
    /// `runtime`.
    #[must_use]
    pub fn new(ports: Ports<H, S>, runtime: Handle) -> Self {
        let (events, _) = broadcast::channel(EVENT_CAPACITY);
        Self {
            inner: Arc::new(Inner {
                http: ports.http,
                store: ports.store,
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
            } => self.send_request(&method, &url, &headers, &body),
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

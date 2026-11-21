//! Application layer of the API client.
//!
//! Every user interface (Tauri, later egui or the command line) talks only to
//! [`App`]: it sends [`Command`]s (answered at once with a [`CommandAck`])
//! and [`Query`]s, and receives the results of long work as [`AppEvent`]s.
//! Everything a UI shows comes as plain serializable view models; display
//! texts travel as [`UserText`] keys and are translated only in the UI.
//!
//! Modules:
//! - [`app`]: the facade (request runs, open collections).
//! - [`command`]: commands, queries and their answers.
//! - [`error`]: why a command cannot be carried out.
//! - [`event`]: events for subscribers.
//! - [`view`]: view models (responses, collection tree, request drafts).
//! - [`ports`]: interfaces to the outside world (HTTP, collection files),
//!   implemented by adapter crates.
//! - [`text`]: translation keys for display texts.
//! - `testing` (feature `testing`): test doubles for the ports.

pub mod app;
pub mod command;
pub mod error;
pub mod event;
pub mod ports;
mod resolve;
mod resolve_body;
#[cfg(test)]
#[path = "tests/support.rs"]
mod test_support;
#[cfg(any(test, feature = "testing"))]
pub mod testing;
pub mod text;
pub mod view;

pub use app::{App, Ports, RunPlan};
pub use command::{Command, CommandAck, Query, QueryResult, RunId};
pub use error::AppError;
pub use event::AppEvent;
pub use ports::{CollectionStore, HttpSender, StoreError, TransportError};
pub use text::UserText;

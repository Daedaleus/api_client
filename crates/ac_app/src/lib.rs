//! Application layer of the API client.
//!
//! Every user interface (Tauri, later egui or the command line) talks only to
//! [`App`]: it sends [`Command`]s, gets a [`CommandAck`] at once and receives
//! the results of long work as [`AppEvent`]s. Everything a UI shows comes as
//! plain serializable view models; display texts travel as [`UserText`] keys
//! and are translated only in the UI.
//!
//! Modules:
//! - [`app`]: the facade.
//! - [`command`]: commands, acknowledgements and their errors.
//! - [`event`]: events for subscribers.
//! - [`view`]: view models; [`body`] builds the body preview.
//! - [`ports`]: interfaces to the outside world (HTTP …), implemented by
//!   adapter crates.
//! - [`text`]: translation keys for display texts.
//! - `testing` (feature `testing`): test doubles for the ports.

pub mod app;
pub mod body;
pub mod command;
pub mod event;
pub mod ports;
mod resolve;
#[cfg(any(test, feature = "testing"))]
pub mod testing;
pub mod text;
pub mod view;

pub use app::{App, Ports};
pub use command::{AppError, Command, CommandAck, RunId};
pub use event::AppEvent;
pub use ports::{HttpSender, TransportError};
pub use text::UserText;

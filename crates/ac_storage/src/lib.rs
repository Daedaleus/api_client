//! Storage adapter: a collection is a folder of TOML files.
//!
//! The format is specified in `docs/format/v1.md`. Principles:
//! - **Never lose user data.** Files are written atomically (temporary file,
//!   then rename); damaged files are reported and never written; files from
//!   a newer format version are read-only; removing goes to a discard step
//!   chosen by the caller (the system trash in the app), never a hard delete.
//! - **Hand edits survive.** Saving rewrites only changed values, so
//!   comments, key order and unknown keys stay.
//! - **Opening changes nothing.** Older versions are migrated in memory and
//!   written only on save; items without an id get a stable derived one.
//!
//! Modules:
//! - [`format`]: format names, versions, file and folder names.
//! - [`error`]: what can go wrong, per file.
//! - [`load`]: reading a collection into the domain tree.
//! - [`request_file`]: reading and writing one request file.
//! - [`settings`]: folder settings (name, id).
//! - [`environment_file`]: environment files (variables).
//! - [`secrets_file`]: encrypted secrets and their recipients (bytes only).
//! - `rows`: name/value/enabled lists (headers, variables).
//! - `variables`: variable lists in collection, folder, environment and
//!   request files.
//! - `auth`: authentication tables in request, folder and collection files.
//! - [`create`]: creating collections, folders and requests.
//! - [`remove`]: removing requests and folders.
//! - [`store`]: [`FileStore`], the `CollectionStore` port of `ac_app`.
//! - [`names`]: file and folder names from display names.
//! - `order`: keeping `order` lists in step with their folder.
//! - `document`: reading and updating TOML values in place, atomic writes.

mod auth;
pub mod create;
mod document;
pub mod environment_file;
pub mod error;
pub mod format;
pub mod load;
pub mod names;
mod order;
pub mod remove;
pub mod request_file;
mod rows;
pub mod secrets_file;
pub mod settings;
pub mod store;
#[cfg(test)]
#[path = "tests/support.rs"]
mod test_support;
mod variables;

pub use create::{create_collection, create_folder, create_request};
pub use environment_file::{create_environment, read_environment, save_environment};
pub use error::{LoadProblem, StorageError};
pub use load::{Entry, EntryKind, LoadedCollection, load_collection};
pub use remove::remove_item;
pub use request_file::{read_request, save_request};
pub use settings::save_folder_settings;
pub use store::FileStore;

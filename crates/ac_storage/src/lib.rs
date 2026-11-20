//! Storage adapter: a collection is a folder of TOML files.
//!
//! The format is specified in `docs/format/v1.md`. Principles:
//! - **Never lose user data.** Files are written atomically (temporary file,
//!   then rename); damaged files are reported and never written; files from
//!   a newer format version are read-only.
//! - **Hand edits survive.** Saving rewrites only changed values, so
//!   comments, key order and unknown keys stay.
//! - **Opening changes nothing.** Older versions are migrated in memory and
//!   written only on save.
//!
//! Modules:
//! - [`format`]: format names, versions, file and folder names.
//! - [`error`]: what can go wrong, per file.
//! - [`load`]: reading a collection into the domain tree.
//! - [`request_file`]: reading and writing one request file.
//! - [`create`]: creating collections and requests (file names, order).
//! - `document`: helpers for reading and updating TOML values in place.

pub mod create;
mod document;
pub mod error;
pub mod format;
pub mod load;
pub mod request_file;
#[cfg(test)]
#[path = "tests/support.rs"]
mod test_support;

pub use create::{create_collection, create_request};
pub use error::{LoadProblem, StorageError};
pub use load::{Entry, EntryKind, LoadedCollection, load_collection};
pub use request_file::{read_request, save_request};

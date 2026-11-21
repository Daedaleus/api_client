//! Domain model of the API client.
//!
//! Pure types and rules without I/O, async or UI dependencies, so every other
//! crate can build on it and everything here is easy to test.
//!
//! Modules:
//! - [`protocol`]: values from the HTTP protocol (methods, status codes,
//!   header names) as types.
//! - [`auth`]: how requests authenticate, inherited from folders and the
//!   collection.
//! - [`collection`]: collections, folders and saved requests as the user
//!   edits them.
//! - [`environment`]: environments, variables and `{{name}}` placeholders.
//! - [`scope`]: which value wins when variables are defined in several
//!   places (collection, environment, folders, request).
//! - [`exchange`]: one HTTP exchange – the request as sent and the response
//!   as received.

pub mod auth;
pub mod collection;
pub mod environment;
pub mod exchange;
pub mod protocol;
pub mod scope;

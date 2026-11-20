//! Domain model of the API client.
//!
//! Pure types and rules without I/O, async or UI dependencies, so every other
//! crate can build on it and everything here is easy to test.
//!
//! Modules:
//! - [`protocol`]: values from the HTTP protocol (methods, status codes,
//!   header names) as types.
//! - [`collection`]: collections, folders and saved requests as the user
//!   edits them.
//! - [`environment`]: environments, variables and `{{name}}` placeholders.
//! - [`exchange`]: one HTTP exchange – the request as sent and the response
//!   as received.

pub mod collection;
pub mod environment;
pub mod exchange;
pub mod protocol;

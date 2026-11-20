//! Domain model of the API client.
//!
//! Pure types and rules without I/O, async or UI dependencies, so every other
//! crate can build on it and everything here is easy to test.
//!
//! Modules:
//! - [`protocol`]: values from the HTTP protocol (methods, …) as types.

pub mod protocol;

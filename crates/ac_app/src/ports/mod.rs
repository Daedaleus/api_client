//! Interfaces to the outside world. Adapter crates implement them; tests use
//! the doubles in `testing`.
//!
//! - [`http`]: sending requests ([`HttpSender`]).
//! - [`store`]: reading and writing collections ([`CollectionStore`]).
//! - [`secrets`]: encrypting secret values ([`SecretVault`]).

pub mod http;
pub mod secrets;
pub mod store;

pub use http::{HttpSender, TransportError};
pub use secrets::{SecretError, SecretValues, SecretVault};
pub use store::{CollectionStore, Location, OpenedCollection, StoreError, StoreErrorKind};

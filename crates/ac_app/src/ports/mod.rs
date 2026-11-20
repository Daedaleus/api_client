//! Interfaces to the outside world. Adapter crates implement them; tests use
//! the doubles in `testing`.
//!
//! - [`http`]: sending requests ([`HttpSender`]).
//! - [`store`]: reading and writing collections ([`CollectionStore`]).

pub mod http;
pub mod store;

pub use http::{HttpSender, TransportError};
pub use store::{CollectionStore, Location, OpenedCollection, StoreError, StoreErrorKind};

//! Interfaces to the outside world. Adapter crates implement them; tests use
//! the doubles in `testing`.
//!
//! - [`http`]: sending requests ([`HttpSender`]).
//! - [`store`]: reading and writing collections ([`CollectionStore`]).
//! - [`secrets`]: encrypting secret values ([`SecretVault`]).
//! - [`login`]: signing in in the browser ([`BrowserLogin`]).
//! - [`script`]: running request scripts ([`ScriptRunner`]).

pub mod http;
pub mod login;
pub mod script;
pub mod secrets;
pub mod store;

pub use http::{HttpSender, TransportError};
pub use login::{BrowserLogin, LoginError, PendingLogin};
pub use script::{
    ScriptError, ScriptJob, ScriptOutcome, ScriptRequest, ScriptResponse, ScriptRunner, TestResult,
};
pub use secrets::{SecretError, SecretValues, SecretVault};
pub use store::{
    CollectionStore, Location, OpenedCollection, StoreError, StoreErrorKind, TextFile,
};

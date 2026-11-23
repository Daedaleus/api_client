//! Test doubles for the ports (feature `testing`).
//!
//! - [`http`]: [`FakeHttpSender`] with queued replies.
//! - [`store`]: [`MemoryStore`], collections kept in memory.
//! - [`secrets`]: [`FakeVault`], readable "encryption" with fake keys.
//! - [`login`]: [`FakeBrowserLogin`], sign-ins with queued answers.
//! - [`script`]: [`FakeScriptRunner`], scripts with queued results.
//! - [`assist`]: [`FakeScriptAssist`], checks and completions with queued answers.
//! - [`history`]: [`MemoryHistory`], the request history in memory.
//! - [`settings`]: [`MemorySettings`], the app's settings in memory.

pub mod assist;
pub mod history;
pub mod http;
pub mod login;
pub mod script;
pub mod secrets;
pub mod settings;
pub mod store;

pub use assist::FakeScriptAssist;
pub use history::MemoryHistory;
pub use http::{FAKE_NOW, FakeHttpSender, response};
pub use login::{FAKE_PORT, FakeBrowserLogin, LoginAnswer};
pub use script::FakeScriptRunner;
pub use secrets::FakeVault;
pub use settings::MemorySettings;
pub use store::MemoryStore;

use std::sync::{Mutex, MutexGuard, PoisonError};

/// A panicking test poisons a lock; the data is still fine for asserts.
fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

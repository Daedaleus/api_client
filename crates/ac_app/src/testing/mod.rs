//! Test doubles for the ports (feature `testing`).
//!
//! - [`http`]: [`FakeHttpSender`] with queued replies.
//! - [`store`]: [`MemoryStore`], collections kept in memory.

pub mod http;
pub mod store;

pub use http::{FakeHttpSender, response};
pub use store::MemoryStore;

use std::sync::{Mutex, MutexGuard, PoisonError};

/// A panicking test poisons a lock; the data is still fine for asserts.
fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

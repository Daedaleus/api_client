//! Redirects and time limit in request, folder and collection files:
//!
//! ```toml
//! follow_redirects = false   # true / false; missing = inherit
//! timeout_seconds = 10       # whole seconds ≥ 1; missing = inherit
//! cookies = false            # true / false; missing = inherit
//! ```

use std::path::Path;

use ac_domain::transport::{Cookies, Redirects, TransportSettings};
use toml_edit::TableLike;

use crate::document;
use crate::error::StorageError;

const FOLLOW_REDIRECTS: &str = "follow_redirects";
const TIMEOUT_SECONDS: &str = "timeout_seconds";
const COOKIES: &str = "cookies";

/// Reads the settings in `table` (missing keys inherit).
pub(crate) fn read(table: &dyn TableLike, path: &Path) -> Result<TransportSettings, StorageError> {
    let redirects = match document::opt_bool(table, FOLLOW_REDIRECTS, path)? {
        None => Redirects::Inherit,
        Some(true) => Redirects::Follow,
        Some(false) => Redirects::Stop,
    };
    let timeout_seconds = match table.get(TIMEOUT_SECONDS) {
        None => None,
        Some(item) => Some(
            item.as_integer()
                .and_then(|seconds| u32::try_from(seconds).ok())
                .filter(|seconds| *seconds > 0)
                .ok_or_else(|| {
                    StorageError::invalid(path, TIMEOUT_SECONDS, "must be a whole number ≥ 1")
                })?,
        ),
    };
    let cookies = match document::opt_bool(table, COOKIES, path)? {
        None => Cookies::Inherit,
        Some(true) => Cookies::On,
        Some(false) => Cookies::Off,
    };
    Ok(TransportSettings {
        redirects,
        timeout_seconds,
        cookies,
    })
}

/// Writes what changed into `table` (inherit removes the key).
pub(crate) fn write(
    table: &mut dyn TableLike,
    before: &TransportSettings,
    after: &TransportSettings,
) {
    if before.redirects != after.redirects {
        match after.redirects {
            Redirects::Inherit => {
                table.remove(FOLLOW_REDIRECTS);
            }
            Redirects::Follow => document::set(table, FOLLOW_REDIRECTS, true),
            Redirects::Stop => document::set(table, FOLLOW_REDIRECTS, false),
        }
    }
    if before.timeout_seconds != after.timeout_seconds {
        match after.timeout_seconds {
            None => {
                table.remove(TIMEOUT_SECONDS);
            }
            Some(seconds) => document::set(table, TIMEOUT_SECONDS, i64::from(seconds)),
        }
    }
    if before.cookies != after.cookies {
        match after.cookies {
            Cookies::Inherit => {
                table.remove(COOKIES);
            }
            Cookies::On => document::set(table, COOKIES, true),
            Cookies::Off => document::set(table, COOKIES, false),
        }
    }
}

#[cfg(test)]
#[path = "tests/transport.rs"]
mod tests;

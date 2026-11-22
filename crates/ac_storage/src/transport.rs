//! Redirects and time limit in request, folder and collection files:
//!
//! ```toml
//! follow_redirects = false   # true / false; missing = inherit
//! timeout_seconds = 10       # whole seconds ≥ 1; missing = inherit
//! ```

use std::path::Path;

use ac_domain::transport::{Redirects, TransportSettings};
use toml_edit::TableLike;

use crate::document;
use crate::error::StorageError;

const FOLLOW_REDIRECTS: &str = "follow_redirects";
const TIMEOUT_SECONDS: &str = "timeout_seconds";

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
    Ok(TransportSettings {
        redirects,
        timeout_seconds,
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
}

#[cfg(test)]
#[path = "tests/transport.rs"]
mod tests;

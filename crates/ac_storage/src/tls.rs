//! The certificate check in request, folder and collection files:
//! `verify_tls = false` switches it off, `true` switches it on again below
//! a folder or collection that switched it off; missing = inherit.

use std::path::Path;

use ac_domain::tls::CertificateCheck;
use toml_edit::TableLike;

use crate::document;
use crate::error::StorageError;

/// The key.
pub(crate) const KEY: &str = "verify_tls";

/// Reads the setting in `table`.
pub(crate) fn read(table: &dyn TableLike, path: &Path) -> Result<CertificateCheck, StorageError> {
    Ok(match document::opt_bool(table, KEY, path)? {
        None => CertificateCheck::Inherit,
        Some(true) => CertificateCheck::On,
        Some(false) => CertificateCheck::Off,
    })
}

/// Writes the setting into `table` if it changed (inherit removes the key).
pub(crate) fn write(table: &mut dyn TableLike, before: CertificateCheck, after: CertificateCheck) {
    if before == after {
        return;
    }
    match after {
        CertificateCheck::Inherit => {
            table.remove(KEY);
        }
        CertificateCheck::On => document::set(table, KEY, true),
        CertificateCheck::Off => document::set(table, KEY, false),
    }
}

#[cfg(test)]
#[path = "tests/tls.rs"]
mod tests;

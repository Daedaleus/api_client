//! Encrypted secrets and their recipients (ADR S4): `secrets/<name>.age`
//! belongs to `environments/<name>.toml`; `.age-recipients` in the collection
//! root lists the public keys to encrypt for. This module only moves bytes
//! and lines – encryption is the job of `ac_secrets`.

use std::path::{Path, PathBuf};

use crate::document;
use crate::error::StorageError;
use crate::format::{RECIPIENTS_FILE, SECRETS_EXTENSION, SECRETS_FOLDER};

/// Comment at the top of a new recipients file.
const RECIPIENTS_HEADER: &str = "\
# Public keys the secrets of this collection are encrypted for, one per line
# (age1… or ssh-ed25519 …/ssh-rsa …). After a change, save an environment
# with secrets to re-encrypt them.
";

/// The secrets file of the environment file `environment_file`.
#[must_use]
pub fn secrets_path(environment_file: &Path) -> PathBuf {
    let stem = environment_file.file_stem().unwrap_or_default();
    let root = environment_file
        .parent()
        .and_then(Path::parent)
        .unwrap_or_else(|| Path::new(""));
    root.join(SECRETS_FOLDER)
        .join(stem)
        .with_extension(SECRETS_EXTENSION)
}

/// The encrypted secrets of an environment, `None` if it has none.
///
/// # Errors
/// If the file exists but cannot be read.
pub fn read_secrets(environment_file: &Path) -> Result<Option<Vec<u8>>, StorageError> {
    let path = secrets_path(environment_file);
    match std::fs::read(&path) {
        Ok(data) => Ok(Some(data)),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(StorageError::io(&path, error)),
    }
}

/// Writes the encrypted secrets of an environment atomically.
///
/// # Errors
/// If the folder or file cannot be written.
pub fn write_secrets(environment_file: &Path, data: &[u8]) -> Result<(), StorageError> {
    let path = secrets_path(environment_file);
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(|error| StorageError::io(dir, error))?;
    }
    // age armor is ASCII; anything else is a bug, not user data to keep.
    let text = std::str::from_utf8(data)
        .map_err(|_| StorageError::invalid(&path, "secrets", "must be ASCII-armored"))?;
    document::write_atomically(&path, text)
}

/// The recipients in `.age-recipients` (comments and blank lines left out);
/// none if the file does not exist.
///
/// # Errors
/// If the file exists but cannot be read.
pub fn read_recipients(root: &Path) -> Result<Vec<String>, StorageError> {
    let path = root.join(RECIPIENTS_FILE);
    match std::fs::read_to_string(&path) {
        Ok(text) => Ok(text
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty() && !line.starts_with('#'))
            .map(str::to_owned)
            .collect()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(Vec::new()),
        Err(error) => Err(StorageError::io(&path, error)),
    }
}

/// Sets the recipients in `.age-recipients`. A new file gets an explaining
/// comment; in an existing one comments and blank lines stay, removed keys
/// go and new keys are appended – hand edits survive.
///
/// # Errors
/// If the existing file cannot be read or the file cannot be written.
pub fn write_recipients(root: &Path, recipients: &[String]) -> Result<(), StorageError> {
    let path = root.join(RECIPIENTS_FILE);
    let existing = match std::fs::read_to_string(&path) {
        Ok(text) => text,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => RECIPIENTS_HEADER.to_owned(),
        Err(error) => return Err(StorageError::io(&path, error)),
    };
    let mut kept: Vec<&str> = Vec::new();
    let mut text = String::new();
    for line in existing.lines() {
        let key = line.trim();
        let is_key = !key.is_empty() && !key.starts_with('#');
        if is_key && (kept.contains(&key) || !recipients.iter().any(|r| r.trim() == key)) {
            continue;
        }
        if is_key {
            kept.push(key);
        }
        text.push_str(line);
        text.push('\n');
    }
    for recipient in recipients {
        let recipient = recipient.trim();
        if !kept.contains(&recipient) {
            kept.push(recipient);
            text.push_str(recipient);
            text.push('\n');
        }
    }
    document::write_atomically(&path, &text)
}

#[cfg(test)]
#[path = "tests/secrets_file.rs"]
mod tests;

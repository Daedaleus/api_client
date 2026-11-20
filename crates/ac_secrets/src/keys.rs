//! The user's keys (identities): the own key file in the app's config folder
//! (created on first use), a key from the [`CI_VARIABLE`] and unencrypted
//! SSH keys. Private keys never leave this module.

use std::fs::OpenOptions;
use std::io::{BufReader, ErrorKind, Write};
use std::path::Path;

use ac_app::ports::SecretError;
use age::secrecy::ExposeSecret;
use age::{Identity, IdentityFile, x25519};

/// Environment variable with the content of an age key file (for CI).
pub const CI_VARIABLE: &str = "API_CLIENT_IDENTITY";
/// Environment variable with another path for the own key file (other
/// location, isolated tests).
pub const IDENTITY_FILE_VARIABLE: &str = "API_CLIENT_IDENTITY_FILE";

/// SSH keys tried for decrypting, in `~/.ssh`.
const SSH_KEYS: [&str; 2] = ["id_ed25519", "id_rsa"];

const SECRET_KEY_PREFIX: &str = "AGE-SECRET-KEY-";

type BoxedIdentity = Box<dyn Identity + Send + Sync>;

/// The own key from `file`; creates the file with a new key if missing.
pub(crate) fn own_identity(file: &Path) -> Result<x25519::Identity, SecretError> {
    match std::fs::read_to_string(file) {
        Ok(text) => parse_own(&text, file),
        Err(error) if error.kind() == ErrorKind::NotFound => create(file),
        Err(error) => Err(key_error(file, error)),
    }
}

fn parse_own(text: &str, file: &Path) -> Result<x25519::Identity, SecretError> {
    text.lines()
        .map(str::trim)
        .find(|line| line.starts_with(SECRET_KEY_PREFIX))
        .and_then(|line| line.parse().ok())
        .ok_or_else(|| key_error(file, "no age key in the file"))
}

fn create(file: &Path) -> Result<x25519::Identity, SecretError> {
    if let Some(dir) = file.parent() {
        std::fs::create_dir_all(dir).map_err(|error| key_error(dir, error))?;
    }
    let identity = x25519::Identity::generate();
    let text = format!(
        "# API Client: your private key for secrets. Never share or commit it.\n\
         # public key: {}\n{}\n",
        identity.to_public(),
        identity.to_string().expose_secret(),
    );
    // `create_new`: never overwrite a key another instance just wrote.
    match OpenOptions::new().write(true).create_new(true).open(file) {
        Ok(mut out) => {
            out.write_all(text.as_bytes())
                .and_then(|()| out.sync_all())
                .map_err(|error| key_error(file, error))?;
            Ok(identity)
        }
        Err(error) if error.kind() == ErrorKind::AlreadyExists => own_identity(file),
        Err(error) => Err(key_error(file, error)),
    }
}

/// Every key that may decrypt: own file (if it exists), CI variable, SSH.
pub(crate) fn identities(config: &crate::KeyConfig) -> Result<Vec<BoxedIdentity>, SecretError> {
    let mut identities = Vec::new();
    match std::fs::read(&config.identity_file) {
        Ok(data) => identities.extend(identity_file(&data, &config.identity_file)?),
        Err(error) if error.kind() == ErrorKind::NotFound => {}
        Err(error) => return Err(key_error(&config.identity_file, error)),
    }
    if let Some(text) = &config.ci_identity {
        identities.extend(identity_file(
            text.as_bytes(),
            Path::new(crate::CI_VARIABLE),
        )?);
    }
    if let Some(dir) = &config.ssh_dir {
        identities.extend(
            SSH_KEYS
                .iter()
                .filter_map(|name| ssh_identity(&dir.join(name))),
        );
    }
    Ok(identities)
}

fn identity_file(data: &[u8], source: &Path) -> Result<Vec<BoxedIdentity>, SecretError> {
    IdentityFile::from_buffer(data)
        .map_err(|error| key_error(source, error))?
        .into_identities()
        .map_err(|error| key_error(source, error))
}

/// An unencrypted SSH key; missing, encrypted or unsupported keys are skipped
/// (they are optional, and asking for passphrases is not supported yet).
fn ssh_identity(path: &Path) -> Option<BoxedIdentity> {
    let file = std::fs::File::open(path).ok()?;
    let identity = age::ssh::Identity::from_buffer(BufReader::new(file), None).ok()?;
    matches!(identity, age::ssh::Identity::Unencrypted(_))
        .then(|| Box::new(identity) as BoxedIdentity)
}

fn key_error(path: &Path, error: impl std::fmt::Display) -> SecretError {
    SecretError::Key(format!("{}: {error}", path.display()))
}

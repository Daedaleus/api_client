//! Secrets adapter: environment secrets encrypted with
//! [age](https://age-encryption.org/) for several recipients
//! (ADR S4, `docs/adr/s4-age-keys.md`).
//!
//! - this file: [`AgeVault`], the [`SecretVault`] implementation.
//! - `keys`: the user's keys – own key file, CI variable, SSH keys.
//! - `recipients`: public keys as written in `.age-recipients`.
//! - `payload`: the small document inside an encrypted file.

mod keys;
mod payload;
mod recipients;

use std::io::{Read, Write};
use std::path::PathBuf;

use ac_app::ports::{SecretError, SecretValues, SecretVault};
use age::armor::{ArmoredReader, ArmoredWriter, Format};
use age::secrecy::zeroize::Zeroize;
use age::{DecryptError, Decryptor, Encryptor};

pub use keys::{CI_VARIABLE, IDENTITY_FILE_VARIABLE};

/// Where the user's keys are.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeyConfig {
    /// The user's own key file, created on first use (app config folder).
    pub identity_file: PathBuf,
    /// Content of a key file from [`CI_VARIABLE`] (CI, scripts).
    pub ci_identity: Option<String>,
    /// Folder with SSH keys (`~/.ssh`); unencrypted ones can decrypt too.
    pub ssh_dir: Option<PathBuf>,
}

impl KeyConfig {
    /// The own key file (`default_file`, or [`IDENTITY_FILE_VARIABLE`] if
    /// set) plus what the process environment offers: the [`CI_VARIABLE`]
    /// and the SSH keys in the home folder.
    #[must_use]
    pub fn from_environment(default_file: PathBuf) -> Self {
        Self {
            identity_file: std::env::var_os(IDENTITY_FILE_VARIABLE)
                .map_or(default_file, PathBuf::from),
            ci_identity: std::env::var(CI_VARIABLE).ok(),
            ssh_dir: std::env::home_dir().map(|home| home.join(".ssh")),
        }
    }
}

/// Encrypts and decrypts secrets with age.
#[derive(Debug, Clone)]
pub struct AgeVault {
    keys: KeyConfig,
}

impl AgeVault {
    /// A vault using the keys in `keys`.
    #[must_use]
    pub fn new(keys: KeyConfig) -> Self {
        Self { keys }
    }
}

impl SecretVault for AgeVault {
    fn own_recipient(&self) -> Result<String, SecretError> {
        Ok(keys::own_identity(&self.keys.identity_file)?
            .to_public()
            .to_string())
    }

    fn check_recipient(&self, recipient: &str) -> Result<(), SecretError> {
        recipients::parse(recipient.trim()).map(drop)
    }

    fn encrypt(
        &self,
        values: &SecretValues,
        recipients: &[String],
    ) -> Result<Vec<u8>, SecretError> {
        let recipients = recipients::parse_all(recipients)?;
        let encryptor = Encryptor::with_recipients(recipients.iter().map(|r| r.as_ref() as _))
            .map_err(|error| SecretError::InvalidRecipient(error.to_string()))?;
        let mut plaintext = payload::encode(values);
        let result = write_armored(encryptor, plaintext.as_bytes());
        plaintext.zeroize();
        result.map_err(|error| SecretError::Damaged(error.to_string()))
    }

    fn decrypt(&self, data: &[u8]) -> Result<SecretValues, SecretError> {
        let identities = keys::identities(&self.keys)?;
        if identities.is_empty() {
            return Err(SecretError::NoKey);
        }
        let decryptor = Decryptor::new_buffered(ArmoredReader::new(data)).map_err(damaged)?;
        let mut reader = decryptor
            .decrypt(identities.iter().map(|identity| identity.as_ref() as _))
            .map_err(|error| match error {
                DecryptError::NoMatchingKeys => SecretError::NoKey,
                other => damaged(other),
            })?;
        let mut plaintext = String::new();
        let read = reader.read_to_string(&mut plaintext);
        let values = read
            .map_err(damaged)
            .and_then(|_| payload::decode(&plaintext));
        plaintext.zeroize();
        values
    }
}

fn write_armored(encryptor: Encryptor, plaintext: &[u8]) -> std::io::Result<Vec<u8>> {
    let armor = ArmoredWriter::wrap_output(Vec::new(), Format::AsciiArmor)?;
    let mut writer = encryptor.wrap_output(armor)?;
    writer.write_all(plaintext)?;
    writer.finish()?.finish()
}

fn damaged(error: impl std::fmt::Display) -> SecretError {
    SecretError::Damaged(error.to_string())
}

#[cfg(test)]
#[path = "tests/vault.rs"]
mod tests;

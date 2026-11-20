//! Encrypting secret variable values (ADR S4, `docs/adr/s4-age-keys.md`).
//! Implemented by `ac_secrets` (age); the app never sees keys, only the
//! user's public key and encrypted bytes.

use std::collections::BTreeMap;

use crate::text::{UserText, keys};

/// Secret values of one environment by variable name.
pub type SecretValues = BTreeMap<String, String>;

/// Encrypts and decrypts the secret values of environments.
pub trait SecretVault: Send + Sync + 'static {
    /// The user's own public key; creates the key on first use.
    ///
    /// # Errors
    /// If the key cannot be read or created.
    fn own_recipient(&self) -> Result<String, SecretError>;

    /// Encrypts `values` for every one of `recipients` (public keys as
    /// written in `.age-recipients`).
    ///
    /// # Errors
    /// If there is no recipient or one is not a valid public key.
    fn encrypt(&self, values: &SecretValues, recipients: &[String])
    -> Result<Vec<u8>, SecretError>;

    /// Decrypts what [`SecretVault::encrypt`] produced with one of the user's
    /// keys.
    ///
    /// # Errors
    /// If none of the user's keys fits or the data is damaged.
    fn decrypt(&self, data: &[u8]) -> Result<SecretValues, SecretError>;
}

/// Why secrets cannot be encrypted or decrypted.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum SecretError {
    /// None of the user's keys can decrypt the file.
    #[error("no key of this user can decrypt the secrets")]
    NoKey,
    /// No recipient to encrypt for.
    #[error("no recipients")]
    NoRecipients,
    /// A line of `.age-recipients` is not a public key.
    #[error("invalid recipient {0:?}")]
    InvalidRecipient(String),
    /// The encrypted file is damaged.
    #[error("damaged secrets: {0}")]
    Damaged(String),
    /// The user's own key cannot be read or created.
    #[error("key file: {0}")]
    Key(String),
}

impl SecretError {
    /// The text to show the user.
    #[must_use]
    pub fn user_text(&self) -> UserText {
        match self {
            Self::NoKey => UserText::new(keys::SECRET_NO_KEY),
            Self::NoRecipients => UserText::new(keys::SECRET_NO_RECIPIENTS),
            Self::InvalidRecipient(recipient) => {
                UserText::new(keys::SECRET_INVALID_RECIPIENT).with("recipient", recipient.clone())
            }
            Self::Damaged(detail) => {
                UserText::new(keys::SECRET_DAMAGED).with("detail", detail.clone())
            }
            Self::Key(detail) => UserText::new(keys::SECRET_KEY).with("detail", detail.clone()),
        }
    }
}

impl<T: SecretVault> SecretVault for std::sync::Arc<T> {
    fn own_recipient(&self) -> Result<String, SecretError> {
        T::own_recipient(self)
    }
    fn encrypt(
        &self,
        values: &SecretValues,
        recipients: &[String],
    ) -> Result<Vec<u8>, SecretError> {
        T::encrypt(self, values, recipients)
    }
    fn decrypt(&self, data: &[u8]) -> Result<SecretValues, SecretError> {
        T::decrypt(self, data)
    }
}

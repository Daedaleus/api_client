//! Test double for the secret vault: "encrypts" into readable JSON that
//! names its recipients, so tests can check who could decrypt what.

use std::sync::Mutex;

use serde::{Deserialize, Serialize};

use super::lock;
use crate::ports::{SecretError, SecretValues, SecretVault};

/// A [`SecretVault`] with a fixed own key.
#[derive(Debug)]
pub struct FakeVault {
    key: String,
    fail_next: Mutex<Option<SecretError>>,
}

#[derive(Serialize, Deserialize)]
struct Sealed {
    recipients: Vec<String>,
    values: SecretValues,
}

impl FakeVault {
    /// The own public key of every `FakeVault::default()`.
    pub const OWN_KEY: &str = "age1me";

    /// A vault whose own public key is `key`.
    #[must_use]
    pub fn with_key(key: &str) -> Self {
        Self {
            key: key.to_owned(),
            fail_next: Mutex::new(None),
        }
    }

    /// Makes the next call fail with `error`.
    pub fn fail_next(&self, error: SecretError) {
        *lock(&self.fail_next) = Some(error);
    }

    /// "Encrypts" `values` for `recipients` without a vault (test data).
    #[must_use]
    pub fn seal(values: &SecretValues, recipients: &[&str]) -> Vec<u8> {
        let sealed = Sealed {
            recipients: recipients.iter().map(|&r| r.to_owned()).collect(),
            values: values.clone(),
        };
        serde_json::to_vec(&sealed).unwrap_or_default()
    }

    fn check_failure(&self) -> Result<(), SecretError> {
        lock(&self.fail_next).take().map_or(Ok(()), Err)
    }
}

impl Default for FakeVault {
    fn default() -> Self {
        Self::with_key(Self::OWN_KEY)
    }
}

impl SecretVault for FakeVault {
    fn own_recipient(&self) -> Result<String, SecretError> {
        self.check_failure()?;
        Ok(self.key.clone())
    }

    fn check_recipient(&self, recipient: &str) -> Result<(), SecretError> {
        if recipient.starts_with("age1") {
            Ok(())
        } else {
            Err(SecretError::InvalidRecipient(recipient.to_owned()))
        }
    }

    fn encrypt(
        &self,
        values: &SecretValues,
        recipients: &[String],
    ) -> Result<Vec<u8>, SecretError> {
        self.check_failure()?;
        if recipients.is_empty() {
            return Err(SecretError::NoRecipients);
        }
        for recipient in recipients {
            self.check_recipient(recipient)?;
        }
        let recipients: Vec<&str> = recipients.iter().map(String::as_str).collect();
        Ok(Self::seal(values, &recipients))
    }

    fn decrypt(&self, data: &[u8]) -> Result<SecretValues, SecretError> {
        self.check_failure()?;
        let sealed: Sealed = serde_json::from_slice(data)
            .map_err(|error| SecretError::Damaged(error.to_string()))?;
        if sealed.recipients.contains(&self.key) {
            Ok(sealed.values)
        } else {
            Err(SecretError::NoKey)
        }
    }
}

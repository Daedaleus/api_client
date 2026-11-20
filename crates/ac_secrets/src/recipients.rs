//! Public keys to encrypt for: `age1…` keys and SSH public keys
//! (`ssh-ed25519 …`, `ssh-rsa …`), one per line of `.age-recipients`.

use ac_app::ports::SecretError;
use age::{Recipient, x25519};

const AGE_PREFIX: &str = "age1";
const SSH_PREFIX: &str = "ssh-";

/// Parses every recipient.
///
/// # Errors
/// [`SecretError::NoRecipients`] for an empty list,
/// [`SecretError::InvalidRecipient`] for the first line that is no public key.
pub(crate) fn parse_all(lines: &[String]) -> Result<Vec<Box<dyn Recipient + Send>>, SecretError> {
    if lines.is_empty() {
        return Err(SecretError::NoRecipients);
    }
    lines.iter().map(|line| parse(line.trim())).collect()
}

fn parse(line: &str) -> Result<Box<dyn Recipient + Send>, SecretError> {
    let invalid = || SecretError::InvalidRecipient(line.to_owned());
    if line.starts_with(AGE_PREFIX) {
        let recipient: x25519::Recipient = line.parse().map_err(|_| invalid())?;
        Ok(Box::new(recipient))
    } else if line.starts_with(SSH_PREFIX) {
        let recipient: age::ssh::Recipient = line.parse().map_err(|_| invalid())?;
        Ok(Box::new(recipient))
    } else {
        Err(invalid())
    }
}

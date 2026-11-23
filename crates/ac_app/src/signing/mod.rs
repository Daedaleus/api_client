//! Authentication that needs the finished request: AWS Signature Version 4
//! signs it, HTTP Digest answers the server's challenge. Both run when
//! sending, after variables are filled in (see `resolve`).
//!
//! - [`aws`]: AWS Signature Version 4.
//! - [`digest`]: HTTP Digest (RFC 7616).
//!
//! Pure functions: time and the client nonce are passed in, so the
//! official test vectors can be checked.

pub(crate) mod aws;
pub(crate) mod digest;

use std::fmt::Write as _;

/// How a request is signed when it is sent.
#[derive(Clone, PartialEq, Eq)]
pub(crate) enum Signer {
    /// Answer a Digest challenge with these credentials.
    Digest {
        /// User name.
        username: String,
        /// Password.
        password: String,
    },
    /// Sign with AWS Signature Version 4.
    Aws(aws::AwsCredentials),
}

impl std::fmt::Debug for Signer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Digest { username, .. } => f
                .debug_struct("Digest")
                .field("username", username)
                .finish_non_exhaustive(),
            Self::Aws(credentials) => f
                .debug_struct("Aws")
                .field("access_key_id", &credentials.access_key_id)
                .field("region", &credentials.region)
                .field("service", &credentials.service)
                .finish_non_exhaustive(),
        }
    }
}

/// The current time in seconds since 1970 (UTC).
pub(crate) fn unix_now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |elapsed| {
            i64::try_from(elapsed.as_secs()).unwrap_or(i64::MAX)
        })
}

/// Lower-case hexadecimal.
pub(crate) fn hex(bytes: &[u8]) -> String {
    bytes
        .iter()
        .fold(String::with_capacity(bytes.len() * 2), |mut text, byte| {
            let _ = write!(text, "{byte:02x}");
            text
        })
}

#[cfg(test)]
#[path = "../tests/signing.rs"]
mod tests;

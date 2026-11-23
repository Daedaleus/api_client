//! Cookies as the cookie manager shows and edits them (see
//! `ac_domain::cookie`).

use ac_domain::cookie::{Cookie, UnixSeconds};
use serde::{Deserialize, Serialize};

/// One cookie of a jar (serialized in camelCase).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct CookieView {
    /// Name.
    pub name: String,
    /// Value.
    pub value: String,
    /// The host it belongs to.
    pub domain: String,
    /// Only for exactly `domain`, not its subdomains.
    pub host_only: bool,
    /// Sent for this path and below.
    pub path: String,
    /// When it expires, in seconds since 1970; `null` for a session
    /// cookie. A number (not an integer type) so the UI gets a plain
    /// JavaScript number.
    pub expires: Option<f64>,
    /// Only over secure connections.
    pub secure: bool,
    /// Marked `HttpOnly` by the server.
    pub http_only: bool,
}

// `expires` is never NaN (whole seconds from the jar or the UI), so the
// equality of `f64` is an equivalence here.
impl Eq for CookieView {}

/// What identifies a cookie in its jar (serialized in camelCase).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct CookieKeyView {
    /// Name.
    pub name: String,
    /// Domain.
    pub domain: String,
    /// Path.
    pub path: String,
}

impl CookieView {
    /// The manager's view of `cookie`.
    #[must_use]
    #[expect(
        clippy::cast_precision_loss,
        reason = "seconds since 1970 stay far below 2^53"
    )]
    pub fn from_cookie(cookie: &Cookie) -> Self {
        Self {
            name: cookie.name.clone(),
            value: cookie.value.clone(),
            domain: cookie.domain.clone(),
            host_only: cookie.host_only,
            path: cookie.path.clone(),
            expires: cookie.expires.map(|seconds| seconds as f64),
            secure: cookie.secure,
            http_only: cookie.http_only,
        }
    }

    /// The cookie the manager describes (names and domains trimmed, the
    /// domain in lower case).
    #[must_use]
    #[expect(
        clippy::cast_possible_truncation,
        reason = "whole seconds; out-of-range values saturate"
    )]
    pub fn to_cookie(&self) -> Cookie {
        Cookie {
            name: self.name.trim().to_owned(),
            value: self.value.clone(),
            domain: self
                .domain
                .trim()
                .trim_start_matches('.')
                .to_ascii_lowercase(),
            host_only: self.host_only,
            path: self.path.trim().to_owned(),
            expires: self.expires.map(|seconds| seconds as UnixSeconds),
            secure: self.secure,
            http_only: self.http_only,
        }
    }
}

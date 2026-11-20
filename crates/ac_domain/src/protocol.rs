//! Values from the HTTP protocol, bundled as types instead of loose literals.

use std::fmt;
use std::str::FromStr;

/// HTTP request method.
///
/// Methods are case-sensitive (RFC 9110, section 9.1): `get` is not `GET`,
/// so a lower-case name becomes a [`HttpMethod::Custom`] method.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum HttpMethod {
    /// `GET`
    Get,
    /// `POST`
    Post,
    /// `PUT`
    Put,
    /// `PATCH`
    Patch,
    /// `DELETE`
    Delete,
    /// `HEAD`
    Head,
    /// `OPTIONS`
    Options,
    /// Any other valid method token, e.g. `PROPFIND`.
    Custom(String),
}

impl HttpMethod {
    /// The method as it is sent on the wire.
    #[must_use]
    pub fn as_str(&self) -> &str {
        match self {
            Self::Get => "GET",
            Self::Post => "POST",
            Self::Put => "PUT",
            Self::Patch => "PATCH",
            Self::Delete => "DELETE",
            Self::Head => "HEAD",
            Self::Options => "OPTIONS",
            Self::Custom(name) => name,
        }
    }
}

impl fmt::Display for HttpMethod {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// A method name that is not a valid HTTP token.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("invalid HTTP method {0:?}: must be a non-empty token")]
pub struct InvalidMethod(pub String);

impl FromStr for HttpMethod {
    type Err = InvalidMethod;

    fn from_str(name: &str) -> Result<Self, Self::Err> {
        Ok(match name {
            "GET" => Self::Get,
            "POST" => Self::Post,
            "PUT" => Self::Put,
            "PATCH" => Self::Patch,
            "DELETE" => Self::Delete,
            "HEAD" => Self::Head,
            "OPTIONS" => Self::Options,
            _ if is_token(name) => Self::Custom(name.to_owned()),
            _ => return Err(InvalidMethod(name.to_owned())),
        })
    }
}

/// `token = 1*tchar` (RFC 9110, section 5.6.2).
fn is_token(text: &str) -> bool {
    const SPECIAL: &[u8] = b"!#$%&'*+-.^_`|~";
    !text.is_empty()
        && text
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || SPECIAL.contains(&b))
}

#[cfg(test)]
#[path = "tests/protocol.rs"]
mod tests;

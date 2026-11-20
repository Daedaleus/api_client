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

/// HTTP status code (`100`–`999`, as accepted on the wire).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct StatusCode(u16);

/// A number outside `100`–`999`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("invalid HTTP status code {0}: must be 100–999")]
pub struct InvalidStatus(pub u16);

/// The class of a status code, given by its first digit (RFC 9110, section 15).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum StatusClass {
    /// `1xx`
    Informational,
    /// `2xx`
    Success,
    /// `3xx`
    Redirection,
    /// `4xx`
    ClientError,
    /// `5xx`
    ServerError,
    /// `6xx`–`9xx`: not defined by the standard.
    Nonstandard,
}

impl StatusCode {
    /// Creates a status code; fails outside `100`–`999`.
    ///
    /// # Errors
    /// [`InvalidStatus`] if the number has not exactly three digits.
    pub fn new(code: u16) -> Result<Self, InvalidStatus> {
        if (100..=999).contains(&code) {
            Ok(Self(code))
        } else {
            Err(InvalidStatus(code))
        }
    }

    /// The numeric code.
    #[must_use]
    pub fn as_u16(self) -> u16 {
        self.0
    }

    /// The class of this code.
    #[must_use]
    pub fn class(self) -> StatusClass {
        match self.0 / 100 {
            1 => StatusClass::Informational,
            2 => StatusClass::Success,
            3 => StatusClass::Redirection,
            4 => StatusClass::ClientError,
            5 => StatusClass::ServerError,
            _ => StatusClass::Nonstandard,
        }
    }

    /// The standard reason phrase for common codes, e.g. `Not Found` for 404.
    #[must_use]
    pub fn canonical_reason(self) -> Option<&'static str> {
        Some(match self.0 {
            100 => "Continue",
            101 => "Switching Protocols",
            200 => "OK",
            201 => "Created",
            202 => "Accepted",
            204 => "No Content",
            206 => "Partial Content",
            301 => "Moved Permanently",
            302 => "Found",
            303 => "See Other",
            304 => "Not Modified",
            307 => "Temporary Redirect",
            308 => "Permanent Redirect",
            400 => "Bad Request",
            401 => "Unauthorized",
            403 => "Forbidden",
            404 => "Not Found",
            405 => "Method Not Allowed",
            406 => "Not Acceptable",
            408 => "Request Timeout",
            409 => "Conflict",
            410 => "Gone",
            413 => "Content Too Large",
            415 => "Unsupported Media Type",
            422 => "Unprocessable Content",
            429 => "Too Many Requests",
            500 => "Internal Server Error",
            501 => "Not Implemented",
            502 => "Bad Gateway",
            503 => "Service Unavailable",
            504 => "Gateway Timeout",
            _ => return None,
        })
    }
}

impl fmt::Display for StatusCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Header names used by the program itself (lower case, compare
/// case-insensitively).
pub mod header {
    /// `Content-Type`
    pub const CONTENT_TYPE: &str = "content-type";
}

#[cfg(test)]
#[path = "tests/protocol.rs"]
mod tests;

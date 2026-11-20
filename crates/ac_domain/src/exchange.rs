//! One HTTP exchange: the request as it is sent and the response as received.

use std::time::Duration;

use url::Url;

use crate::protocol::{HttpMethod, StatusCode};

/// One header line. Names keep the spelling they were sent or received with;
/// compare them case-insensitively.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Header {
    /// Header name, e.g. `Content-Type`.
    pub name: String,
    /// Header value.
    pub value: String,
}

impl Header {
    /// Creates a header.
    #[must_use]
    pub fn new(name: impl Into<String>, value: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            value: value.into(),
        }
    }
}

/// A request with every variable resolved, ready to be sent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedRequest {
    /// Request method.
    pub method: HttpMethod,
    /// Absolute URL.
    pub url: Url,
    /// Headers in the order they are sent.
    pub headers: Vec<Header>,
    /// Body bytes, if the request has a body.
    pub body: Option<Vec<u8>>,
}

/// A response as received.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Response {
    /// Status code.
    pub status: StatusCode,
    /// Headers in the order they were received.
    pub headers: Vec<Header>,
    /// Raw body.
    pub body: Vec<u8>,
    /// Time from sending the request until the whole body was received.
    pub elapsed: Duration,
}

impl Response {
    /// Size of the body in bytes.
    #[must_use]
    pub fn size(&self) -> usize {
        self.body.len()
    }

    /// Value of the first header with this name (case-insensitive).
    #[must_use]
    pub fn header(&self, name: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|header| header.name.eq_ignore_ascii_case(name))
            .map(|header| header.value.as_str())
    }
}

#[cfg(test)]
#[path = "tests/exchange.rs"]
mod tests;

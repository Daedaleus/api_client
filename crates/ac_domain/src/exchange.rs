//! One HTTP exchange: the request as it is sent and the response as received.

use std::path::PathBuf;
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
    /// Body, if the request has one.
    pub body: Option<RequestBody>,
}

impl ResolvedRequest {
    /// The body if it is given as bytes (not files).
    #[must_use]
    pub fn body_bytes(&self) -> Option<&[u8]> {
        match &self.body {
            Some(RequestBody::Bytes(bytes)) => Some(bytes),
            _ => None,
        }
    }
}

/// The body of a resolved request. Files are named by path and read only
/// when the request is sent (by the HTTP adapter), so the application layer
/// never touches the file system and large files need not be copied around.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RequestBody {
    /// These bytes.
    Bytes(Vec<u8>),
    /// The contents of this file.
    File(PathBuf),
    /// Multipart form data; the adapter chooses the boundary and sets the
    /// `Content-Type` with it.
    Multipart(Vec<Part>),
}

/// One part of a multipart body.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Part {
    /// Field name.
    pub name: String,
    /// Its content.
    pub content: PartContent,
}

/// The content of a multipart [`Part`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PartContent {
    /// Text.
    Text(String),
    /// The contents of this file (sent with its file name).
    File(PathBuf),
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

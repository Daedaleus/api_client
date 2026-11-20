//! Sending HTTP requests.

use std::future::Future;
use std::sync::Arc;

use ac_domain::exchange::{ResolvedRequest, Response};

use crate::text::{UserText, keys};

/// Sends HTTP requests.
pub trait HttpSender: Send + Sync + 'static {
    /// Sends the request and reads the whole response.
    ///
    /// A response with an error status (4xx, 5xx) is a normal `Ok`; only
    /// failures to get a response at all are errors.
    fn send(
        &self,
        request: ResolvedRequest,
    ) -> impl Future<Output = Result<Response, TransportError>> + Send;
}

impl<T: HttpSender> HttpSender for Arc<T> {
    fn send(
        &self,
        request: ResolvedRequest,
    ) -> impl Future<Output = Result<Response, TransportError>> + Send {
        T::send(self, request)
    }
}

/// Why no response arrived.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum TransportError {
    /// The server did not answer in time.
    #[error("timed out")]
    Timeout,
    /// No connection could be established (DNS, refused, TLS …).
    #[error("connection failed: {0}")]
    Connect(String),
    /// Any other failure while sending or receiving.
    #[error("request failed: {0}")]
    Other(String),
}

impl TransportError {
    /// The text to show the user.
    #[must_use]
    pub fn user_text(&self) -> UserText {
        match self {
            Self::Timeout => UserText::new(keys::TIMEOUT),
            Self::Connect(detail) => UserText::new(keys::CONNECT).with("detail", detail.clone()),
            Self::Other(detail) => UserText::new(keys::REQUEST).with("detail", detail.clone()),
        }
    }
}

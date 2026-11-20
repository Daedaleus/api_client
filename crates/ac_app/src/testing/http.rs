//! Test double for sending requests.

use std::collections::VecDeque;
use std::sync::Mutex;
use std::time::Duration;

use ac_domain::exchange::{Header, ResolvedRequest, Response};
use ac_domain::protocol::StatusCode;

use super::lock;
use crate::ports::{HttpSender, TransportError};

/// An [`HttpSender`] that answers with queued replies and records what was
/// sent. Without a queued reply it fails with [`TransportError::Other`].
#[derive(Debug, Default)]
pub struct FakeHttpSender {
    replies: Mutex<VecDeque<Result<Response, TransportError>>>,
    sent: Mutex<Vec<ResolvedRequest>>,
}

impl FakeHttpSender {
    /// A sender without replies.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Queues the next reply.
    #[must_use]
    pub fn reply(self, reply: Result<Response, TransportError>) -> Self {
        lock(&self.replies).push_back(reply);
        self
    }

    /// All requests sent so far.
    #[must_use]
    pub fn sent(&self) -> Vec<ResolvedRequest> {
        lock(&self.sent).clone()
    }
}

impl HttpSender for FakeHttpSender {
    fn send(
        &self,
        request: ResolvedRequest,
    ) -> impl Future<Output = Result<Response, TransportError>> + Send {
        lock(&self.sent).push(request);
        let reply = lock(&self.replies)
            .pop_front()
            .unwrap_or_else(|| Err(TransportError::Other("no reply queued".into())));
        std::future::ready(reply)
    }
}

/// A response with status, `Content-Type` and body; 7 ms elapsed.
///
/// # Panics
/// If `status` is not a three-digit code.
#[must_use]
pub fn response(status: u16, content_type: &str, body: &str) -> Response {
    Response {
        status: StatusCode::new(status).expect("three-digit status"),
        headers: vec![Header::new("Content-Type", content_type)],
        body: body.as_bytes().to_vec(),
        elapsed: Duration::from_millis(7),
    }
}

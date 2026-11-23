//! Test double for sending requests.

use std::collections::VecDeque;
use std::sync::Mutex;
use std::time::Duration;

use ac_domain::cookie::UnixSeconds;
use ac_domain::exchange::{Header, ResolvedRequest, Response};
use ac_domain::protocol::StatusCode;

use super::lock;
use crate::ports::{HttpSender, SettingsError, SharedCookieJar, TransportError};

/// The time the fake handles cookies at: 2026-01-01 00:00:00 UTC.
pub const FAKE_NOW: UnixSeconds = 1_767_225_600;

/// `Set-Cookie`, compared without case.
const SET_COOKIE: &str = "set-cookie";
use crate::settings::{NetworkSettings, ProxySetting};

/// An [`HttpSender`] that answers with queued replies and records what was
/// sent. Without a queued reply it fails with [`TransportError::Other`].
#[derive(Debug, Default)]
pub struct FakeHttpSender {
    replies: Mutex<VecDeque<Result<Response, TransportError>>>,
    sent: Mutex<Vec<ResolvedRequest>>,
    cookies_sent: Mutex<Vec<Option<String>>>,
    network: Mutex<Option<NetworkSettings>>,
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

    /// Per request sent: the `Cookie` header its jar gave (`None` without
    /// a jar or matching cookies).
    #[must_use]
    pub fn cookies_sent(&self) -> Vec<Option<String>> {
        lock(&self.cookies_sent).clone()
    }

    /// The network settings applied last, if any.
    #[must_use]
    pub fn network(&self) -> Option<NetworkSettings> {
        lock(&self.network).clone()
    }
}

impl HttpSender for FakeHttpSender {
    /// Like the real sender, sends the jar's cookies and keeps the ones the
    /// reply sets (at [`FAKE_NOW`]; no redirects).
    fn send(
        &self,
        request: ResolvedRequest,
        cookies: Option<SharedCookieJar>,
    ) -> impl Future<Output = Result<Response, TransportError>> + Send {
        let header = cookies
            .as_ref()
            .and_then(|jar| lock(jar).header_for(&request.url, FAKE_NOW));
        lock(&self.cookies_sent).push(header);
        let url = request.url.clone();
        lock(&self.sent).push(request);
        let reply = lock(&self.replies)
            .pop_front()
            .unwrap_or_else(|| Err(TransportError::Other("no reply queued".into())));
        if let (Some(jar), Ok(response)) = (&cookies, &reply) {
            let mut jar = lock(jar);
            for header in &response.headers {
                if header.name.eq_ignore_ascii_case(SET_COOKIE) {
                    jar.store(&url, &header.value, FAKE_NOW);
                }
            }
        }
        std::future::ready(reply)
    }

    /// Refuses a manual proxy whose URL is not `http(s)://` or `socks5://`
    /// (like the real sender would), else remembers the settings.
    fn configure(&self, network: &NetworkSettings) -> Result<(), SettingsError> {
        if let ProxySetting::Manual { url, .. } = &network.proxy
            && !["http://", "https://", "socks5://"]
                .iter()
                .any(|scheme| url.starts_with(scheme))
        {
            return Err(SettingsError(format!("invalid proxy URL {url}")));
        }
        *lock(&self.network) = Some(network.clone());
        Ok(())
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

//! Sending HTTP requests.

use std::future::Future;
use std::sync::{Arc, Mutex};

use ac_domain::cookie::CookieJar;
use ac_domain::exchange::{ResolvedRequest, Response};

use crate::ports::SettingsError;
use crate::settings::NetworkSettings;
use crate::text::{UserText, keys};

/// The cookies of one collection, shared between the requests sent for it.
pub type SharedCookieJar = Arc<Mutex<CookieJar>>;

/// Sends HTTP requests.
pub trait HttpSender: Send + Sync + 'static {
    /// Sends the request and reads the whole response.
    ///
    /// With a `cookies` jar, its matching cookies are sent and the cookies
    /// the server sets are kept in it – also on every redirect followed. A
    /// `Cookie` header of the request itself replaces the jar's cookies.
    ///
    /// A response with an error status (4xx, 5xx) is a normal `Ok`; only
    /// failures to get a response at all are errors.
    fn send(
        &self,
        request: ResolvedRequest,
        cookies: Option<SharedCookieJar>,
    ) -> impl Future<Output = Result<Response, TransportError>> + Send;

    /// Applies the app's network settings (proxy, extra CA certificates) to
    /// all later requests. Nothing changes if they cannot be applied.
    ///
    /// # Errors
    /// [`SettingsError`] if the proxy URL is invalid or a certificate file
    /// cannot be read or parsed.
    fn configure(&self, network: &NetworkSettings) -> Result<(), SettingsError>;
}

impl<T: HttpSender> HttpSender for Arc<T> {
    fn send(
        &self,
        request: ResolvedRequest,
        cookies: Option<SharedCookieJar>,
    ) -> impl Future<Output = Result<Response, TransportError>> + Send {
        T::send(self, request, cookies)
    }

    fn configure(&self, network: &NetworkSettings) -> Result<(), SettingsError> {
        T::configure(self, network)
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
    /// A file of the body cannot be read.
    #[error("cannot read {path}: {detail}")]
    File {
        /// The file.
        path: String,
        /// Why (from the operating system).
        detail: String,
    },
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
            Self::File { path, detail } => UserText::new(keys::FILE_UNREADABLE)
                .with("path", path.clone())
                .with("detail", detail.clone()),
            Self::Other(detail) => UserText::new(keys::REQUEST).with("detail", detail.clone()),
        }
    }
}

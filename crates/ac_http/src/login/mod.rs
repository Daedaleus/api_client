//! Signing in in the browser (OAuth 2 for native apps, RFC 8252): a tiny
//! HTTP listener on the loopback address receives the redirect that ends
//! the sign-in.
//!
//! - this file: [`LoopbackLogin`], the [`BrowserLogin`] adapter.
//! - `browser`: starting the user's browser ([`SystemBrowser`]).
//! - `callback`: reading the redirect and answering it.

mod browser;
mod callback;

pub use browser::{BrowserOpener, SystemBrowser};

use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::sync::Arc;
use std::time::Duration;

use ac_app::{BrowserLogin, LoginError, PendingLogin};
use tokio::net::TcpListener;
use url::{Host, Url};

/// Time the user has to sign in.
pub const SIGN_IN_TIMEOUT: Duration = Duration::from_mins(5);

/// [`BrowserLogin`] that listens on the loopback address and opens the
/// sign-in page with a [`BrowserOpener`].
#[derive(Debug, Clone)]
pub struct LoopbackLogin<O = SystemBrowser> {
    opener: Arc<O>,
    timeout: Duration,
}

impl LoopbackLogin {
    /// Opens the user's browser; [`SIGN_IN_TIMEOUT`] to sign in.
    #[must_use]
    pub fn new() -> Self {
        Self::with_opener(SystemBrowser, SIGN_IN_TIMEOUT)
    }
}

impl Default for LoopbackLogin {
    fn default() -> Self {
        Self::new()
    }
}

impl<O: BrowserOpener> LoopbackLogin<O> {
    /// Opens sign-in pages with `opener`; `timeout` to sign in.
    #[must_use]
    pub fn with_opener(opener: O, timeout: Duration) -> Self {
        Self {
            opener: Arc::new(opener),
            timeout,
        }
    }
}

impl<O: BrowserOpener> BrowserLogin for LoopbackLogin<O> {
    type Pending = LoopbackPending<O>;

    async fn listen(&self, redirect: &Url) -> Result<LoopbackPending<O>, LoginError> {
        let address = loopback_address(redirect)?;
        let listener = TcpListener::bind(address)
            .await
            .map_err(|error| LoginError::Listen {
                address: address.to_string(),
                detail: error.to_string(),
            })?;
        let port = listener
            .local_addr()
            .map_err(|error| LoginError::Redirect(error.to_string()))?
            .port();
        let mut redirect_uri = redirect.clone();
        // Cannot fail: the URL is http with a host.
        let _ = redirect_uri.set_port(Some(port));
        Ok(LoopbackPending {
            listener,
            redirect_uri,
            opener: Arc::clone(&self.opener),
            timeout: self.timeout,
        })
    }
}

/// Where to listen for `redirect`: its loopback address and port (0 for
/// any free one).
fn loopback_address(redirect: &Url) -> Result<SocketAddr, LoginError> {
    let ip = match redirect.host() {
        Some(Host::Domain(domain)) if domain.eq_ignore_ascii_case("localhost") => {
            IpAddr::V4(Ipv4Addr::LOCALHOST)
        }
        Some(Host::Ipv4(address)) if address.is_loopback() => IpAddr::V4(address),
        Some(Host::Ipv6(address)) if address.is_loopback() => IpAddr::V6(address),
        _ => {
            return Err(LoginError::Listen {
                address: redirect.to_string(),
                detail: "not a loopback address".into(),
            });
        }
    };
    let port = redirect.port_or_known_default().unwrap_or(0);
    Ok(SocketAddr::new(ip, port))
}

/// A sign-in waiting for the redirect.
#[derive(Debug)]
pub struct LoopbackPending<O> {
    listener: TcpListener,
    redirect_uri: Url,
    opener: Arc<O>,
    timeout: Duration,
}

impl<O: BrowserOpener> PendingLogin for LoopbackPending<O> {
    fn redirect_uri(&self) -> &Url {
        &self.redirect_uri
    }

    async fn complete(self, url: Url) -> Result<Vec<(String, String)>, LoginError> {
        self.opener.open(&url).map_err(LoginError::Browser)?;
        let path = self.redirect_uri.path().to_owned();
        tokio::time::timeout(self.timeout, callback::receive(&self.listener, &path))
            .await
            .map_err(|_| LoginError::TimedOut)?
    }
}

#[cfg(test)]
#[path = "../tests/login.rs"]
mod tests;

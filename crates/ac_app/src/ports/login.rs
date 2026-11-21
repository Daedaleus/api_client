//! Signing in through the user's browser (OAuth 2 authorization code for
//! native apps, RFC 8252): the app listens on a loopback address, opens the
//! sign-in page in the browser and waits for the redirect back.

use std::future::Future;
use std::sync::Arc;

use url::Url;

use crate::text::{UserText, keys};

/// Opens sign-in pages and receives the redirect back.
pub trait BrowserLogin: Send + Sync + 'static {
    /// A sign-in that waits for the browser.
    type Pending: PendingLogin;

    /// Starts listening for the redirect to `redirect`, a loopback URL
    /// (port 0: any free port).
    fn listen(
        &self,
        redirect: &Url,
    ) -> impl Future<Output = Result<Self::Pending, LoginError>> + Send;
}

impl<T: BrowserLogin> BrowserLogin for Arc<T> {
    type Pending = T::Pending;

    fn listen(
        &self,
        redirect: &Url,
    ) -> impl Future<Output = Result<Self::Pending, LoginError>> + Send {
        T::listen(self, redirect)
    }
}

/// A sign-in in progress.
pub trait PendingLogin: Send + 'static {
    /// The redirect URI to give the authorization server (with the port
    /// actually listened on).
    fn redirect_uri(&self) -> &Url;

    /// Opens `url` in the browser and waits for the redirect; returns its
    /// query parameters.
    fn complete(
        self,
        url: Url,
    ) -> impl Future<Output = Result<Vec<(String, String)>, LoginError>> + Send;
}

/// Why signing in in the browser failed.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum LoginError {
    /// The redirect address cannot be listened on (in use, say).
    #[error("cannot listen on {address}: {detail}")]
    Listen {
        /// The address.
        address: String,
        /// Why (from the operating system).
        detail: String,
    },
    /// The browser cannot be started.
    #[error("cannot open the browser: {0}")]
    Browser(String),
    /// Nobody signed in in time.
    #[error("no sign-in in time")]
    TimedOut,
    /// Receiving the redirect failed.
    #[error("redirect failed: {0}")]
    Redirect(String),
}

impl LoginError {
    /// The text to show the user.
    #[must_use]
    pub fn user_text(&self) -> UserText {
        match self {
            Self::TimedOut => UserText::new(keys::OAUTH_SIGN_IN_TIMEOUT),
            other => UserText::new(keys::OAUTH_TOKEN).with("detail", other.to_string()),
        }
    }
}

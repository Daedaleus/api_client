//! OAuth 2: the access token is obtained just before a request is sent and
//! kept in memory until shortly before it expires – never on disk, so no
//! token ends up in a collection folder or in git.
//!
//! - this file: the resolved settings ([`OAuthClient`]), errors and adding
//!   the token to a request.
//! - `token`: token requests and responses (RFC 6749, section 5).
//! - `pkce`: the sign-in URL secured with PKCE (RFC 7636).
//! - `cache`: obtaining tokens (client credentials, sign-in in the browser,
//!   refresh) and keeping them.

mod cache;
mod pkce;
mod token;

pub(crate) use cache::TokenCache;

use ac_domain::auth::{OAuth2, OAuth2Grant, is_loopback_redirect};
use ac_domain::environment::UnresolvedVariable;
use ac_domain::exchange::{Header, ResolvedRequest};
use ac_domain::protocol::{auth_scheme, header};
use url::Url;

use crate::error::AppError;
use crate::ports::{LoginError, TransportError};
use crate::resolve::resolve_url;
use crate::text::{UserText, keys};

/// Stands in for the token in exported cURL commands: the token is fetched
/// only when sending.
pub(crate) const TOKEN_PLACEHOLDER: &str = "<access token>";

/// Where the browser returns to when no redirect URL is set: any free port
/// on the loopback address, as RFC 8252 (section 7.3) allows.
const DEFAULT_REDIRECT: &str = "http://127.0.0.1:0/callback";

/// The OAuth 2 settings of a request, variables filled in. Also the key of
/// the token cache: other settings, other token.
#[derive(Clone, PartialEq, Eq, Hash)]
pub(crate) struct OAuthClient {
    pub flow: Flow,
    pub token_url: Url,
    pub client_id: String,
    pub client_secret: String,
    pub scope: String,
}

/// How the token is obtained.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) enum Flow {
    /// Client credentials: no user involved.
    ClientCredentials,
    /// The user signs in in the browser, which returns to `redirect`.
    AuthorizationCode { auth_url: Url, redirect: Url },
}

/// The secret is never shown in debug output.
impl std::fmt::Debug for OAuthClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OAuthClient")
            .field("flow", &self.flow)
            .field("token_url", &self.token_url.as_str())
            .field("client_id", &self.client_id)
            .field("scope", &self.scope)
            .finish_non_exhaustive()
    }
}

impl OAuthClient {
    /// `oauth` with every `{{name}}` filled in and the URLs checked.
    pub(crate) fn resolve(
        oauth: &OAuth2,
        fill: &impl Fn(&str) -> Result<String, UnresolvedVariable>,
    ) -> Result<Self, AppError> {
        let flow = match oauth.grant {
            OAuth2Grant::ClientCredentials => Flow::ClientCredentials,
            OAuth2Grant::AuthorizationCode => Flow::AuthorizationCode {
                auth_url: resolve_url(&fill(&oauth.auth_url)?)?,
                redirect: loopback_redirect(&fill(&oauth.redirect_url)?)?,
            },
        };
        Ok(Self {
            flow,
            token_url: resolve_url(&fill(&oauth.token_url)?)?,
            client_id: fill(&oauth.client_id)?.trim().to_owned(),
            client_secret: fill(&oauth.client_secret)?,
            scope: fill(&oauth.scope)?.trim().to_owned(),
        })
    }

    /// Whether getting a token needs the user (in the browser).
    fn is_interactive(&self) -> bool {
        matches!(self.flow, Flow::AuthorizationCode { .. })
    }
}

/// The redirect URL, checked with [`is_loopback_redirect`].
fn loopback_redirect(text: &str) -> Result<Url, AppError> {
    let text = text.trim();
    let text = if text.is_empty() {
        DEFAULT_REDIRECT
    } else {
        text
    };
    let invalid = || AppError::RedirectNotLoopback(text.to_owned());
    let url = Url::parse(text).map_err(|_| invalid())?;
    if is_loopback_redirect(&url) {
        Ok(url)
    } else {
        Err(invalid())
    }
}

/// Why no access token could be obtained.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub(crate) enum TokenError {
    /// The token URL did not answer.
    #[error("{0}")]
    Transport(TransportError),
    /// The server refused, e.g. with `invalid_client`.
    #[error("status {status}: {reason}")]
    Refused {
        /// The status of the answer.
        status: u16,
        /// `error` and `error_description` of the answer, or its start.
        reason: String,
    },
    /// The answer holds no access token.
    #[error("no access_token in the answer")]
    NoToken,
    /// Signing in in the browser failed.
    #[error(transparent)]
    Login(LoginError),
    /// The sign-in was refused or cancelled (`error` of the redirect).
    #[error("sign-in refused: {0}")]
    Denied(String),
    /// The redirect does not belong to this sign-in (wrong `state`).
    #[error("the redirect does not belong to this sign-in")]
    StateMismatch,
    /// The redirect has no `code`.
    #[error("no code in the redirect")]
    NoCode,
}

impl TokenError {
    /// The text to show the user.
    pub(crate) fn user_text(&self) -> UserText {
        match self {
            Self::Login(error) => error.user_text(),
            other => UserText::new(keys::OAUTH_TOKEN).with("detail", other.to_string()),
        }
    }
}

/// Adds the token as bearer authorization, unless the request already has
/// an `Authorization` header (what the user typed wins).
pub(crate) fn authorize(request: &mut ResolvedRequest, token: &str) {
    let present = request
        .headers
        .iter()
        .any(|existing| existing.name.eq_ignore_ascii_case(header::AUTHORIZATION));
    if !present {
        request.headers.push(Header::new(
            header::AUTHORIZATION,
            format!("{} {token}", auth_scheme::BEARER),
        ));
    }
}

#[cfg(test)]
#[path = "../tests/oauth.rs"]
mod tests;

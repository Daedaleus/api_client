//! Obtaining access tokens and keeping them until they expire.

use std::collections::HashMap;
use std::sync::{Mutex, MutexGuard, PoisonError};
use std::time::{Duration, Instant};

use url::Url;

use super::pkce::SignIn;
use super::token::{TokenGrant, TokenResponse, parse, token_request};
use super::{Flow, OAuthClient, TokenError};
use crate::ports::{BrowserLogin, HttpSender, PendingLogin};

/// A token is obtained anew this long before it expires, so it does not
/// run out on the way to the server.
const EXPIRY_MARGIN: Duration = Duration::from_secs(30);

/// Query parameters of the redirect after signing in (RFC 6749, 4.1.2).
mod param {
    pub const CODE: &str = "code";
    pub const STATE: &str = "state";
    pub const ERROR: &str = "error";
    pub const ERROR_DESCRIPTION: &str = "error_description";
}

/// Access tokens obtained so far, until they expire.
#[derive(Debug, Default)]
pub(crate) struct TokenCache {
    tokens: Mutex<HashMap<OAuthClient, CachedToken>>,
    /// One sign-in in the browser at a time: requests sent meanwhile wait
    /// for it and then use its token.
    sign_in: tokio::sync::Mutex<()>,
}

#[derive(Debug, Default)]
struct CachedToken {
    /// The access token, if it may be used again.
    access_token: Option<String>,
    /// When to stop using it; `None` if the server gave no lifetime.
    renew_at: Option<Instant>,
    refresh_token: Option<String>,
}

impl CachedToken {
    fn valid(&self, now: Instant) -> Option<&str> {
        self.access_token
            .as_deref()
            .filter(|_| self.renew_at.is_none_or(|renew_at| renew_at > now))
    }
}

impl TokenCache {
    /// A valid access token for `client`: from the cache, refreshed, or
    /// obtained anew (for the authorization code by signing in in the
    /// browser).
    pub(crate) async fn token(
        &self,
        http: &impl HttpSender,
        login: &impl BrowserLogin,
        client: &OAuthClient,
        verify_tls: bool,
    ) -> Result<String, TokenError> {
        if let Some(token) = self.valid(client) {
            return Ok(token);
        }
        // The std lock is never held across an await; this one is.
        let _signing_in = if client.is_interactive() {
            let guard = self.sign_in.lock().await;
            // Another request may have signed in while this one waited.
            if let Some(token) = self.valid(client) {
                return Ok(token);
            }
            Some(guard)
        } else {
            None
        };
        let refresh_token = self
            .lock()
            .get(client)
            .and_then(|cached| cached.refresh_token.clone());
        if let Some(refresh_token) = refresh_token {
            let grant = TokenGrant::Refresh {
                refresh_token: &refresh_token,
            };
            match request(http, client, &grant, verify_tls).await {
                Ok(response) => return Ok(self.keep(client, response, Some(refresh_token))),
                // Expired or revoked: obtain a new one below.
                Err(_) => {
                    self.lock().remove(client);
                }
            }
        }
        let response = match &client.flow {
            Flow::ClientCredentials => {
                request(http, client, &TokenGrant::ClientCredentials, verify_tls).await?
            }
            Flow::AuthorizationCode { auth_url, redirect } => {
                sign_in(http, login, client, auth_url, redirect, verify_tls).await?
            }
        };
        Ok(self.keep(client, response, None))
    }

    fn valid(&self, client: &OAuthClient) -> Option<String> {
        self.lock()
            .get(client)
            .and_then(|cached| cached.valid(Instant::now()))
            .map(str::to_owned)
    }

    /// Keeps what may be used again and returns the access token. Without a
    /// lifetime a client credentials token is used once (getting another
    /// is cheap), a signed-in one until the app ends (getting another needs
    /// the user).
    fn keep(
        &self,
        client: &OAuthClient,
        response: TokenResponse,
        previous_refresh_token: Option<String>,
    ) -> String {
        let now = Instant::now();
        let reusable = match response.expires_in {
            Some(lifetime) => lifetime
                .checked_sub(EXPIRY_MARGIN)
                .filter(|valid| !valid.is_zero())
                .map(|valid| Some(now + valid)),
            None if client.is_interactive() => Some(None),
            None => None,
        };
        let cached = CachedToken {
            access_token: reusable.map(|_| response.access_token.clone()),
            renew_at: reusable.flatten(),
            // The server may keep the refresh token as it was.
            refresh_token: response.refresh_token.or(previous_refresh_token),
        };
        let mut tokens = self.lock();
        if cached.access_token.is_none() && cached.refresh_token.is_none() {
            tokens.remove(client);
        } else {
            tokens.insert(client.clone(), cached);
        }
        response.access_token
    }

    fn lock(&self) -> MutexGuard<'_, HashMap<OAuthClient, CachedToken>> {
        // The map stays consistent: entries are inserted or removed whole.
        self.tokens.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

/// Sends a token request and reads the answer.
async fn request(
    http: &impl HttpSender,
    client: &OAuthClient,
    grant: &TokenGrant<'_>,
    verify_tls: bool,
) -> Result<TokenResponse, TokenError> {
    let response = http
        // Token requests carry no cookies of a collection.
        .send(token_request(client, grant, verify_tls), None)
        .await
        .map_err(TokenError::Transport)?;
    parse(&response)
}

/// Signs in in the browser and exchanges the code for a token.
async fn sign_in(
    http: &impl HttpSender,
    login: &impl BrowserLogin,
    client: &OAuthClient,
    auth_url: &Url,
    redirect: &Url,
    verify_tls: bool,
) -> Result<TokenResponse, TokenError> {
    let pending = login.listen(redirect).await.map_err(TokenError::Login)?;
    let redirect_uri = pending.redirect_uri().clone();
    let sign_in = SignIn::new(client, auth_url, &redirect_uri);
    let answer = pending
        .complete(sign_in.url)
        .await
        .map_err(TokenError::Login)?;
    let get = |name: &str| {
        answer
            .iter()
            .find(|(key, _)| key == name)
            .map(|(_, value)| value.as_str())
    };
    if get(param::STATE) != Some(sign_in.state.as_str()) {
        return Err(TokenError::StateMismatch);
    }
    if let Some(error) = get(param::ERROR) {
        return Err(TokenError::Denied(match get(param::ERROR_DESCRIPTION) {
            Some(description) => format!("{error} ({description})"),
            None => error.to_owned(),
        }));
    }
    let code = get(param::CODE).ok_or(TokenError::NoCode)?;
    let grant = TokenGrant::AuthorizationCode {
        code,
        redirect_uri: &redirect_uri,
        verifier: &sign_in.verifier,
    };
    request(http, client, &grant, verify_tls).await
}

#[cfg(test)]
#[path = "../tests/oauth_cache.rs"]
mod tests;

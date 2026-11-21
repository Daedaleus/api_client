//! OAuth 2 client credentials grant (RFC 6749, section 4.4): the access
//! token is fetched from the token URL just before a request is sent and
//! kept in memory until shortly before it expires – never on disk, so no
//! token ends up in a collection folder or in git.

use std::collections::HashMap;
use std::sync::{Mutex, PoisonError};
use std::time::{Duration, Instant};

use ac_domain::auth::OAuth2;
use ac_domain::environment::UnresolvedVariable;
use ac_domain::exchange::{Header, RequestBody, ResolvedRequest, Response};
use ac_domain::protocol::{HttpMethod, StatusClass, auth_scheme, grant_type, header, media_type};
use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64;
use url::Url;
use url::form_urlencoded::{Serializer, byte_serialize};

use crate::error::AppError;
use crate::ports::{HttpSender, TransportError};
use crate::resolve::resolve_url;
use crate::text::{UserText, keys};

/// A token is fetched anew this long before it expires, so it does not run
/// out on the way to the server.
const EXPIRY_MARGIN: Duration = Duration::from_secs(30);

/// Stands in for the token in exported cURL commands: the token is fetched
/// only when sending.
pub(crate) const TOKEN_PLACEHOLDER: &str = "<access token>";

/// Form fields and values of the token request and response (RFC 6749).
mod field {
    pub const GRANT_TYPE: &str = "grant_type";
    pub const SCOPE: &str = "scope";
    pub const ACCESS_TOKEN: &str = "access_token";
    pub const EXPIRES_IN: &str = "expires_in";
    pub const ERROR: &str = "error";
    pub const ERROR_DESCRIPTION: &str = "error_description";
}

/// The client credentials of a request, variables filled in. Also the key
/// of the token cache: other credentials, other token.
#[derive(Clone, PartialEq, Eq, Hash)]
pub(crate) struct ClientCredentials {
    pub token_url: Url,
    pub client_id: String,
    pub client_secret: String,
    pub scope: String,
}

/// The secret is never shown in debug output.
impl std::fmt::Debug for ClientCredentials {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ClientCredentials")
            .field("token_url", &self.token_url.as_str())
            .field("client_id", &self.client_id)
            .field("scope", &self.scope)
            .finish_non_exhaustive()
    }
}

impl ClientCredentials {
    /// `oauth` with every `{{name}}` filled in.
    pub(crate) fn resolve(
        oauth: &OAuth2,
        fill: &impl Fn(&str) -> Result<String, UnresolvedVariable>,
    ) -> Result<Self, AppError> {
        Ok(Self {
            token_url: resolve_url(&fill(&oauth.token_url)?)?,
            client_id: fill(&oauth.client_id)?.trim().to_owned(),
            client_secret: fill(&oauth.client_secret)?,
            scope: fill(&oauth.scope)?.trim().to_owned(),
        })
    }

    /// The token request: a form POST, the client authenticated with HTTP
    /// Basic (id and secret form-encoded first, RFC 6749 section 2.3.1).
    fn token_request(&self, verify_tls: bool) -> ResolvedRequest {
        let mut form = Serializer::new(String::new());
        form.append_pair(field::GRANT_TYPE, grant_type::CLIENT_CREDENTIALS);
        if !self.scope.is_empty() {
            form.append_pair(field::SCOPE, &self.scope);
        }
        let encode = |text: &str| byte_serialize(text.as_bytes()).collect::<String>();
        let credentials = format!(
            "{}:{}",
            encode(&self.client_id),
            encode(&self.client_secret)
        );
        ResolvedRequest {
            method: HttpMethod::Post,
            url: self.token_url.clone(),
            headers: vec![
                Header::new(
                    header::AUTHORIZATION,
                    format!("{} {}", auth_scheme::BASIC, BASE64.encode(credentials)),
                ),
                Header::new(header::CONTENT_TYPE, media_type::FORM),
                Header::new(header::ACCEPT, media_type::JSON),
            ],
            body: Some(RequestBody::Bytes(form.finish().into_bytes())),
            verify_tls,
        }
    }
}

/// Why no access token could be fetched.
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
}

impl TokenError {
    /// The text to show the user.
    pub(crate) fn user_text(&self) -> UserText {
        UserText::new(keys::OAUTH_TOKEN).with("detail", self.to_string())
    }
}

/// Access tokens fetched so far, until they expire.
#[derive(Debug, Default)]
pub(crate) struct TokenCache {
    tokens: Mutex<HashMap<ClientCredentials, CachedToken>>,
}

#[derive(Debug)]
struct CachedToken {
    value: String,
    renew_at: Instant,
}

impl TokenCache {
    /// A valid access token for `credentials`: from the cache, or fetched.
    pub(crate) async fn token(
        &self,
        http: &impl HttpSender,
        credentials: &ClientCredentials,
        verify_tls: bool,
    ) -> Result<String, TokenError> {
        let now = Instant::now();
        if let Some(cached) = self.lock().get(credentials)
            && cached.renew_at > now
        {
            return Ok(cached.value.clone());
        }
        // The lock is not held while fetching; two requests at once may
        // both fetch a token, which is harmless.
        let response = http
            .send(credentials.token_request(verify_tls))
            .await
            .map_err(TokenError::Transport)?;
        let (value, expires_in) = parse(&response)?;
        let mut tokens = self.lock();
        match expires_in.and_then(|lifetime| lifetime.checked_sub(EXPIRY_MARGIN)) {
            // Without a lifetime the token cannot be kept safely.
            None => {
                tokens.remove(credentials);
            }
            Some(valid) => {
                tokens.insert(
                    credentials.clone(),
                    CachedToken {
                        value: value.clone(),
                        renew_at: now + valid,
                    },
                );
            }
        }
        Ok(value)
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, HashMap<ClientCredentials, CachedToken>> {
        // The map stays consistent: entries are inserted or removed whole.
        self.tokens.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

/// Access token and lifetime from the token response.
fn parse(response: &Response) -> Result<(String, Option<Duration>), TokenError> {
    let json: Option<serde_json::Value> = serde_json::from_slice(&response.body).ok();
    let text = |key: &str| {
        json.as_ref()
            .and_then(|json| json.get(key))
            .and_then(serde_json::Value::as_str)
    };
    if response.status.class() != StatusClass::Success {
        let reason = match (text(field::ERROR), text(field::ERROR_DESCRIPTION)) {
            (Some(error), Some(description)) => format!("{error} ({description})"),
            (Some(error), None) => error.to_owned(),
            _ => String::from_utf8_lossy(&response.body)
                .chars()
                .take(200)
                .collect(),
        };
        return Err(TokenError::Refused {
            status: response.status.as_u16(),
            reason,
        });
    }
    let token = text(field::ACCESS_TOKEN)
        .filter(|token| !token.is_empty())
        .ok_or(TokenError::NoToken)?;
    // Some servers send the lifetime as a string.
    let expires_in = json
        .as_ref()
        .and_then(|json| json.get(field::EXPIRES_IN))
        .and_then(|value| value.as_u64().or_else(|| value.as_str()?.parse().ok()))
        .map(Duration::from_secs);
    Ok((token.to_owned(), expires_in))
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
#[path = "tests/oauth.rs"]
mod tests;

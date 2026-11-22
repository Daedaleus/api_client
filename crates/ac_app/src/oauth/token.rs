//! Token requests and their answers (RFC 6749, sections 4 to 6).

use std::time::Duration;

use ac_domain::exchange::{Header, RequestBody, ResolvedRequest, Response};
use ac_domain::protocol::{HttpMethod, StatusClass, auth_scheme, grant_type, header, media_type};
use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64;
use url::Url;
use url::form_urlencoded::{Serializer, byte_serialize};

use super::{OAuthClient, TokenError};

/// Form fields of token requests and responses (RFC 6749).
mod field {
    pub const GRANT_TYPE: &str = "grant_type";
    pub const SCOPE: &str = "scope";
    pub const CLIENT_ID: &str = "client_id";
    pub const CODE: &str = "code";
    pub const REDIRECT_URI: &str = "redirect_uri";
    pub const CODE_VERIFIER: &str = "code_verifier";
    pub const REFRESH_TOKEN: &str = "refresh_token";
    pub const ACCESS_TOKEN: &str = "access_token";
    pub const EXPIRES_IN: &str = "expires_in";
    pub const ERROR: &str = "error";
    pub const ERROR_DESCRIPTION: &str = "error_description";
}

/// What a token is requested with.
#[derive(Debug)]
pub(super) enum TokenGrant<'a> {
    /// The client's own credentials.
    ClientCredentials,
    /// The code of a sign-in, with the redirect URI and PKCE verifier used.
    AuthorizationCode {
        code: &'a str,
        redirect_uri: &'a Url,
        verifier: &'a str,
    },
    /// A refresh token from an earlier answer.
    Refresh { refresh_token: &'a str },
}

/// A successful token answer.
#[derive(Debug, PartialEq, Eq)]
pub(super) struct TokenResponse {
    pub access_token: String,
    /// How long the token is valid, if the server says.
    pub expires_in: Option<Duration>,
    pub refresh_token: Option<String>,
}

/// The token request: a form POST. A client with a secret authenticates
/// with HTTP Basic (id and secret form-encoded first, RFC 6749 section
/// 2.3.1); a public client only names its id.
pub(super) fn token_request(
    client: &OAuthClient,
    grant: &TokenGrant<'_>,
    verify_tls: bool,
) -> ResolvedRequest {
    let mut form = Serializer::new(String::new());
    match grant {
        TokenGrant::ClientCredentials => {
            form.append_pair(field::GRANT_TYPE, grant_type::CLIENT_CREDENTIALS);
        }
        TokenGrant::AuthorizationCode {
            code,
            redirect_uri,
            verifier,
        } => {
            form.append_pair(field::GRANT_TYPE, grant_type::AUTHORIZATION_CODE);
            form.append_pair(field::CODE, code);
            form.append_pair(field::REDIRECT_URI, redirect_uri.as_str());
            form.append_pair(field::CODE_VERIFIER, verifier);
        }
        TokenGrant::Refresh { refresh_token } => {
            form.append_pair(field::GRANT_TYPE, grant_type::REFRESH_TOKEN);
            form.append_pair(field::REFRESH_TOKEN, refresh_token);
        }
    }
    // The code already carries the scope it was granted for.
    if !client.scope.is_empty() && !matches!(grant, TokenGrant::AuthorizationCode { .. }) {
        form.append_pair(field::SCOPE, &client.scope);
    }
    let mut headers = vec![
        Header::new(header::CONTENT_TYPE, media_type::FORM),
        Header::new(header::ACCEPT, media_type::JSON),
    ];
    if client.client_secret.is_empty() {
        form.append_pair(field::CLIENT_ID, &client.client_id);
    } else {
        let encode = |text: &str| byte_serialize(text.as_bytes()).collect::<String>();
        let credentials = format!(
            "{}:{}",
            encode(&client.client_id),
            encode(&client.client_secret)
        );
        headers.insert(
            0,
            Header::new(
                header::AUTHORIZATION,
                format!("{} {}", auth_scheme::BASIC, BASE64.encode(credentials)),
            ),
        );
    }
    ResolvedRequest {
        transport: ac_domain::transport::EffectiveTransport::default(),
        method: HttpMethod::Post,
        url: client.token_url.clone(),
        headers,
        body: Some(RequestBody::Bytes(form.finish().into_bytes())),
        verify_tls,
    }
}

/// Access token, lifetime and refresh token from the token answer.
pub(super) fn parse(response: &Response) -> Result<TokenResponse, TokenError> {
    let json: Option<serde_json::Value> = serde_json::from_slice(&response.body).ok();
    let text = |key: &str| {
        json.as_ref()
            .and_then(|json| json.get(key))
            .and_then(serde_json::Value::as_str)
            .filter(|value| !value.is_empty())
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
    let access_token = text(field::ACCESS_TOKEN).ok_or(TokenError::NoToken)?;
    // Some servers send the lifetime as a string.
    let expires_in = json
        .as_ref()
        .and_then(|json| json.get(field::EXPIRES_IN))
        .and_then(|value| value.as_u64().or_else(|| value.as_str()?.parse().ok()))
        .map(Duration::from_secs);
    Ok(TokenResponse {
        access_token: access_token.to_owned(),
        expires_in,
        refresh_token: text(field::REFRESH_TOKEN).map(str::to_owned),
    })
}

#[cfg(test)]
#[path = "../tests/oauth_token.rs"]
mod tests;

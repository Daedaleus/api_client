//! Turns what the user entered into a [`ResolvedRequest`].

use std::collections::HashMap;
use std::path::Path;

use ac_domain::auth::{Auth, KeyPlacement};
use ac_domain::collection::{Body, HeaderEntry};
use ac_domain::environment::{UnresolvedVariable, interpolate};
use ac_domain::exchange::{Header, ResolvedRequest};
use ac_domain::protocol::{HttpMethod, auth_scheme, header, is_token};
use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64;
use url::Url;

use crate::error::AppError;
use crate::oauth::ClientCredentials;
use crate::resolve_body::resolve_body;

/// Scheme assumed when the user enters a URL without one, as other API
/// clients do.
const DEFAULT_SCHEME: &str = "http://";
const SUPPORTED_SCHEMES: [&str; 2] = ["http", "https"];

/// A request as the user entered it, with the authentication that applies.
pub(crate) struct Entered<'a> {
    pub method: &'a str,
    pub url: &'a str,
    pub headers: &'a [HeaderEntry],
    pub body: &'a Body,
    pub auth: &'a Auth,
    /// Where relative file paths of the body start (the collection folder).
    pub files_base: Option<&'a Path>,
    /// Check the server's TLS certificate.
    pub verify_tls: bool,
}

/// A request ready to send – once an OAuth 2 token, if any, is added.
#[derive(Debug)]
pub(crate) struct Prepared {
    pub request: ResolvedRequest,
    /// Fetch a token with these and send it as bearer authorization.
    pub client_credentials: Option<ClientCredentials>,
}

/// Validates method, URL and headers as entered and prepares the body.
///
/// Every `{{name}}` in the URL, header names and values, the body and the
/// authentication is replaced from `variables` first. Disabled headers and
/// rows without a name are left out. A JSON or text body gets a matching
/// `Content-Type` unless the request sets one itself; likewise the
/// authentication adds its header or query parameter unless the request
/// already has one of that name (what the user typed wins).
pub(crate) fn resolve_request(
    entered: &Entered<'_>,
    variables: &HashMap<&str, &str>,
) -> Result<Prepared, AppError> {
    let fill = |text: &str| interpolate(text, |name| variables.get(name).copied());
    let method: HttpMethod = entered.method.trim().parse()?;
    let mut url = resolve_url(&fill(entered.url)?)?;
    let headers: Vec<HeaderEntry> = entered
        .headers
        .iter()
        .filter(|entry| entry.enabled)
        .map(|entry| {
            Ok(HeaderEntry {
                name: fill(&entry.name)?,
                value: fill(&entry.value)?,
                enabled: true,
            })
        })
        .collect::<Result<_, UnresolvedVariable>>()?;
    let mut headers = resolve_headers(&headers)?;
    apply_auth(entered.auth, &mut url, &mut headers, &fill)?;
    let body = resolve_body(entered.body, &mut headers, entered.files_base, &fill)?;
    let client_credentials = match entered.auth {
        Auth::OAuth2(oauth) => Some(ClientCredentials::resolve(oauth, &fill)?),
        _ => None,
    };
    Ok(Prepared {
        request: ResolvedRequest {
            method,
            url,
            headers,
            body,
            verify_tls: entered.verify_tls,
        },
        client_credentials,
    })
}

pub(crate) fn resolve_url(url: &str) -> Result<Url, AppError> {
    let url = url.trim();
    if url.is_empty() {
        return Err(AppError::EmptyUrl);
    }
    let with_scheme = if url.contains("://") {
        url.to_owned()
    } else {
        format!("{DEFAULT_SCHEME}{url}")
    };
    let url = Url::parse(&with_scheme).map_err(|error| AppError::InvalidUrl(error.to_string()))?;
    if !SUPPORTED_SCHEMES.contains(&url.scheme()) {
        return Err(AppError::UnsupportedScheme(url.scheme().to_owned()));
    }
    Ok(url)
}

fn resolve_headers(entries: &[HeaderEntry]) -> Result<Vec<Header>, AppError> {
    entries
        .iter()
        .filter(|entry| entry.enabled && !entry.name.trim().is_empty())
        .map(|entry| checked_header(entry.name.trim(), &entry.value))
        .collect()
}

fn checked_header(name: &str, value: &str) -> Result<Header, AppError> {
    if is_token(name) && header::is_valid_value(value) {
        Ok(Header::new(name, value))
    } else {
        Err(AppError::InvalidHeader(name.to_owned()))
    }
}

/// Adds the header or query parameter of `auth`, unless the request already
/// has one of that name.
fn apply_auth(
    auth: &Auth,
    url: &mut Url,
    headers: &mut Vec<Header>,
    fill: &impl Fn(&str) -> Result<String, UnresolvedVariable>,
) -> Result<(), AppError> {
    let (name, value) = match auth {
        // The OAuth 2 token is fetched and added when sending.
        Auth::Inherit | Auth::None | Auth::OAuth2(_) => return Ok(()),
        Auth::Basic { username, password } => {
            let credentials = format!("{}:{}", fill(username)?, fill(password)?);
            let value = format!("{} {}", auth_scheme::BASIC, BASE64.encode(credentials));
            (header::AUTHORIZATION.to_owned(), value)
        }
        Auth::Bearer { token } => (
            header::AUTHORIZATION.to_owned(),
            format!("{} {}", auth_scheme::BEARER, fill(token)?),
        ),
        Auth::ApiKey {
            name,
            value,
            placement: KeyPlacement::Query,
        } => {
            let name = fill(name)?.trim().to_owned();
            if !url.query_pairs().any(|(key, _)| key == name) {
                url.query_pairs_mut().append_pair(&name, &fill(value)?);
            }
            return Ok(());
        }
        Auth::ApiKey {
            name,
            value,
            placement: KeyPlacement::Header,
        } => (fill(name)?.trim().to_owned(), fill(value)?),
    };
    let present = headers
        .iter()
        .any(|header| header.name.eq_ignore_ascii_case(&name));
    if !present {
        headers.push(checked_header(&name, &value)?);
    }
    Ok(())
}

#[cfg(test)]
#[path = "tests/resolve.rs"]
mod tests;

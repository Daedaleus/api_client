//! Turns what the user entered into a [`ResolvedRequest`].

use ac_domain::collection::{Body, HeaderEntry};
use ac_domain::exchange::{Header, ResolvedRequest};
use ac_domain::protocol::{HttpMethod, header, is_token, media_type};
use url::Url;

use crate::error::AppError;

/// Scheme assumed when the user enters a URL without one, as other API
/// clients do.
const DEFAULT_SCHEME: &str = "http://";
const SUPPORTED_SCHEMES: [&str; 2] = ["http", "https"];

/// Validates method, URL and headers as entered and prepares the body.
///
/// Disabled headers and rows without a name are left out. A JSON or text
/// body gets a matching `Content-Type` unless the request sets one itself.
pub(crate) fn resolve_request(
    method: &str,
    url: &str,
    headers: &[HeaderEntry],
    body: &Body,
) -> Result<ResolvedRequest, AppError> {
    let method: HttpMethod = method.trim().parse()?;
    let url = resolve_url(url)?;
    let mut headers = resolve_headers(headers)?;
    let body = match body {
        Body::None => None,
        Body::Json(content) => Some(with_content_type(&mut headers, media_type::JSON, content)),
        Body::Text(content) => Some(with_content_type(&mut headers, media_type::TEXT, content)),
    };
    Ok(ResolvedRequest {
        method,
        url,
        headers,
        body,
    })
}

fn resolve_url(url: &str) -> Result<Url, AppError> {
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
        .map(|entry| {
            let name = entry.name.trim();
            if is_token(name) && header::is_valid_value(&entry.value) {
                Ok(Header::new(name, entry.value.as_str()))
            } else {
                Err(AppError::InvalidHeader(name.to_owned()))
            }
        })
        .collect()
}

/// The body bytes; adds `Content-Type: default` if no header sets one.
fn with_content_type(headers: &mut Vec<Header>, default: &str, content: &str) -> Vec<u8> {
    let has_type = headers
        .iter()
        .any(|header| header.name.eq_ignore_ascii_case(header::CONTENT_TYPE));
    if !has_type {
        headers.push(Header::new("Content-Type", default));
    }
    content.as_bytes().to_vec()
}

#[cfg(test)]
#[path = "tests/resolve.rs"]
mod tests;

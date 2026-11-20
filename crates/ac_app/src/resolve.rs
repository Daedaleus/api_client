//! Turns what the user entered into a [`ResolvedRequest`].

use ac_domain::exchange::ResolvedRequest;
use ac_domain::protocol::HttpMethod;
use url::Url;

use crate::error::AppError;

/// Scheme assumed when the user enters a URL without one, as other API
/// clients do.
const DEFAULT_SCHEME: &str = "http://";
const SUPPORTED_SCHEMES: [&str; 2] = ["http", "https"];

/// Validates method and URL as entered.
pub(crate) fn resolve_request(method: &str, url: &str) -> Result<ResolvedRequest, AppError> {
    let method: HttpMethod = method.trim().parse()?;
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
    Ok(ResolvedRequest {
        method,
        url,
        headers: Vec::new(),
    })
}

#[cfg(test)]
#[path = "tests/resolve.rs"]
mod tests;

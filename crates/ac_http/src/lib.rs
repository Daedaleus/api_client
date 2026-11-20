//! HTTP adapter: sends requests with [reqwest](https://docs.rs/reqwest).
//!
//! One [`ReqwestSender`] per application; it shares connections between all
//! requests. Redirects are followed (up to 10), TLS certificates are verified.

use std::time::{Duration, Instant};

use ac_app::{HttpSender, TransportError};
use ac_domain::exchange::{Header, ResolvedRequest, Response};
use ac_domain::protocol::StatusCode;

/// Sent as `User-Agent` unless a request sets its own.
pub const USER_AGENT: &str = concat!("api_client/", env!("CARGO_PKG_VERSION"));

/// Time allowed for a whole request, including reading the body.
pub const DEFAULT_TIMEOUT: Duration = Duration::from_secs(30);

/// [`HttpSender`] backed by a shared reqwest client.
#[derive(Debug, Clone)]
pub struct ReqwestSender {
    client: reqwest::Client,
}

impl ReqwestSender {
    /// Creates the sender with a total timeout per request.
    ///
    /// # Errors
    /// If the TLS backend cannot be initialised.
    pub fn new(timeout: Duration) -> Result<Self, reqwest::Error> {
        let client = reqwest::Client::builder()
            .user_agent(USER_AGENT)
            .timeout(timeout)
            .build()?;
        Ok(Self { client })
    }
}

impl HttpSender for ReqwestSender {
    async fn send(&self, request: ResolvedRequest) -> Result<Response, TransportError> {
        let method = reqwest::Method::from_bytes(request.method.as_str().as_bytes())
            .map_err(|error| TransportError::Other(error.to_string()))?;
        let mut builder = self.client.request(method, request.url);
        for header in request.headers {
            builder = builder.header(header.name, header.value);
        }
        if let Some(body) = request.body {
            builder = builder.body(body);
        }

        let started = Instant::now();
        let response = builder
            .send()
            .await
            .map_err(|error| transport_error(&error))?;
        let status = StatusCode::new(response.status().as_u16())
            .map_err(|error| TransportError::Other(error.to_string()))?;
        let headers = response
            .headers()
            .iter()
            .map(|(name, value)| {
                Header::new(name.as_str(), String::from_utf8_lossy(value.as_bytes()))
            })
            .collect();
        let body = response
            .bytes()
            .await
            .map_err(|error| transport_error(&error))?
            .to_vec();

        Ok(Response {
            status,
            headers,
            body,
            elapsed: started.elapsed(),
        })
    }
}

/// Maps a reqwest error to the port's error, with the root cause as detail.
fn transport_error(error: &reqwest::Error) -> TransportError {
    if error.is_timeout() {
        return TransportError::Timeout;
    }
    let detail = root_cause(error);
    if error.is_connect() {
        TransportError::Connect(detail)
    } else {
        TransportError::Other(detail)
    }
}

fn root_cause(error: &reqwest::Error) -> String {
    let mut cause: &dyn std::error::Error = error;
    while let Some(source) = cause.source() {
        cause = source;
    }
    cause.to_string()
}

#[cfg(test)]
#[path = "tests/lib.rs"]
mod tests;

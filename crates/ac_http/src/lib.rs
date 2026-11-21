//! HTTP adapter: sends requests with [reqwest](https://docs.rs/reqwest).
//!
//! One [`ReqwestSender`] per application; it shares connections between all
//! requests. Redirects are followed (up to 10), TLS certificates are verified.
//!
//! [`login`]: receiving the redirect of an OAuth 2 sign-in in the browser
//! ([`LoopbackLogin`]).

pub mod login;

pub use login::{LoopbackLogin, SystemBrowser};

use std::path::Path;
use std::time::{Duration, Instant};

use ac_app::{HttpSender, TransportError};
use ac_domain::exchange::{Header, Part, PartContent, RequestBody, ResolvedRequest, Response};
use ac_domain::protocol::{StatusCode, media_type};
use reqwest::multipart::Form;

/// Sent as `User-Agent` unless a request sets its own.
pub const USER_AGENT: &str = concat!("api_client/", env!("CARGO_PKG_VERSION"));

/// Time allowed for a whole request, including reading the body.
pub const DEFAULT_TIMEOUT: Duration = Duration::from_secs(30);

/// [`HttpSender`] backed by shared reqwest clients: one that checks TLS
/// certificates (the default) and one that does not, for requests whose
/// user switched the check off.
#[derive(Debug, Clone)]
pub struct ReqwestSender {
    client: reqwest::Client,
    unchecked: reqwest::Client,
}

impl ReqwestSender {
    /// Creates the sender with a total timeout per request.
    ///
    /// # Errors
    /// If the TLS backend cannot be initialised.
    pub fn new(timeout: Duration) -> Result<Self, reqwest::Error> {
        let builder = || {
            reqwest::Client::builder()
                .user_agent(USER_AGENT)
                .timeout(timeout)
        };
        Ok(Self {
            client: builder().build()?,
            unchecked: builder().danger_accept_invalid_certs(true).build()?,
        })
    }
}

impl HttpSender for ReqwestSender {
    async fn send(&self, request: ResolvedRequest) -> Result<Response, TransportError> {
        let method = reqwest::Method::from_bytes(request.method.as_str().as_bytes())
            .map_err(|error| TransportError::Other(error.to_string()))?;
        let client = if request.verify_tls {
            &self.client
        } else {
            &self.unchecked
        };
        let mut builder = client.request(method, request.url);
        for header in request.headers {
            builder = builder.header(header.name, header.value);
        }
        builder = match request.body {
            None => builder,
            Some(RequestBody::Bytes(bytes)) => builder.body(bytes),
            Some(RequestBody::File(path)) => builder.body(read(&path).await?),
            Some(RequestBody::Multipart(parts)) => builder.multipart(form(parts).await?),
        };

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

/// Reads a file to send; failing to read it is a failed request.
async fn read(path: &Path) -> Result<Vec<u8>, TransportError> {
    tokio::fs::read(path)
        .await
        .map_err(|error| TransportError::File {
            path: path.display().to_string(),
            detail: error.to_string(),
        })
}

/// Multipart form data; files are sent with their file name.
async fn form(parts: Vec<Part>) -> Result<Form, TransportError> {
    let mut form = Form::new();
    for part in parts {
        form = match part.content {
            PartContent::Text(text) => form.text(part.name, text),
            PartContent::File(path) => {
                let data = read(&path).await?;
                let file_name = path
                    .file_name()
                    .map_or_default(|name| name.to_string_lossy().into_owned());
                let file = reqwest::multipart::Part::bytes(data)
                    .file_name(file_name)
                    .mime_str(media_type::OCTET_STREAM)
                    .map_err(|error| TransportError::Other(error.to_string()))?;
                form.part(part.name, file)
            }
        };
    }
    Ok(form)
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

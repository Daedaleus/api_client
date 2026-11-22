//! HTTP adapter: sends requests with [reqwest](https://docs.rs/reqwest).
//!
//! One [`ReqwestSender`] per application; it shares connections between all
//! requests. By default redirects are followed (up to 10) and TLS
//! certificates are verified; each request may say otherwise, and brings its
//! own time limit.
//!
//! [`login`]: receiving the redirect of an OAuth 2 sign-in in the browser
//! ([`LoopbackLogin`]).

pub mod login;

pub use login::{LoopbackLogin, SystemBrowser};

use std::path::Path;
use std::sync::{Arc, PoisonError, RwLock};
use std::time::{Duration, Instant};

use ac_app::ports::SettingsError;
use ac_app::settings::{NetworkSettings, ProxySetting};
use ac_app::{HttpSender, TransportError};
use ac_domain::exchange::{Header, Part, PartContent, RequestBody, ResolvedRequest, Response};
use ac_domain::protocol::{StatusCode, media_type};
use reqwest::multipart::Form;

/// Sent as `User-Agent` unless a request sets its own.
pub const USER_AGENT: &str = concat!("api_client/", env!("CARGO_PKG_VERSION"));

/// Time allowed for a whole request, including reading the body, when the
/// request brings none (it always does when sent by the app).
pub const DEFAULT_TIMEOUT: Duration = Duration::from_secs(30);

/// [`HttpSender`] backed by shared reqwest clients, one per combination of
/// certificate check and redirect policy (reqwest sets both per client).
/// The app's network settings (proxy, extra CA certificates) apply to all
/// of them; changing them builds new clients.
#[derive(Debug, Clone)]
pub struct ReqwestSender {
    /// Indexed by [`client_index`]; replaced whole by `configure`.
    clients: Arc<RwLock<Arc<[reqwest::Client; 4]>>>,
    timeout: Duration,
}

/// Which client fits a request.
const fn client_index(verify_tls: bool, follow_redirects: bool) -> usize {
    (verify_tls as usize) * 2 + follow_redirects as usize
}

impl ReqwestSender {
    /// Creates the sender with a total timeout per request.
    ///
    /// # Errors
    /// If the TLS backend cannot be initialised.
    pub fn new(timeout: Duration) -> Result<Self, reqwest::Error> {
        let clients = build_clients(timeout, &Network::default())?;
        Ok(Self {
            clients: Arc::new(RwLock::new(Arc::new(clients))),
            timeout,
        })
    }

    fn client(&self, verify_tls: bool, follow_redirects: bool) -> reqwest::Client {
        let clients = self.clients.read().unwrap_or_else(PoisonError::into_inner);
        clients[client_index(verify_tls, follow_redirects)].clone()
    }
}

/// The network settings in reqwest's terms, checked.
#[derive(Default)]
struct Network {
    /// `None`: the system's proxy settings.
    proxy: Option<ProxyChoice>,
    certificates: Vec<reqwest::Certificate>,
}

enum ProxyChoice {
    Direct,
    Via(Box<reqwest::Proxy>),
}

impl Network {
    /// Checks the settings: a usable proxy URL, readable certificate files.
    fn from_settings(settings: &NetworkSettings) -> Result<Self, SettingsError> {
        let proxy = match &settings.proxy {
            ProxySetting::System => None,
            ProxySetting::None => Some(ProxyChoice::Direct),
            ProxySetting::Manual { url, no_proxy } => {
                let proxy = reqwest::Proxy::all(url.as_str())
                    .map_err(|error| SettingsError(format!("proxy {url}: {}", root_cause(&error))))?
                    .no_proxy(reqwest::NoProxy::from_string(no_proxy));
                Some(ProxyChoice::Via(Box::new(proxy)))
            }
        };
        let mut certificates = Vec::new();
        for path in &settings.ca_certificates {
            certificates.extend(certificates_in(path)?);
        }
        Ok(Self {
            proxy,
            certificates,
        })
    }
}

/// The certificates in a PEM file (one or several) or a DER file.
fn certificates_in(path: &Path) -> Result<Vec<reqwest::Certificate>, SettingsError> {
    let failed = |detail: String| SettingsError(format!("{}: {detail}", path.display()));
    let bytes = std::fs::read(path).map_err(|error| failed(error.to_string()))?;
    let certificates = if bytes.starts_with(b"-----BEGIN") {
        reqwest::Certificate::from_pem_bundle(&bytes)
    } else {
        reqwest::Certificate::from_der(&bytes).map(|certificate| vec![certificate])
    }
    .map_err(|error| failed(root_cause(&error)))?;
    if certificates.is_empty() {
        return Err(failed("no certificate in the file".into()));
    }
    Ok(certificates)
}

/// One client per certificate check and redirect policy.
fn build_clients(
    timeout: Duration,
    network: &Network,
) -> Result<[reqwest::Client; 4], reqwest::Error> {
    let build = |verify_tls: bool, follow_redirects: bool| {
        let mut builder = reqwest::Client::builder()
            .user_agent(USER_AGENT)
            .timeout(timeout)
            .danger_accept_invalid_certs(!verify_tls)
            .redirect(if follow_redirects {
                reqwest::redirect::Policy::default()
            } else {
                reqwest::redirect::Policy::none()
            });
        builder = match &network.proxy {
            None => builder,
            Some(ProxyChoice::Direct) => builder.no_proxy(),
            Some(ProxyChoice::Via(proxy)) => builder.proxy(proxy.as_ref().clone()),
        };
        if !network.certificates.is_empty() {
            builder = builder.tls_certs_merge(network.certificates.iter().cloned());
        }
        builder.build()
    };
    Ok([
        build(false, false)?,
        build(false, true)?,
        build(true, false)?,
        build(true, true)?,
    ])
}

impl HttpSender for ReqwestSender {
    async fn send(&self, request: ResolvedRequest) -> Result<Response, TransportError> {
        let method = reqwest::Method::from_bytes(request.method.as_str().as_bytes())
            .map_err(|error| TransportError::Other(error.to_string()))?;
        let client = self.client(request.verify_tls, request.transport.follow_redirects);
        let mut builder = client
            .request(method, request.url)
            .timeout(request.transport.timeout);
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

    fn configure(&self, network: &NetworkSettings) -> Result<(), SettingsError> {
        let checked = Network::from_settings(network)?;
        let clients = build_clients(self.timeout, &checked)
            .map_err(|error| SettingsError(root_cause(&error)))?;
        *self.clients.write().unwrap_or_else(PoisonError::into_inner) = Arc::new(clients);
        Ok(())
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

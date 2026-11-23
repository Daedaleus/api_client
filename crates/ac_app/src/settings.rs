//! Settings of the app itself (not of a collection), kept in the app's
//! configuration folder: how requests reach the network – proxy, extra
//! trusted CA certificates and client certificates per host. They apply to
//! every request of every collection, because they belong to the computer
//! and its network, not to an API.
//!
//! A client certificate's passphrase is stored only encrypted for the
//! user's own age key ([`EncryptedPassphrase`]); the HTTP sender gets it
//! decrypted, in memory ([`ClientIdentity`]).

use std::path::PathBuf;

use url::Url;

/// How requests reach servers.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum ProxySetting {
    /// The operating system's proxy settings (and `HTTP_PROXY` & co.).
    #[default]
    System,
    /// Directly, without a proxy.
    None,
    /// Through this proxy.
    Manual {
        /// Proxy URL, e.g. `http://proxy.example.com:8080` or `socks5://…`.
        url: String,
        /// Hosts reached directly, comma-separated (e.g. `localhost,.internal`).
        no_proxy: String,
    },
}

/// Network settings of the app.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct NetworkSettings {
    /// The proxy.
    pub proxy: ProxySetting,
    /// PEM or DER files with CA certificates trusted in addition to the
    /// system's (e.g. a company CA).
    pub ca_certificates: Vec<PathBuf>,
    /// Client certificates (mTLS); the first whose host matches is used.
    pub client_certificates: Vec<ClientCertificate>,
}

/// A client certificate for the servers of one host (mTLS).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClientCertificate {
    /// Where it is sent.
    pub host: HostPattern,
    /// A PEM file (certificate chain, possibly with the key) or a PKCS#12
    /// file (`.p12`, `.pfx`) with certificate and key.
    pub certificate: PathBuf,
    /// A PEM file with the private key, if it is not in `certificate`.
    pub key: Option<PathBuf>,
    /// The passphrase of the key or PKCS#12 file, if it has one.
    pub passphrase: Option<EncryptedPassphrase>,
}

/// A passphrase encrypted for the user's own age key (base64), as kept in
/// the settings file and shown to the settings dialog.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EncryptedPassphrase(pub String);

/// A client certificate ready for the sender: the passphrase decrypted.
#[derive(Clone, PartialEq, Eq)]
pub struct ClientIdentity {
    /// Where it is sent.
    pub host: HostPattern,
    /// PEM or PKCS#12 file.
    pub certificate: PathBuf,
    /// PEM key file, if separate.
    pub key: Option<PathBuf>,
    /// The passphrase in plain text.
    pub passphrase: Option<String>,
}

impl std::fmt::Debug for ClientIdentity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ClientIdentity")
            .field("host", &self.host)
            .field("certificate", &self.certificate)
            .field("key", &self.key)
            .field("passphrase", &self.passphrase.as_ref().map(|_| "***"))
            .finish()
    }
}

/// Which hosts a client certificate is for: `api.example.com`,
/// `api.example.com:8443` (only that port) or `*.example.com` (its
/// subdomains, not `example.com` itself). Compared without case.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HostPattern(String);

impl HostPattern {
    /// The pattern as written (trimmed, lower case).
    #[must_use]
    pub fn new(pattern: &str) -> Self {
        Self(pattern.trim().to_ascii_lowercase())
    }

    /// As written.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Whether a request to `url` goes to these hosts.
    #[must_use]
    pub fn matches(&self, url: &Url) -> bool {
        let Some(host) = url.host_str().map(str::to_ascii_lowercase) else {
            return false;
        };
        let (pattern, port) = match self.0.rsplit_once(':') {
            Some((name, port)) if !name.ends_with(']') || name.starts_with('[') => {
                match port.parse::<u16>() {
                    Ok(port) => (name, Some(port)),
                    Err(_) => (self.0.as_str(), None),
                }
            }
            _ => (self.0.as_str(), None),
        };
        if port.is_some_and(|port| url.port_or_known_default() != Some(port)) {
            return false;
        }
        match pattern.strip_prefix("*.") {
            Some(domain) => host
                .strip_suffix(domain)
                .is_some_and(|sub| sub.len() > 1 && sub.ends_with('.')),
            None => !pattern.is_empty() && host == pattern,
        }
    }
}

/// All settings of the app.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct AppSettings {
    /// Network.
    pub network: NetworkSettings,
}

#[cfg(test)]
#[path = "tests/settings.rs"]
mod tests;

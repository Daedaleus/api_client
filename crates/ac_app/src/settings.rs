//! Settings of the app itself (not of a collection), kept in the app's
//! configuration folder: how requests reach the network – proxy and extra
//! trusted CA certificates. They apply to every request of every
//! collection, because they belong to the computer and its network, not to
//! an API.

use std::path::PathBuf;

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
}

/// All settings of the app.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct AppSettings {
    /// Network.
    pub network: NetworkSettings,
}

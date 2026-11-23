//! The reqwest clients of the sender. reqwest sets certificate check,
//! redirect policy, proxy, trusted CAs and the client certificate per
//! client, so there is one client per certificate check and redirect policy
//! ([`ClientSet`]) – and one such set per configured client certificate,
//! chosen by the request's host. rustls cannot choose a client certificate
//! by server name itself (its resolver does not see it), hence the sets.

use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use ac_app::ports::SettingsError;
use ac_app::settings::{ClientIdentity, HostPattern, NetworkSettings, ProxySetting};
use url::Url;

use crate::{USER_AGENT, cookies, identity, root_cause};

/// One client per certificate check and redirect policy, indexed by
/// [`index`].
pub(crate) type ClientSet = [reqwest::Client; 4];

/// Which client of a set fits a request.
const fn index(verify_tls: bool, follow_redirects: bool) -> usize {
    (verify_tls as usize) * 2 + follow_redirects as usize
}

/// All clients: without client certificate, and one set per certificate.
#[derive(Debug)]
pub(crate) struct Clients {
    plain: ClientSet,
    with_certificate: Vec<(HostPattern, ClientSet)>,
}

impl Clients {
    /// Builds the clients for `network` and the client certificates.
    pub(crate) fn build(
        timeout: Duration,
        network: &NetworkSettings,
        identities: &[ClientIdentity],
    ) -> Result<Self, SettingsError> {
        let checked = Network::from_settings(network)?;
        let built = |identity: Option<&reqwest::Identity>| {
            build_set(timeout, &checked, identity)
                .map_err(|error| SettingsError(root_cause(&error)))
        };
        let mut with_certificate = Vec::with_capacity(identities.len());
        for client_identity in identities {
            let loaded = identity::load(client_identity)?;
            with_certificate.push((client_identity.host.clone(), built(Some(&loaded))?));
        }
        Ok(Self {
            plain: built(None)?,
            with_certificate,
        })
    }

    /// The defaults (no proxy settings, no extra certificates).
    pub(crate) fn defaults(timeout: Duration) -> Result<Self, reqwest::Error> {
        Ok(Self {
            plain: build_set(timeout, &Network::default(), None)?,
            with_certificate: Vec::new(),
        })
    }

    /// The client for a request to `url`: with the first client
    /// certificate whose host matches, else without.
    pub(crate) fn pick(
        &self,
        url: &Url,
        verify_tls: bool,
        follow_redirects: bool,
    ) -> reqwest::Client {
        let set = self
            .with_certificate
            .iter()
            .find(|(host, _)| host.matches(url))
            .map_or(&self.plain, |(_, set)| set);
        set[index(verify_tls, follow_redirects)].clone()
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
fn build_set(
    timeout: Duration,
    network: &Network,
    identity: Option<&reqwest::Identity>,
) -> Result<ClientSet, reqwest::Error> {
    let build = |verify_tls: bool, follow_redirects: bool| {
        let mut builder = reqwest::Client::builder()
            .user_agent(USER_AGENT)
            .timeout(timeout)
            .danger_accept_invalid_certs(!verify_tls)
            .cookie_provider(Arc::new(cookies::JarProvider))
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
        if let Some(identity) = identity {
            builder = builder.identity(identity.clone());
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

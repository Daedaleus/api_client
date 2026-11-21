//! Whether the server's TLS certificate is checked. It is, unless a request,
//! a folder around it or the collection switches it off – for test servers
//! with self-signed certificates. Like authentication, the setting is
//! inherited from the folders and the collection.

/// The certificate check of a request, folder or collection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CertificateCheck {
    /// As the folder or collection around it (on, if nothing says else).
    #[default]
    Inherit,
    /// Check the certificate.
    On,
    /// Do not check it: any certificate is accepted.
    Off,
}

/// Whether certificates are checked, given the chain from the request
/// outwards (request, inner folder … outer folder, collection): the first
/// setting that does not inherit decides; checking is the default.
#[must_use]
pub fn checks_certificates(chain: impl IntoIterator<Item = CertificateCheck>) -> bool {
    chain
        .into_iter()
        .find(|check| *check != CertificateCheck::Inherit)
        .is_none_or(|check| check == CertificateCheck::On)
}

#[cfg(test)]
#[path = "tests/tls.rs"]
mod tests;

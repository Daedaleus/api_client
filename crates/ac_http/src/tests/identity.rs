use std::path::PathBuf;
use std::sync::Arc;

use ac_app::HttpSender;
use ac_app::settings::{ClientIdentity, HostPattern, NetworkSettings};
use ac_domain::exchange::ResolvedRequest;
use ac_domain::protocol::HttpMethod;
use rcgen::{
    BasicConstraints, CertificateParams, CertifiedIssuer, DistinguishedName, DnType, IsCa, KeyPair,
};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio_rustls::TlsAcceptor;
use tokio_rustls::rustls::pki_types::{CertificateDer, PrivateKeyDer, PrivatePkcs8KeyDer};
use tokio_rustls::rustls::server::WebPkiClientVerifier;
use tokio_rustls::rustls::{RootCertStore, ServerConfig, crypto};
use url::Url;

use super::*;
use crate::ReqwestSender;

const PASSPHRASE: &str = "c0rrect horse";

/// A CA, a server certificate for `localhost` and a client certificate in
/// every supported form, written to a temporary folder.
struct Pki {
    folder: tempfile::TempDir,
    ca_der: CertificateDer<'static>,
    server_der: CertificateDer<'static>,
    server_key_der: Vec<u8>,
}

impl Pki {
    fn new() -> Self {
        let folder = tempfile::tempdir().unwrap();
        // Distinct names: with rcgen's default name for all, the server
        // certificate would look self-signed (subject = issuer).
        let named = |names: Vec<String>, common_name: &str| {
            let mut params = CertificateParams::new(names).unwrap();
            params.distinguished_name = DistinguishedName::new();
            params
                .distinguished_name
                .push(DnType::CommonName, common_name);
            params
        };
        let mut ca_params = named(Vec::new(), "api_client test CA");
        ca_params.is_ca = IsCa::Ca(BasicConstraints::Unconstrained);
        let ca = CertifiedIssuer::self_signed(ca_params, KeyPair::generate().unwrap()).unwrap();
        let server_key = KeyPair::generate().unwrap();
        let server = named(vec!["localhost".to_owned()], "localhost")
            .signed_by(&server_key, &ca)
            .unwrap();
        let client_key = KeyPair::generate().unwrap();
        let client = named(vec!["tester".to_owned()], "tester")
            .signed_by(&client_key, &ca)
            .unwrap();

        let write =
            |name: &str, bytes: &[u8]| std::fs::write(folder.path().join(name), bytes).unwrap();
        write("ca.pem", ca.pem().as_bytes());
        write("client.pem", client.pem().as_bytes());
        write("client-key.pem", client_key.serialize_pem().as_bytes());
        write(
            "combined.pem",
            format!("{}{}", client_key.serialize_pem(), client.pem()).as_bytes(),
        );
        // Encrypted PKCS#8 (PBES2: PBKDF2-SHA256, AES-256-CBC).
        let key_der = client_key.serialize_der();
        let info = pkcs8::PrivateKeyInfo::try_from(key_der.as_slice()).unwrap();
        let params =
            pkcs8::pkcs5::pbes2::Parameters::pbkdf2_sha256_aes256cbc(2048, b"saltsalt", &[7; 16])
                .unwrap();
        let encrypted = info.encrypt_with_params(params, PASSPHRASE).unwrap();
        write(
            "encrypted-key.pem",
            encrypted
                .to_pem("ENCRYPTED PRIVATE KEY", LineEnding::LF)
                .unwrap()
                .as_bytes(),
        );
        write(
            "legacy-key.pem",
            b"-----BEGIN RSA PRIVATE KEY-----\nProc-Type: 4,ENCRYPTED\nDEK-Info: AES-128-CBC,00\n\nAAAA\n-----END RSA PRIVATE KEY-----\n",
        );
        // PKCS#12 with passphrase.
        let mut store = p12_keystore::KeyStore::new();
        store.add_entry(
            "tester",
            p12_keystore::KeyStoreEntry::PrivateKeyChain(p12_keystore::PrivateKeyChain::new(
                b"tester".as_slice(),
                p12_keystore::PrivateKey::from_der(&client_key.serialize_der()).unwrap(),
                [p12_keystore::Certificate::from_der(client.der()).unwrap()],
            )),
        );
        write("client.p12", &store.writer(PASSPHRASE).write().unwrap());
        Self {
            ca_der: ca.der().clone(),
            server_der: server.der().clone(),
            server_key_der: server_key.serialize_der(),
            folder,
        }
    }

    fn path(&self, name: &str) -> PathBuf {
        self.folder.path().join(name)
    }

    fn identity(
        &self,
        certificate: &str,
        key: Option<&str>,
        passphrase: Option<&str>,
    ) -> ClientIdentity {
        ClientIdentity {
            host: HostPattern::new("localhost"),
            certificate: self.path(certificate),
            key: key.map(|key| self.path(key)),
            passphrase: passphrase.map(Into::into),
        }
    }
}

fn error_of(identity: &ClientIdentity) -> String {
    load(identity).unwrap_err().0
}

#[test]
fn every_form_of_client_certificate_loads() {
    let pki = Pki::new();
    for identity in [
        pki.identity("client.pem", Some("client-key.pem"), None),
        pki.identity("combined.pem", None, None),
        pki.identity("client.pem", Some("encrypted-key.pem"), Some(PASSPHRASE)),
        pki.identity("client.p12", None, Some(PASSPHRASE)),
    ] {
        load(&identity).unwrap_or_else(|error| panic!("{identity:?}: {error:?}"));
    }
}

#[test]
fn unusable_certificates_say_why() {
    let pki = Pki::new();
    let cases = [
        (pki.identity("missing.pem", None, None), "missing.pem"),
        (pki.identity("client.pem", None, None), "no private key"),
        (pki.identity("client-key.pem", None, None), "no certificate"),
        (
            pki.identity("client.pem", Some("encrypted-key.pem"), None),
            "needs a passphrase",
        ),
        (
            pki.identity("client.pem", Some("encrypted-key.pem"), Some("wrong")),
            "wrong passphrase",
        ),
        (
            pki.identity("client.pem", Some("legacy-key.pem"), None),
            "openssl pkcs8 -topk8",
        ),
        (
            pki.identity("client.p12", None, Some("wrong")),
            "wrong passphrase",
        ),
        (
            pki.identity("client.pem", Some("missing-key.pem"), None),
            "missing-key.pem",
        ),
    ];
    for (identity, expected) in cases {
        let error = error_of(&identity);
        assert!(error.contains(expected), "{expected}: {error}");
    }
    std::fs::write(pki.path("binary.bin"), [0xff, 0xfe, 0x00]).unwrap();
    assert!(error_of(&pki.identity("binary.bin", None, None)).contains("PKCS#12"));
    std::fs::write(pki.path("latin1.pem"), b"-----BEGIN CERTIFICATE\xff").unwrap();
    assert!(error_of(&pki.identity("latin1.pem", None, None)).contains("neither PEM"));
}

/// A TLS server on localhost that requires a client certificate of the
/// test CA and answers `200 ok`.
async fn mtls_server(pki: &Pki) -> u16 {
    let provider = Arc::new(crypto::aws_lc_rs::default_provider());
    let mut roots = RootCertStore::empty();
    roots.add(pki.ca_der.clone()).unwrap();
    let verifier = WebPkiClientVerifier::builder_with_provider(Arc::new(roots), provider.clone())
        .build()
        .unwrap();
    let config = ServerConfig::builder_with_provider(provider)
        .with_safe_default_protocol_versions()
        .unwrap()
        .with_client_cert_verifier(verifier)
        .with_single_cert(
            vec![pki.server_der.clone()],
            PrivateKeyDer::Pkcs8(PrivatePkcs8KeyDer::from(pki.server_key_der.clone())),
        )
        .unwrap();
    let acceptor = TlsAcceptor::from(Arc::new(config));
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    tokio::spawn(async move {
        while let Ok((stream, _)) = listener.accept().await {
            let acceptor = acceptor.clone();
            tokio::spawn(async move {
                let Ok(mut tls) = acceptor.accept(stream).await else {
                    return;
                };
                let mut buffer = [0_u8; 4096];
                let mut request = Vec::new();
                while !request.windows(4).any(|w| w == b"\r\n\r\n") {
                    match tls.read(&mut buffer).await {
                        Ok(0) | Err(_) => return,
                        Ok(read) => request.extend_from_slice(&buffer[..read]),
                    }
                }
                let _ = tls
                    .write_all(
                        b"HTTP/1.1 200 OK\r\ncontent-length: 2\r\nconnection: close\r\n\r\nok",
                    )
                    .await;
                let _ = tls.shutdown().await;
            });
        }
    });
    port
}

/// `GET https://localhost:<port>/` (the server certificate is for `localhost`).
fn get(port: u16) -> ResolvedRequest {
    ResolvedRequest {
        transport: ac_domain::transport::EffectiveTransport::default(),
        verify_tls: true,
        method: HttpMethod::Get,
        url: Url::parse(&format!("https://localhost:{port}/")).unwrap(),
        headers: Vec::new(),
        body: None,
    }
}

fn network(pki: &Pki) -> NetworkSettings {
    NetworkSettings {
        ca_certificates: vec![pki.path("ca.pem")],
        ..NetworkSettings::default()
    }
}

#[tokio::test]
async fn the_certificate_of_the_matching_host_is_sent() {
    let pki = Pki::new();
    let port = mtls_server(&pki).await;
    let sender = ReqwestSender::new(std::time::Duration::from_secs(10)).unwrap();
    let for_host = |host: &str, identity: ClientIdentity| ClientIdentity {
        host: HostPattern::new(host),
        ..identity
    };

    // PKCS#12 for exactly this host and port.
    let p12 = pki.identity("client.p12", None, Some(PASSPHRASE));
    sender
        .configure(
            &network(&pki),
            &[for_host(&format!("localhost:{port}"), p12.clone())],
        )
        .unwrap();
    let response = sender.send(get(port), None).await.unwrap();
    assert_eq!(response.status.as_u16(), 200);
    assert_eq!(response.body, b"ok");

    // A PEM certificate with encrypted key, the first matching entry wins.
    let encrypted = pki.identity("client.pem", Some("encrypted-key.pem"), Some(PASSPHRASE));
    sender
        .configure(
            &network(&pki),
            &[
                for_host("other.example", p12.clone()),
                for_host("localhost", encrypted),
            ],
        )
        .unwrap();
    assert_eq!(
        sender.send(get(port), None).await.unwrap().status.as_u16(),
        200
    );

    // No certificate for this host: the server refuses the handshake.
    sender
        .configure(&network(&pki), &[for_host("*.example", p12)])
        .unwrap();
    assert!(sender.send(get(port), None).await.is_err());

    // A certificate that cannot be read changes nothing.
    let broken = pki.identity("client.pem", None, None);
    assert!(sender.configure(&network(&pki), &[broken]).is_err());
    assert!(sender.send(get(port), None).await.is_err());
}

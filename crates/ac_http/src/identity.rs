//! Client certificates (mTLS) for reqwest: certificate chain and private
//! key as PEM, which is what rustls reads. Converted from
//!
//! - PEM with an unencrypted key (PKCS#8, PKCS#1 RSA or SEC1 EC), in the
//!   certificate file or a separate key file,
//! - PEM with an encrypted PKCS#8 key (`ENCRYPTED PRIVATE KEY`, PBES2 or
//!   3DES), decrypted with the passphrase,
//! - PKCS#12 (`.p12`, `.pfx`), decrypted with the passphrase (or none).
//!
//! Old OpenSSL-encrypted PEM keys (`Proc-Type: 4,ENCRYPTED`) are refused
//! with a hint how to convert them.

use std::path::Path;

use ac_app::ports::SettingsError;
use ac_app::settings::ClientIdentity;
use pkcs8::der::pem::LineEnding;
use pkcs8::der::{Decode as _, pem};

/// PEM labels.
mod label {
    pub const CERTIFICATE: &str = "CERTIFICATE";
    pub const PRIVATE_KEY: &str = "PRIVATE KEY";
    pub const RSA_PRIVATE_KEY: &str = "RSA PRIVATE KEY";
    pub const EC_PRIVATE_KEY: &str = "EC PRIVATE KEY";
    pub const ENCRYPTED_PRIVATE_KEY: &str = "ENCRYPTED PRIVATE KEY";
}

/// File endings of PKCS#12 files.
const PKCS12_EXTENSIONS: [&str; 2] = ["p12", "pfx"];
/// Marks a PEM block.
const PEM_BEGIN: &str = "-----BEGIN ";
/// The header of keys encrypted the old OpenSSL way.
const LEGACY_ENCRYPTION: &str = "Proc-Type: 4,ENCRYPTED";

/// The identity reqwest sends for `identity`.
pub(crate) fn load(identity: &ClientIdentity) -> Result<reqwest::Identity, SettingsError> {
    let failed =
        |detail: String| SettingsError(format!("{}: {detail}", identity.certificate.display()));
    let pem = identity_pem(identity).map_err(failed)?;
    reqwest::Identity::from_pem(pem.as_bytes()).map_err(|error| failed(crate::root_cause(&error)))
}

/// Certificate chain and unencrypted PKCS#8/PKCS#1/SEC1 key as one PEM text.
fn identity_pem(identity: &ClientIdentity) -> Result<String, String> {
    let bytes = read(&identity.certificate)?;
    let passphrase = identity.passphrase.as_deref();
    if is_pkcs12(&identity.certificate, &bytes) {
        return pkcs12_pem(&bytes, passphrase.unwrap_or_default());
    }
    let text = String::from_utf8(bytes).map_err(|_| "neither PEM nor PKCS#12".to_owned())?;
    let mut blocks = blocks(&text);
    if let Some(key_file) = &identity.key {
        let key = read(key_file)?;
        let key = String::from_utf8(key)
            .map_err(|_| format!("{}: not a PEM key file", key_file.display()))?;
        blocks.extend(self::blocks(&key));
    }
    let certificates: Vec<&str> = blocks
        .iter()
        .filter(|block| block.label == label::CERTIFICATE)
        .map(|block| block.text.as_str())
        .collect();
    if certificates.is_empty() {
        return Err("no certificate in the file".into());
    }
    let key = blocks
        .iter()
        .find(|block| {
            [
                label::PRIVATE_KEY,
                label::RSA_PRIVATE_KEY,
                label::EC_PRIVATE_KEY,
                label::ENCRYPTED_PRIVATE_KEY,
            ]
            .contains(&block.label.as_str())
        })
        .ok_or_else(|| "no private key (choose the key file)".to_owned())?;
    let key = if key.text.contains(LEGACY_ENCRYPTION) {
        return Err(
            "this key is encrypted the old OpenSSL way; convert it with \
             `openssl pkcs8 -topk8 -in key.pem -out key-pkcs8.pem`"
                .into(),
        );
    } else if key.label == label::ENCRYPTED_PRIVATE_KEY {
        let passphrase = passphrase.ok_or_else(|| "the key needs a passphrase".to_owned())?;
        decrypt_pkcs8(&key.text, passphrase)?
    } else {
        key.text.clone()
    };
    Ok(format!("{}{key}", certificates.concat()))
}

fn read(path: &Path) -> Result<Vec<u8>, String> {
    std::fs::read(path).map_err(|error| format!("{}: {error}", path.display()))
}

fn is_pkcs12(path: &Path, bytes: &[u8]) -> bool {
    let by_name = path.extension().is_some_and(|extension| {
        PKCS12_EXTENSIONS
            .iter()
            .any(|known| extension.eq_ignore_ascii_case(known))
    });
    by_name || !bytes.trim_ascii_start().starts_with(PEM_BEGIN.as_bytes())
}

/// The key and certificates of a PKCS#12 file as PEM.
fn pkcs12_pem(bytes: &[u8], passphrase: &str) -> Result<String, String> {
    let store = p12_keystore::KeyStore::from_pkcs12(
        bytes,
        passphrase,
        p12_keystore::Pkcs12ImportPolicy::Relaxed,
    )
    .map_err(|error| format!("cannot open the PKCS#12 file (wrong passphrase?): {error}"))?;
    let (_, chain) = store
        .private_key_chain()
        .ok_or_else(|| "no private key in the PKCS#12 file".to_owned())?;
    if chain.certs().is_empty() {
        return Err("no certificate for the key in the PKCS#12 file".into());
    }
    let mut text = String::new();
    for certificate in chain.certs() {
        text.push_str(&encode(label::CERTIFICATE, certificate.as_der())?);
    }
    text.push_str(&encode(label::PRIVATE_KEY, chain.key().as_der())?);
    Ok(text)
}

/// An encrypted PKCS#8 key (PEM) decrypted, as unencrypted PKCS#8 PEM.
fn decrypt_pkcs8(text: &str, passphrase: &str) -> Result<String, String> {
    let (_, der) = pem::decode_vec(text.as_bytes()).map_err(|error| error.to_string())?;
    let info = pkcs8::EncryptedPrivateKeyInfo::from_der(&der).map_err(|error| error.to_string())?;
    let key = info
        .decrypt(passphrase)
        .map_err(|_| "cannot decrypt the key (wrong passphrase?)".to_owned())?;
    encode(label::PRIVATE_KEY, key.as_bytes())
}

fn encode(label: &str, der: &[u8]) -> Result<String, String> {
    pem::encode_string(label, LineEnding::LF, der).map_err(|error| error.to_string())
}

/// One PEM block: its label and its text (with BEGIN/END lines).
struct Block {
    label: String,
    text: String,
}

/// The PEM blocks of `text`, in order.
fn blocks(text: &str) -> Vec<Block> {
    let mut found = Vec::new();
    let mut rest = text;
    while let Some(start) = rest.find(PEM_BEGIN) {
        let after = &rest[start + PEM_BEGIN.len()..];
        let Some(label_end) = after.find("-----") else {
            break;
        };
        let label = after[..label_end].to_owned();
        let end_marker = format!("-----END {label}-----");
        let Some(end) = rest[start..].find(&end_marker) else {
            break;
        };
        let stop = start + end + end_marker.len();
        found.push(Block {
            label,
            text: format!("{}\n", &rest[start..stop]),
        });
        rest = &rest[stop..];
    }
    found
}

#[cfg(test)]
#[path = "tests/identity.rs"]
mod tests;

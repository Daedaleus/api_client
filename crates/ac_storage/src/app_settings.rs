//! [`FileSettings`]: the app's own settings as `settings.toml` in its
//! configuration folder (never in a collection).
//!
//! ```toml
//! format = "api-client/settings"
//! version = 1
//!
//! [proxy]
//! mode = "manual"            # "system" (default), "none" or "manual"
//! url = "http://proxy:8080"
//! no_proxy = "localhost,.internal"
//!
//! [tls]
//! ca_certificates = ["C:/certs/company-ca.pem"]
//!
//! [[tls.client_certificates]]
//! host = "api.internal"            # or "api.internal:8443", "*.internal"
//! certificate = "C:/certs/me.p12"  # PEM or PKCS#12
//! key = "C:/certs/me.key"          # optional: PEM key file
//! passphrase = "YWdlLWVu…"         # optional: age-encrypted, base64
//! ```
//!
//! The passphrase is encrypted for the user's own age key by the app; this
//! file never holds it in plain text.
//!
//! A file that cannot be read is reported and kept: saving moves it aside
//! (`settings.toml.<time>.bak`) instead of overwriting it.

use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use ac_app::ports::{SettingsError, SettingsStore};
use ac_app::settings::{
    AppSettings, ClientCertificate, EncryptedPassphrase, HostPattern, NetworkSettings, ProxySetting,
};
use toml_edit::{Array, ArrayOfTables, DocumentMut, Item, Table, value};

use crate::document;
use crate::format::CURRENT_VERSION;

/// File name in the configuration folder.
pub const SETTINGS_FILE: &str = "settings.toml";
/// Format name in the file.
const FORMAT: &str = "api-client/settings";
const PROXY: &str = "proxy";
const MODE: &str = "mode";
const URL: &str = "url";
const NO_PROXY: &str = "no_proxy";
const TLS: &str = "tls";
const CA_CERTIFICATES: &str = "ca_certificates";
const CLIENT_CERTIFICATES: &str = "client_certificates";
const HOST: &str = "host";
const CERTIFICATE: &str = "certificate";
const KEY: &str = "key";
const PASSPHRASE: &str = "passphrase";
const MODE_SYSTEM: &str = "system";
const MODE_NONE: &str = "none";
const MODE_MANUAL: &str = "manual";

/// The app's settings in `<folder>/settings.toml`.
#[derive(Debug, Clone)]
pub struct FileSettings {
    path: PathBuf,
}

impl FileSettings {
    /// The settings in `folder` (created when first saved).
    #[must_use]
    pub fn new(folder: &Path) -> Self {
        Self {
            path: folder.join(SETTINGS_FILE),
        }
    }

    fn failed(&self, detail: impl std::fmt::Display) -> SettingsError {
        SettingsError(format!("{}: {detail}", self.path.display()))
    }
}

impl SettingsStore for FileSettings {
    fn load(&self) -> Result<AppSettings, SettingsError> {
        let text = match std::fs::read_to_string(&self.path) {
            Ok(text) => text,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                return Ok(AppSettings::default());
            }
            Err(error) => return Err(self.failed(error)),
        };
        let parsed = document::parse(&self.path, &text, FORMAT)
            .map_err(|error| SettingsError(error.to_string()))?;
        settings_from(&parsed.doc).map_err(|detail| self.failed(detail))
    }

    fn save(&self, settings: &AppSettings) -> Result<(), SettingsError> {
        if self.path.exists() && self.load().is_err() {
            // Never overwrite what cannot be read: keep it next to the new file.
            let backup = self.path.with_extension(format!("toml.{}.bak", now_ms()));
            std::fs::rename(&self.path, &backup).map_err(|error| self.failed(error))?;
        }
        if let Some(folder) = self.path.parent() {
            std::fs::create_dir_all(folder).map_err(|error| self.failed(error))?;
        }
        document::write_atomically(&self.path, document_of(settings).to_string())
            .map_err(|error| SettingsError(error.to_string()))
    }
}

fn settings_from(doc: &DocumentMut) -> Result<AppSettings, String> {
    let text = |table: &Table, key: &str| -> Result<String, String> {
        match table.get(key) {
            None => Ok(String::new()),
            Some(item) => item
                .as_str()
                .map(str::to_owned)
                .ok_or_else(|| format!("{key} must be a string")),
        }
    };
    let proxy = match doc.get(PROXY).and_then(Item::as_table) {
        None => ProxySetting::System,
        Some(table) => match text(table, MODE)?.as_str() {
            "" | MODE_SYSTEM => ProxySetting::System,
            MODE_NONE => ProxySetting::None,
            MODE_MANUAL => ProxySetting::Manual {
                url: text(table, URL)?,
                no_proxy: text(table, NO_PROXY)?,
            },
            other => return Err(format!("unknown proxy mode {other:?}")),
        },
    };
    let ca_certificates = match doc
        .get(TLS)
        .and_then(Item::as_table)
        .and_then(|table| table.get(CA_CERTIFICATES))
    {
        None => Vec::new(),
        Some(item) => item
            .as_array()
            .ok_or_else(|| format!("{CA_CERTIFICATES} must be a list"))?
            .iter()
            .map(|entry| {
                entry
                    .as_str()
                    .map(PathBuf::from)
                    .ok_or_else(|| format!("{CA_CERTIFICATES} must hold file paths"))
            })
            .collect::<Result<_, _>>()?,
    };
    let client_certificates = match doc
        .get(TLS)
        .and_then(Item::as_table)
        .and_then(|table| table.get(CLIENT_CERTIFICATES))
    {
        None => Vec::new(),
        Some(item) => item
            .as_array_of_tables()
            .ok_or_else(|| format!("{CLIENT_CERTIFICATES} must be a list of tables"))?
            .iter()
            .map(|table| {
                let optional = |key: &str| -> Result<Option<String>, String> {
                    Ok(Some(text(table, key)?).filter(|value| !value.is_empty()))
                };
                Ok(ClientCertificate {
                    host: HostPattern::new(&text(table, HOST)?),
                    certificate: PathBuf::from(text(table, CERTIFICATE)?),
                    key: optional(KEY)?.map(PathBuf::from),
                    passphrase: optional(PASSPHRASE)?.map(EncryptedPassphrase),
                })
            })
            .collect::<Result<_, String>>()?,
    };
    Ok(AppSettings {
        network: NetworkSettings {
            proxy,
            ca_certificates,
            client_certificates,
        },
    })
}

fn document_of(settings: &AppSettings) -> DocumentMut {
    let mut doc = DocumentMut::new();
    doc.insert("format", value(FORMAT));
    doc.insert("version", value(CURRENT_VERSION));
    let mut proxy = Table::new();
    match &settings.network.proxy {
        ProxySetting::System => {
            proxy.insert(MODE, value(MODE_SYSTEM));
        }
        ProxySetting::None => {
            proxy.insert(MODE, value(MODE_NONE));
        }
        ProxySetting::Manual { url, no_proxy } => {
            proxy.insert(MODE, value(MODE_MANUAL));
            proxy.insert(URL, value(url.as_str()));
            proxy.insert(NO_PROXY, value(no_proxy.as_str()));
        }
    }
    doc.insert(PROXY, Item::Table(proxy));
    let certificates: Array = settings
        .network
        .ca_certificates
        .iter()
        .map(|path| path.to_string_lossy().into_owned())
        .collect();
    let mut tls = Table::new();
    tls.insert(CA_CERTIFICATES, value(certificates));
    let mut clients = ArrayOfTables::new();
    for certificate in &settings.network.client_certificates {
        let mut table = Table::new();
        table.insert(HOST, value(certificate.host.as_str()));
        table.insert(
            CERTIFICATE,
            value(certificate.certificate.to_string_lossy().into_owned()),
        );
        if let Some(key) = &certificate.key {
            table.insert(KEY, value(key.to_string_lossy().into_owned()));
        }
        if let Some(passphrase) = &certificate.passphrase {
            table.insert(PASSPHRASE, value(passphrase.0.as_str()));
        }
        clients.push(table);
    }
    if !clients.is_empty() {
        tls.insert(CLIENT_CERTIFICATES, Item::ArrayOfTables(clients));
    }
    doc.insert(TLS, Item::Table(tls));
    doc
}

fn now_ms() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |since| since.as_millis())
}

#[cfg(test)]
#[path = "tests/app_settings.rs"]
mod tests;

//! The app's own settings: loaded and applied to the HTTP sender when the
//! app starts, shown and saved by the settings dialog. Settings are applied
//! before they are saved, so settings that cannot work are never kept.

use base64::Engine as _;
use base64::engine::general_purpose::STANDARD as BASE64;

use super::App;
use crate::command::{CommandAck, QueryResult};
use crate::error::AppError;
use crate::ports::{
    BrowserLogin, CollectionStore, HistoryStore, HttpSender, ScriptAssist, ScriptRunner,
    SecretError, SecretValues, SecretVault, SettingsStore,
};
use crate::settings::{ClientIdentity, EncryptedPassphrase, NetworkSettings};
use crate::view::SettingsView;

/// The key of the passphrase in its encrypted values.
const PASSPHRASE: &str = "passphrase";

impl<H, S, V, B, R, A, Y, Z> App<H, S, V, B, R, A, Y, Z>
where
    H: HttpSender,
    S: CollectionStore,
    V: SecretVault,
    B: BrowserLogin,
    R: ScriptRunner,
    A: ScriptAssist,
    Y: HistoryStore,
    Z: SettingsStore,
{
    /// Applies the saved settings to the sender (call once at start). If
    /// they cannot be read or applied, requests go out with the defaults and
    /// the settings dialog says why.
    ///
    /// # Errors
    /// [`AppError::Settings`] with the reason.
    pub fn apply_saved_settings(&self) -> Result<(), AppError> {
        let result = self
            .inner
            .settings
            .load()
            .map_err(AppError::from)
            .and_then(|settings| self.configure(&settings.network));
        *self.settings_problem() = result.as_ref().err().map(AppError::user_text);
        result
    }

    pub(super) fn query_settings(&self) -> Result<QueryResult, AppError> {
        let settings = self.inner.settings.load()?;
        Ok(QueryResult::Settings {
            settings: SettingsView::from_settings(&settings),
            problem: self.settings_problem().clone(),
        })
    }

    pub(super) fn save_settings(&self, view: &SettingsView) -> Result<CommandAck, AppError> {
        let mut settings = view.to_settings();
        let entries = view
            .client_certificates
            .iter()
            .filter(|entry| !entry.host.trim().is_empty() || !entry.certificate.trim().is_empty());
        for (certificate, entry) in settings.network.client_certificates.iter_mut().zip(entries) {
            match entry.new_passphrase.as_deref() {
                None => {}
                Some("") => certificate.passphrase = None,
                Some(passphrase) => {
                    certificate.passphrase = Some(self.encrypt_passphrase(passphrase)?);
                }
            }
        }
        self.configure(&settings.network)?;
        self.inner.settings.save(&settings)?;
        *self.settings_problem() = None;
        Ok(CommandAck::SettingsSaved)
    }

    /// Applies `network` to the sender, the passphrases decrypted.
    fn configure(&self, network: &NetworkSettings) -> Result<(), AppError> {
        let identities = network
            .client_certificates
            .iter()
            .map(|certificate| {
                Ok(ClientIdentity {
                    host: certificate.host.clone(),
                    certificate: certificate.certificate.clone(),
                    key: certificate.key.clone(),
                    passphrase: certificate
                        .passphrase
                        .as_ref()
                        .map(|encrypted| self.decrypt_passphrase(encrypted))
                        .transpose()?,
                })
            })
            .collect::<Result<Vec<_>, AppError>>()?;
        Ok(self.inner.http.configure(network, &identities)?)
    }

    /// Encrypts a passphrase for the user's own key only.
    fn encrypt_passphrase(&self, passphrase: &str) -> Result<EncryptedPassphrase, AppError> {
        let vault = &self.inner.vault;
        let values = SecretValues::from([(PASSPHRASE.to_owned(), passphrase.to_owned())]);
        let bytes = vault.encrypt(&values, &[vault.own_recipient()?])?;
        Ok(EncryptedPassphrase(BASE64.encode(bytes)))
    }

    fn decrypt_passphrase(&self, encrypted: &EncryptedPassphrase) -> Result<String, AppError> {
        let bytes = BASE64
            .decode(encrypted.0.trim())
            .map_err(|error| SecretError::Damaged(error.to_string()))?;
        let mut values = self.inner.vault.decrypt(&bytes)?;
        Ok(values
            .remove(PASSPHRASE)
            .ok_or_else(|| SecretError::Damaged(format!("no {PASSPHRASE}")))?)
    }
}

#[cfg(test)]
#[path = "../tests/app_settings.rs"]
mod tests;

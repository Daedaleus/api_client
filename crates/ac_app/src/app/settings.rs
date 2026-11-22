//! The app's own settings: loaded and applied to the HTTP sender when the
//! app starts, shown and saved by the settings dialog. Settings are applied
//! before they are saved, so settings that cannot work are never kept.

use super::App;
use crate::command::{CommandAck, QueryResult};
use crate::error::AppError;
use crate::ports::{
    BrowserLogin, CollectionStore, HistoryStore, HttpSender, ScriptAssist, ScriptRunner,
    SecretVault, SettingsError, SettingsStore,
};
use crate::view::SettingsView;

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
            .and_then(|settings| self.inner.http.configure(&settings.network));
        *self.settings_problem() = result.as_ref().err().map(SettingsError::user_text);
        Ok(result?)
    }

    pub(super) fn query_settings(&self) -> Result<QueryResult, AppError> {
        let settings = self.inner.settings.load()?;
        Ok(QueryResult::Settings {
            settings: SettingsView::from_settings(&settings),
            problem: self.settings_problem().clone(),
        })
    }

    pub(super) fn save_settings(&self, view: &SettingsView) -> Result<CommandAck, AppError> {
        let settings = view.to_settings();
        self.inner.http.configure(&settings.network)?;
        self.inner.settings.save(&settings)?;
        *self.settings_problem() = None;
        Ok(CommandAck::SettingsSaved)
    }
}

#[cfg(test)]
#[path = "../tests/app_settings.rs"]
mod tests;

//! Keeping the app's own settings (see [`crate::settings`]).

use std::sync::Arc;

use crate::settings::AppSettings;
use crate::text::{UserText, keys};

/// Reads and writes the app's settings.
pub trait SettingsStore: Send + Sync + 'static {
    /// The saved settings; the defaults if none were saved yet.
    ///
    /// # Errors
    /// If the settings cannot be read or are damaged (they are then never
    /// overwritten).
    fn load(&self) -> Result<AppSettings, SettingsError>;

    /// Saves `settings`.
    ///
    /// # Errors
    /// If they cannot be written.
    fn save(&self, settings: &AppSettings) -> Result<(), SettingsError>;
}

impl<T: SettingsStore> SettingsStore for Arc<T> {
    fn load(&self) -> Result<AppSettings, SettingsError> {
        T::load(self)
    }

    fn save(&self, settings: &AppSettings) -> Result<(), SettingsError> {
        T::save(self, settings)
    }
}

/// Why settings could not be read, written or applied.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("{0}")]
pub struct SettingsError(pub String);

impl SettingsError {
    /// The text to show the user.
    #[must_use]
    pub fn user_text(&self) -> UserText {
        UserText::new(keys::SETTINGS).with("detail", self.0.clone())
    }
}

/// Settings in memory only (the command line runner, which takes no
/// settings from the desktop app).
#[derive(Debug, Default)]
pub struct DefaultSettings;

impl SettingsStore for DefaultSettings {
    fn load(&self) -> Result<AppSettings, SettingsError> {
        Ok(AppSettings::default())
    }

    fn save(&self, _settings: &AppSettings) -> Result<(), SettingsError> {
        Ok(())
    }
}

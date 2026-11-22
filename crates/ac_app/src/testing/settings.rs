//! Test double for the app's settings.

use std::sync::Mutex;

use super::lock;
use crate::ports::{SettingsError, SettingsStore};
use crate::settings::AppSettings;

/// A [`SettingsStore`] in memory; [`MemorySettings::failing`] makes every
/// call fail.
#[derive(Debug, Default)]
pub struct MemorySettings {
    settings: Mutex<Option<AppSettings>>,
    fail: Mutex<Option<String>>,
}

impl MemorySettings {
    /// No settings saved yet.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// What was saved last, if anything.
    #[must_use]
    pub fn saved(&self) -> Option<AppSettings> {
        lock(&self.settings).clone()
    }

    /// From now on every call fails with `message`.
    pub fn failing(&self, message: &str) {
        *lock(&self.fail) = Some(message.to_owned());
    }

    fn check(&self) -> Result<(), SettingsError> {
        lock(&self.fail)
            .clone()
            .map_or(Ok(()), |message| Err(SettingsError(message)))
    }
}

impl SettingsStore for MemorySettings {
    fn load(&self) -> Result<AppSettings, SettingsError> {
        self.check()?;
        Ok(lock(&self.settings).clone().unwrap_or_default())
    }

    fn save(&self, settings: &AppSettings) -> Result<(), SettingsError> {
        self.check()?;
        *lock(&self.settings) = Some(settings.clone());
        Ok(())
    }
}

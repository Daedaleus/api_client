//! Secret variable values (ADR S4): decrypted only for sending, encrypted
//! again when an environment with changed secrets is saved. The UI never
//! receives them.

use std::collections::BTreeSet;
use std::path::Path;

use ac_domain::environment::{Environment, UnresolvedVariable};

use super::App;
use crate::error::AppError;
use crate::ports::{
    BrowserLogin, CollectionStore, HttpSender, ScriptRunner, SecretError, SecretValues, SecretVault,
};
use crate::ports::{HistoryStore, ScriptAssist, SettingsStore};

/// An environment for sending, with the secret values that could be
/// decrypted.
pub(super) struct Unlocked {
    pub environment: Environment,
    /// Why the secrets could not be decrypted (plain variables still work).
    pub locked: Option<SecretError>,
}

impl<
    H: HttpSender,
    S: CollectionStore,
    V: SecretVault,
    B: BrowserLogin,
    R: ScriptRunner,
    A: ScriptAssist,
    Y: HistoryStore,
    Z: SettingsStore,
> App<H, S, V, B, R, A, Y, Z>
{
    /// Fills in the secret values of `environment`, whose file is `file`.
    pub(super) fn unlock(
        &self,
        file: &Path,
        environment: Environment,
    ) -> Result<Unlocked, AppError> {
        let mut unlocked = Unlocked {
            environment,
            locked: None,
        };
        if !unlocked.environment.has_secrets() {
            return Ok(unlocked);
        }
        let Some(data) = self.inner.store.read_secrets(file)? else {
            return Ok(unlocked);
        };
        match self.inner.vault.decrypt(&data) {
            Ok(values) => fill(&mut unlocked.environment, &values),
            Err(reason) => unlocked.locked = Some(reason),
        }
        Ok(unlocked)
    }

    /// Stores the secrets of `edited` next to `file`. A secret with an empty
    /// value keeps its stored value; `stored` is the environment as it is on
    /// disk. Unchanged secrets are not rewritten (encrypting twice never
    /// gives the same bytes, which would be noise in git).
    pub(super) fn save_secrets(
        &self,
        root: &Path,
        file: &Path,
        stored: &Environment,
        edited: &Environment,
    ) -> Result<(), AppError> {
        let secrets = edited.variables.iter().filter(|variable| variable.secret);
        let new_values = secrets.clone().any(|variable| !variable.value.is_empty());
        if !new_values && secret_names(stored) == secret_names(edited) {
            return Ok(());
        }
        let mut kept = if secrets.clone().any(|variable| variable.value.is_empty()) {
            self.stored_secrets(file)?
        } else {
            SecretValues::new()
        };
        let values: SecretValues = secrets
            .filter_map(|variable| {
                let name = variable.name.trim();
                let value = if variable.value.is_empty() {
                    kept.remove(name)?
                } else {
                    variable.value.clone()
                };
                Some((name.to_owned(), value))
            })
            .collect();
        if values.is_empty() {
            return Ok(self.inner.store.write_secrets(file, None)?);
        }
        let data = self.inner.vault.encrypt(&values, &self.recipients(root)?)?;
        Ok(self.inner.store.write_secrets(file, Some(&data))?)
    }

    fn stored_secrets(&self, file: &Path) -> Result<SecretValues, AppError> {
        match self.inner.store.read_secrets(file)? {
            Some(data) => Ok(self.inner.vault.decrypt(&data)?),
            None => Ok(SecretValues::new()),
        }
    }

    /// Who secrets of the collection are encrypted for; the first time only
    /// the user, who is then written to the recipients list.
    fn recipients(&self, root: &Path) -> Result<Vec<String>, AppError> {
        let recipients = self.inner.store.recipients(root)?;
        if !recipients.is_empty() {
            return Ok(recipients);
        }
        let own = vec![self.inner.vault.own_recipient()?];
        self.inner.store.write_recipients(root, &own)?;
        Ok(own)
    }
}

fn fill(environment: &mut Environment, values: &SecretValues) {
    for variable in &mut environment.variables {
        if variable.secret
            && let Some(value) = values.get(variable.name.trim())
        {
            variable.value.clone_from(value);
        }
    }
}

fn secret_names(environment: &Environment) -> BTreeSet<&str> {
    environment
        .variables
        .iter()
        .filter(|variable| variable.secret)
        .map(|variable| variable.name.trim())
        .collect()
}

/// A missing value for a secret is reported as such, with the reason it
/// could not be decrypted.
pub(super) fn explain(error: AppError, unlocked: Option<&Unlocked>) -> AppError {
    let AppError::UnresolvedVariable(UnresolvedVariable(name)) = &error else {
        return error;
    };
    let Some(unlocked) = unlocked else {
        return error;
    };
    let is_secret = unlocked
        .environment
        .variables
        .iter()
        .any(|variable| variable.secret && variable.enabled && variable.name.trim() == name);
    if !is_secret {
        return error;
    }
    AppError::SecretUnavailable {
        name: name.clone(),
        reason: unlocked.locked.clone().unwrap_or(SecretError::NoKey),
    }
}

#[cfg(test)]
#[path = "../tests/app_secrets.rs"]
mod tests;

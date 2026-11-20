//! Environments of open collections: creating and saving them, and their
//! variables for sending. Deleting goes through `delete_item`.

use std::collections::HashMap;

use ac_domain::environment::Environment;

use super::App;
use super::collections::{item_id_from, location, writable};
use super::secrets::Unlocked;
use crate::command::CommandAck;
use crate::error::AppError;
use crate::ports::{CollectionStore, HttpSender, OpenedCollection, SecretVault};
use crate::view::collection::non_empty;
use crate::view::{EnvironmentDraft, EnvironmentRef};

impl<H: HttpSender, S: CollectionStore, V: SecretVault> App<H, S, V> {
    pub(super) fn create_environment(
        &self,
        collection_id: &str,
        name: &str,
    ) -> Result<CommandAck, AppError> {
        let name = non_empty(name)?;
        let opened = self.current(collection_id)?;
        let environment = Environment::new(name);
        self.inner
            .store
            .create_environment(&opened.root, &environment)?;
        self.reload(&opened.root, Some(environment.id))
    }

    pub(super) fn save_environment(
        &self,
        collection_id: &str,
        draft: &EnvironmentDraft,
    ) -> Result<CommandAck, AppError> {
        let opened = self.current(collection_id)?;
        let environment = draft.to_environment()?;
        let stored = find(&opened, environment.id)?;
        let file = writable(&opened, environment.id)?;
        self.save_secrets(&opened.root, file, stored, &environment)?;
        self.inner
            .store
            .save_environment(file, &without_secret_values(environment))?;
        self.reload(&opened.root, None)
    }

    /// The environment to send with (`None` = no environment), with its
    /// secrets decrypted if possible.
    pub(super) fn environment(
        &self,
        reference: Option<&EnvironmentRef>,
    ) -> Result<Option<Unlocked>, AppError> {
        let Some(reference) = reference else {
            return Ok(None);
        };
        let opened = self.current(&reference.collection_id)?;
        let id = item_id_from(&reference.environment_id)?;
        let environment = find(&opened, id)?.clone();
        let file = &location(&opened, id)?.path;
        self.unlock(file, environment).map(Some)
    }
}

fn find(
    opened: &OpenedCollection,
    id: ac_domain::collection::ItemId,
) -> Result<&Environment, AppError> {
    opened
        .environments
        .iter()
        .find(|environment| environment.id == id)
        .ok_or(AppError::UnknownItem)
}

/// Secret values never go to the store in plain text.
fn without_secret_values(mut environment: Environment) -> Environment {
    for variable in &mut environment.variables {
        if variable.secret {
            variable.value.clear();
        }
    }
    environment
}

/// The values to resolve `{{variables}}` with; none without an environment.
pub(super) fn values(environment: Option<&Environment>) -> HashMap<&str, &str> {
    environment.map_or_default(Environment::values)
}

#[cfg(test)]
#[path = "../tests/app_environments.rs"]
mod tests;

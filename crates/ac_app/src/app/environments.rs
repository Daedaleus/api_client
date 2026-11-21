//! Environments of open collections: creating and saving them, and their
//! variables for sending. Deleting goes through `delete_item`.

use ac_domain::auth::{Auth, effective};
use ac_domain::collection::ItemId;
use ac_domain::environment::{Environment, Variable};
use ac_domain::scope::Scope;
use ac_domain::tls::{CertificateCheck, checks_certificates};

use super::App;
use super::collections::{item_id_from, location, writable};
use super::secrets::Unlocked;
use crate::command::CommandAck;
use crate::error::AppError;
use crate::ports::{CollectionStore, HttpSender, OpenedCollection, SecretVault};
use crate::view::collection::non_empty;
use crate::view::{EnvironmentDraft, SendContext};

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

    /// The collection, request and environment a request is sent in, with
    /// the environment's secrets decrypted if possible.
    pub(super) fn context(
        &self,
        context: Option<&SendContext>,
    ) -> Result<Option<Context>, AppError> {
        let Some(context) = context else {
            return Ok(None);
        };
        let opened = self.current(&context.collection_id)?;
        let request = context
            .request_id
            .as_deref()
            .map(item_id_from)
            .transpose()?;
        let unlocked = match context.environment_id.as_deref() {
            None => None,
            Some(id) => {
                let id = item_id_from(id)?;
                let environment = find(&opened, id)?.clone();
                let file = &location(&opened, id)?.path;
                Some(self.unlock(file, environment)?)
            }
        };
        Ok(Some(Context {
            opened,
            request,
            unlocked,
        }))
    }
}

/// Everything around a request being sent.
pub(super) struct Context {
    opened: OpenedCollection,
    request: Option<ItemId>,
    pub unlocked: Option<Unlocked>,
}

impl Context {
    /// Whether the certificate is checked: `own`, or inherited from the
    /// folders around the request and the collection.
    pub fn checks_certificates(&self, own: CertificateCheck) -> bool {
        let collection = &self.opened.collection;
        match self.request {
            Some(request) => collection.checks_certificates_for(request, own),
            None => checks_certificates([own, collection.certificate_check]),
        }
    }

    /// The collection folder: relative file paths start here.
    pub fn root(&self) -> &std::path::Path {
        &self.opened.root
    }

    /// The authentication that applies: `own`, or inherited from the
    /// folders around the request and the collection.
    pub fn auth(&self, own: &Auth) -> Auth {
        let collection = &self.opened.collection;
        match self.request {
            Some(request) => collection.auth_for(request, own),
            None => effective([own, &collection.auth]).clone(),
        }
    }

    /// The variable lists for the request, weakest first; `own` are the
    /// request's own variables as edited (see [`ac_domain::scope`]).
    pub fn scope<'a>(&'a self, own: &'a [Variable]) -> Scope<'a> {
        let collection = &self.opened.collection;
        let mut scope = Scope::new().then(&collection.variables);
        if let Some(unlocked) = &self.unlocked {
            scope = scope.then(&unlocked.environment.variables);
        }
        if let Some(request) = self.request {
            for folder in collection.folders_around(request) {
                scope = scope.then(&folder.variables);
            }
        }
        scope.then(own)
    }
}

fn find(opened: &OpenedCollection, id: ItemId) -> Result<&Environment, AppError> {
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

#[cfg(test)]
#[path = "../tests/app_environments.rs"]
mod tests;

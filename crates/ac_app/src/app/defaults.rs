//! What a collection and its folders pass on to their requests: variables,
//! authentication, certificate check and scripts (a request's own are saved
//! with the request).

use ac_domain::collection::{Item, RequestDefaults};

use super::App;
use super::collections::{item_id_from, writable};
use crate::command::CommandAck;
use crate::error::AppError;
use crate::ports::{BrowserLogin, CollectionStore, HttpSender, ScriptRunner, SecretVault};
use crate::ports::{HistoryStore, ScriptAssist, SettingsStore};
use crate::view::{AuthDraft, CertificateCheckView, ScriptsDraft, TransportDraft, VariableDraft};

/// What the defaults dialog saves for the collection or a folder.
pub(super) struct DefaultsDraft<'a> {
    pub variables: &'a [VariableDraft],
    pub auth: &'a AuthDraft,
    pub certificate_check: CertificateCheckView,
    pub scripts: &'a ScriptsDraft,
    pub docs: &'a str,
    pub transport: TransportDraft,
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
    pub(super) fn save_defaults(
        &self,
        collection_id: &str,
        item_id: &str,
        draft: &DefaultsDraft<'_>,
    ) -> Result<CommandAck, AppError> {
        let opened = self.current(collection_id)?;
        let id = item_id_from(item_id)?;
        let defaults = RequestDefaults {
            scripts: draft.scripts.to_scripts(),
            transport: draft.transport.to_settings(),
            variables: VariableDraft::to_plain_variables(draft.variables),
            auth: draft.auth.to_auth(),
            certificate_check: draft.certificate_check.to_check(),
            docs: draft.docs.to_owned(),
        };
        if id == opened.collection.id {
            writable(&opened, id)?;
            self.inner
                .store
                .save_collection_defaults(&opened.root, &defaults)?;
        } else {
            let folder = match opened.collection.find(id) {
                Some(Item::Folder(folder)) => folder,
                Some(Item::Request(_)) => return Err(AppError::NotAFolder),
                None => return Err(AppError::UnknownItem),
            };
            let dir = writable(&opened, id)?;
            self.inner
                .store
                .save_folder_defaults(dir, folder.id, &folder.name, &defaults)?;
        }
        self.reload(&opened.root, None)
    }
}

#[cfg(test)]
#[path = "../tests/app_defaults.rs"]
mod tests;

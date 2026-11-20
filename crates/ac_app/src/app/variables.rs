//! Variables of a collection and of its folders (request variables are
//! saved with the request).

use ac_domain::collection::Item;

use super::App;
use super::collections::{item_id_from, writable};
use crate::command::CommandAck;
use crate::error::AppError;
use crate::ports::{CollectionStore, HttpSender, SecretVault};
use crate::view::VariableDraft;

impl<H: HttpSender, S: CollectionStore, V: SecretVault> App<H, S, V> {
    pub(super) fn save_variables(
        &self,
        collection_id: &str,
        item_id: &str,
        drafts: &[VariableDraft],
    ) -> Result<CommandAck, AppError> {
        let opened = self.current(collection_id)?;
        let id = item_id_from(item_id)?;
        let variables = VariableDraft::to_plain_variables(drafts);
        if id == opened.collection.id {
            writable(&opened, id)?;
            self.inner
                .store
                .save_collection_variables(&opened.root, &variables)?;
        } else {
            let folder = match opened.collection.find(id) {
                Some(Item::Folder(folder)) => folder,
                Some(Item::Request(_)) => return Err(AppError::NotAFolder),
                None => return Err(AppError::UnknownItem),
            };
            let dir = writable(&opened, id)?;
            self.inner
                .store
                .save_folder_variables(dir, folder.id, &folder.name, &variables)?;
        }
        self.reload(&opened.root, None)
    }
}

#[cfg(test)]
#[path = "../tests/app_variables.rs"]
mod tests;

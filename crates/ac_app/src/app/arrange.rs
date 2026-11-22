//! Duplicating and moving requests and folders within a collection.
//!
//! A copy gets new ids throughout (a folder with everything in it) and is
//! placed right after the original. Moving reorders within a folder or puts
//! the item into another folder of the same collection.

use std::path::Path;

use ac_domain::collection::{Collection, Folder, Item, ItemId, Request, RequestDefaults};

use super::App;
use super::collections::{item_id_from, writable};
use crate::command::CommandAck;
use crate::error::AppError;
use crate::ports::{
    BrowserLogin, CollectionStore, HistoryStore, HttpSender, ScriptAssist, ScriptRunner,
    SecretVault, SettingsStore,
};
use crate::view::collection::non_empty;

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
    /// Copies the request or folder `item_id` as `name`, right after it.
    pub(super) fn duplicate_item(
        &self,
        collection_id: &str,
        item_id: &str,
        name: &str,
    ) -> Result<CommandAck, AppError> {
        let name = non_empty(name)?;
        let opened = self.current(collection_id)?;
        let id = item_id_from(item_id)?;
        let item = opened.collection.find(id).ok_or(AppError::UnknownItem)?;
        let (parent, index) = parent_of(&opened.collection, id).ok_or(AppError::UnknownItem)?;
        let dir = writable(&opened, parent)?;
        let (path, copy) = match item {
            Item::Request(request) => {
                let copy = Request {
                    id: ItemId::new(),
                    name: name.to_owned(),
                    ..request.clone()
                };
                (self.inner.store.create_request(dir, &copy)?, copy.id)
            }
            Item::Folder(folder) => self.copy_folder(folder, dir, name)?,
        };
        self.inner.store.move_item(&path, dir, index + 1)?;
        self.reload(&opened.root, Some(copy))
    }

    /// Moves `item_id` into the collection or folder `parent_id`, at
    /// `index` among its items.
    pub(super) fn move_item(
        &self,
        collection_id: &str,
        item_id: &str,
        parent_id: &str,
        index: usize,
    ) -> Result<CommandAck, AppError> {
        let opened = self.current(collection_id)?;
        let id = item_id_from(item_id)?;
        let target = item_id_from(parent_id)?;
        if opened.collection.find(id).is_none() {
            return Err(AppError::UnknownItem);
        }
        let into_itself = target == id
            || opened
                .collection
                .folders_around(target)
                .iter()
                .any(|folder| folder.id == id);
        if into_itself {
            return Err(AppError::MoveIntoItself);
        }
        let is_folder = target == opened.collection.id
            || matches!(opened.collection.find(target), Some(Item::Folder(_)));
        if !is_folder {
            return Err(AppError::NotAFolder);
        }
        let path = writable(&opened, id)?.clone();
        let to_dir = writable(&opened, target)?;
        self.inner.store.move_item(&path, to_dir, index)?;
        self.reload(&opened.root, None)
    }

    /// Copies `folder` with new ids into `parent` as `name`, keeping the
    /// order of its items; returns the new folder's path and id.
    fn copy_folder(
        &self,
        folder: &Folder,
        parent: &Path,
        name: &str,
    ) -> Result<(std::path::PathBuf, ItemId), AppError> {
        let store = &self.inner.store;
        let (dir, id) = store.create_folder(parent, name)?;
        store.save_folder_defaults(
            &dir,
            id,
            name,
            &RequestDefaults {
                transport: folder.transport,
                variables: folder.variables.clone(),
                auth: folder.auth.clone(),
                certificate_check: folder.certificate_check,
                scripts: folder.scripts.clone(),
            },
        )?;
        for (position, item) in folder.items.iter().enumerate() {
            let path = match item {
                Item::Request(request) => store.create_request(
                    &dir,
                    &Request {
                        id: ItemId::new(),
                        ..request.clone()
                    },
                )?,
                Item::Folder(inner) => self.copy_folder(inner, &dir, &inner.name)?.0,
            };
            store.move_item(&path, &dir, position)?;
        }
        Ok((dir, id))
    }
}

/// The collection or folder holding `id`, and its position there.
fn parent_of(collection: &Collection, id: ItemId) -> Option<(ItemId, usize)> {
    let (parent, items) = match collection.folders_around(id).last() {
        Some(folder) => (folder.id, &folder.items),
        None => (collection.id, &collection.items),
    };
    let index = items.iter().position(|item| item.id() == id)?;
    Some((parent, index))
}

#[cfg(test)]
#[path = "../tests/app_arrange.rs"]
mod tests;

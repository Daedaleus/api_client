//! Open collections and changes to them.
//!
//! Every command reads the collection anew from the store, acts on the
//! files the store reports, and answers with the tree as it is afterwards.
//! So the UI always shows what is on disk, including changes made outside
//! the app (an editor, `git pull`).

use std::path::{Path, PathBuf};

use ac_domain::collection::{Item, ItemId, Request};

use super::App;
use crate::command::{CommandAck, QueryResult};
use crate::error::AppError;
use crate::ports::{
    BrowserLogin, CollectionStore, HttpSender, Location, OpenedCollection, ScriptRunner,
    SecretVault, StoreError, StoreErrorKind,
};
use crate::ports::{HistoryStore, ScriptAssist, SettingsStore};
use crate::view::collection::non_empty;
use crate::view::{CollectionView, RequestDraft};

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
    pub(super) fn open_collection(&self, path: &str) -> Result<CommandAck, AppError> {
        let opened = self.inner.store.open(Path::new(path))?;
        self.remember(&opened);
        Ok(self.changed(&opened, None))
    }

    pub(super) fn create_collection(&self, path: &str, name: &str) -> Result<CommandAck, AppError> {
        let name = non_empty(name)?;
        let opened = self.inner.store.create_collection(Path::new(path), name)?;
        self.remember(&opened);
        Ok(self.changed(&opened, None))
    }

    pub(super) fn close_collection(&self, collection_id: &str) -> Result<CommandAck, AppError> {
        let id = collection_id_from(collection_id)?;
        let root = self.root_of(id)?;
        let mut open = self.open_collections();
        let before = open.len();
        open.retain(|(open_id, _)| *open_id != id);
        if open.len() == before {
            return Err(AppError::UnknownCollection);
        }
        drop(open);
        if self.temporary().remove(&id) {
            self.inner.store.discard_temporary(&root)?;
        }
        self.forget_runtime_variables(&id.to_string());
        Ok(CommandAck::CollectionClosed {
            collection_id: id.to_string(),
        })
    }

    pub(super) fn save_request(
        &self,
        collection_id: &str,
        draft: &RequestDraft,
    ) -> Result<CommandAck, AppError> {
        let opened = self.current(collection_id)?;
        let request = draft.to_request()?;
        let Some(Item::Request(_)) = opened.collection.find(request.id) else {
            return Err(AppError::UnknownItem);
        };
        let file = writable(&opened, request.id)?;
        self.inner.store.save_request(file, &request)?;
        self.reload(&opened.root, None)
    }

    pub(super) fn create_request(
        &self,
        collection_id: &str,
        parent_id: &str,
        name: &str,
    ) -> Result<CommandAck, AppError> {
        let name = non_empty(name)?;
        let opened = self.current(collection_id)?;
        let dir = folder_path(&opened, parent_id)?;
        let request = Request::new(name);
        let _ = self.inner.store.create_request(dir, &request)?;
        self.reload(&opened.root, Some(request.id))
    }

    pub(super) fn create_folder(
        &self,
        collection_id: &str,
        parent_id: &str,
        name: &str,
    ) -> Result<CommandAck, AppError> {
        let name = non_empty(name)?;
        let opened = self.current(collection_id)?;
        let dir = folder_path(&opened, parent_id)?;
        let (_, id) = self.inner.store.create_folder(dir, name)?;
        self.reload(&opened.root, Some(id))
    }

    pub(super) fn rename_item(
        &self,
        collection_id: &str,
        item_id: &str,
        name: &str,
    ) -> Result<CommandAck, AppError> {
        let name = non_empty(name)?;
        let opened = self.current(collection_id)?;
        let id = item_id_from(item_id)?;
        match opened.collection.find(id) {
            Some(Item::Request(request)) => {
                let file = writable(&opened, id)?;
                let renamed = Request {
                    name: name.to_owned(),
                    ..request.clone()
                };
                self.inner.store.save_request(file, &renamed)?;
            }
            Some(Item::Folder(_)) => {
                let dir = writable(&opened, id)?;
                self.inner.store.rename_folder(dir, id, name)?;
            }
            None => return Err(AppError::UnknownItem),
        }
        self.reload(&opened.root, None)
    }

    pub(super) fn delete_item(
        &self,
        collection_id: &str,
        item_id: &str,
    ) -> Result<CommandAck, AppError> {
        let opened = self.current(collection_id)?;
        let id = item_id_from(item_id)?;
        let in_tree = opened.collection.find(id).is_some();
        let is_environment = opened
            .environments
            .iter()
            .any(|environment| environment.id == id);
        if !in_tree && !is_environment {
            return Err(AppError::UnknownItem);
        }
        let path = location(&opened, id)?.path.clone();
        self.inner.store.remove(&path)?;
        self.reload(&opened.root, None)
    }

    pub(super) fn query_collection(&self, collection_id: &str) -> Result<QueryResult, AppError> {
        let opened = self.current(collection_id)?;
        Ok(QueryResult::Collection {
            collection: self.view(&opened),
        })
    }

    pub(super) fn query_request(
        &self,
        collection_id: &str,
        request_id: &str,
    ) -> Result<QueryResult, AppError> {
        let opened = self.current(collection_id)?;
        let id = item_id_from(request_id)?;
        let Some(Item::Request(request)) = opened.collection.find(id) else {
            return Err(AppError::UnknownItem);
        };
        let read_only = location(&opened, id)?.read_only;
        Ok(QueryResult::Request {
            request: RequestDraft::from_request(request, read_only),
        })
    }

    /// The open collection, read anew from disk.
    pub(super) fn current(&self, collection_id: &str) -> Result<OpenedCollection, AppError> {
        let id = collection_id_from(collection_id)?;
        let root = self
            .open_collections()
            .iter()
            .find(|(open_id, _)| *open_id == id)
            .map(|(_, root)| root.clone())
            .ok_or(AppError::UnknownCollection)?;
        Ok(self.inner.store.open(&root)?)
    }

    /// Answers with the collection as it is on disk after a change.
    pub(super) fn reload(
        &self,
        root: &Path,
        created: Option<ItemId>,
    ) -> Result<CommandAck, AppError> {
        let opened = self.inner.store.open(root)?;
        Ok(self.changed(&opened, created))
    }

    /// The view of an opened collection, marked if it is unsaved.
    pub(super) fn view(&self, opened: &OpenedCollection) -> CollectionView {
        let mut view = CollectionView::from_opened(opened);
        view.temporary = self.temporary().contains(&opened.collection.id);
        view
    }

    fn changed(&self, opened: &OpenedCollection, created: Option<ItemId>) -> CommandAck {
        CommandAck::CollectionChanged {
            collection: Box::new(self.view(opened)),
            created_id: created.map(|id| id.to_string()),
        }
    }

    /// Saves an unsaved collection by moving it to `path`.
    pub(super) fn save_collection_as(
        &self,
        collection_id: &str,
        path: &str,
    ) -> Result<CommandAck, AppError> {
        let id = collection_id_from(collection_id)?;
        let from = self.root_of(id)?;
        if !self.temporary().contains(&id) {
            return Err(AppError::AlreadySaved);
        }
        let to = Path::new(path).to_owned();
        self.inner.store.move_collection(&from, &to)?;
        for entry in self.open_collections().iter_mut() {
            if entry.0 == id {
                entry.1.clone_from(&to);
            }
        }
        self.temporary().remove(&id);
        self.reload(&to, None)
    }

    /// Deletes every unsaved collection (when the app ends); errors are
    /// returned per collection, the others are deleted anyway.
    #[must_use = "errors say which unsaved collections stayed on disk"]
    pub fn discard_unsaved(&self) -> Vec<AppError> {
        let ids: Vec<ItemId> = self.temporary().drain().collect();
        ids.into_iter()
            .filter_map(|id| {
                let root = self.root_of(id).ok()?;
                self.inner
                    .store
                    .discard_temporary(&root)
                    .err()
                    .map(AppError::from)
            })
            .collect()
    }

    fn root_of(&self, id: ItemId) -> Result<PathBuf, AppError> {
        self.open_collections()
            .iter()
            .find(|(open_id, _)| *open_id == id)
            .map(|(_, root)| root.clone())
            .ok_or(AppError::UnknownCollection)
    }

    /// Adds the collection to the open ones (once).
    pub(super) fn remember(&self, opened: &OpenedCollection) {
        let mut open = self.open_collections();
        let id = opened.collection.id;
        if !open.iter().any(|(open_id, _)| *open_id == id) {
            open.push((id, opened.root.clone()));
        }
    }
}

fn collection_id_from(text: &str) -> Result<ItemId, AppError> {
    text.parse().map_err(|_| AppError::UnknownCollection)
}

pub(super) fn item_id_from(text: &str) -> Result<ItemId, AppError> {
    text.parse().map_err(|_| AppError::UnknownItem)
}

pub(super) fn location(opened: &OpenedCollection, id: ItemId) -> Result<&Location, AppError> {
    opened.locations.get(&id).ok_or(AppError::UnknownItem)
}

/// The path of an item that may be written.
pub(super) fn writable(opened: &OpenedCollection, id: ItemId) -> Result<&PathBuf, AppError> {
    let location = location(opened, id)?;
    if location.read_only {
        return Err(AppError::Store(StoreError {
            kind: StoreErrorKind::ReadOnly,
            path: location.path.clone(),
            detail: "read-only".into(),
        }));
    }
    Ok(&location.path)
}

/// The directory of the collection or of a folder in it, for new items.
fn folder_path<'a>(opened: &'a OpenedCollection, parent_id: &str) -> Result<&'a Path, AppError> {
    let id = item_id_from(parent_id)?;
    let is_folder =
        id == opened.collection.id || matches!(opened.collection.find(id), Some(Item::Folder(_)));
    if !is_folder {
        return match opened.collection.find(id) {
            Some(Item::Request(_)) => Err(AppError::NotAFolder),
            _ => Err(AppError::UnknownItem),
        };
    }
    Ok(&location(opened, id)?.path)
}

#[cfg(test)]
#[path = "../tests/app_collections.rs"]
mod tests;

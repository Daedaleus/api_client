//! The [`CollectionStore`] port of `ac_app`, implemented on the files of
//! this crate.

use std::path::Path;

use ac_app::ports::{CollectionStore, Location, OpenedCollection, StoreError, StoreErrorKind};
use ac_domain::collection::{ItemId, Request};
use ac_domain::environment::Environment;

use crate::error::StorageError;
use crate::load::{LoadedCollection, load_collection};
use crate::{create, environment_file, remove, request_file, settings};

/// How removed items are discarded.
pub type Discard = fn(&Path) -> std::io::Result<()>;

/// Collections as folders of TOML files.
#[derive(Debug, Clone, Copy)]
pub struct FileStore {
    discard: Discard,
}

impl Default for FileStore {
    fn default() -> Self {
        Self {
            discard: move_to_trash,
        }
    }
}

impl FileStore {
    /// A store that moves removed items to the system trash.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// A store that discards removed items with `discard` instead (tests
    /// must not touch the real trash).
    #[must_use]
    pub fn with_discard(discard: Discard) -> Self {
        Self { discard }
    }
}

/// The system trash (recycle bin); nothing is deleted for good.
fn move_to_trash(path: &Path) -> std::io::Result<()> {
    trash::delete(path).map_err(|error| std::io::Error::other(error.to_string()))
}

impl CollectionStore for FileStore {
    fn open(&self, root: &Path) -> Result<OpenedCollection, StoreError> {
        load_collection(root).map(opened).map_err(store_error)
    }

    fn create_collection(&self, root: &Path, name: &str) -> Result<OpenedCollection, StoreError> {
        create::create_collection(root, name)
            .map(opened)
            .map_err(store_error)
    }

    fn save_request(&self, file: &Path, request: &Request) -> Result<(), StoreError> {
        request_file::save_request(file, request).map_err(store_error)
    }

    fn create_request(&self, dir: &Path, request: &Request) -> Result<(), StoreError> {
        create::create_request(dir, request)
            .map(|_| ())
            .map_err(store_error)
    }

    fn create_folder(&self, parent: &Path, name: &str) -> Result<ItemId, StoreError> {
        create::create_folder(parent, name)
            .map(|(_, id)| id)
            .map_err(store_error)
    }

    fn rename_folder(&self, dir: &Path, id: ItemId, name: &str) -> Result<(), StoreError> {
        settings::save_folder_settings(dir, id, name).map_err(store_error)
    }

    fn remove(&self, path: &Path) -> Result<(), StoreError> {
        remove::remove_item(path, self.discard).map_err(store_error)
    }

    fn save_environment(&self, file: &Path, environment: &Environment) -> Result<(), StoreError> {
        environment_file::save_environment(file, environment).map_err(store_error)
    }

    fn create_environment(&self, root: &Path, environment: &Environment) -> Result<(), StoreError> {
        environment_file::create_environment(root, environment)
            .map(|_| ())
            .map_err(store_error)
    }
}

fn opened(loaded: LoadedCollection) -> OpenedCollection {
    OpenedCollection {
        root: loaded.root,
        collection: loaded.collection,
        environments: loaded.environments,
        locations: loaded
            .entries
            .into_iter()
            .map(|(id, entry)| {
                let location = Location {
                    path: entry.path,
                    read_only: entry.read_only,
                };
                (id, location)
            })
            .collect(),
        problems: loaded
            .problems
            .into_iter()
            .map(|problem| store_error(problem.error))
            .collect(),
    }
}

fn store_error(error: StorageError) -> StoreError {
    let (kind, path, detail) = match error {
        StorageError::Io { path, source } => (StoreErrorKind::Io, path, source.to_string()),
        StorageError::Toml { path, message } => (StoreErrorKind::Damaged, path, message),
        StorageError::WrongFormat {
            path,
            expected,
            found,
        } => (
            StoreErrorKind::Damaged,
            path,
            format!("expected format {expected:?}, found {found:?}"),
        ),
        StorageError::Invalid {
            path,
            field,
            message,
        } => (StoreErrorKind::Damaged, path, format!("{field}: {message}")),
        StorageError::ReadOnly { path, version } => (
            StoreErrorKind::ReadOnly,
            path,
            format!("format version {version}"),
        ),
        StorageError::NotACollection { path } => (
            StoreErrorKind::NotACollection,
            path,
            "collection.toml missing".into(),
        ),
        StorageError::AlreadyExists { path } => (
            StoreErrorKind::AlreadyExists,
            path,
            "collection exists".into(),
        ),
    };
    StoreError { kind, path, detail }
}

#[cfg(test)]
#[path = "tests/store.rs"]
mod tests;

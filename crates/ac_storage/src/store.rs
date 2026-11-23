//! The [`CollectionStore`] port of `ac_app`, implemented on the files of
//! this crate.

use std::path::{Path, PathBuf};

use ac_app::ports::{
    CollectionStore, Location, OpenedCollection, StoreError, StoreErrorKind, TextFile,
};
use ac_domain::collection::{Collection, ItemId, Request, RequestDefaults};
use ac_domain::environment::Environment;

use crate::error::StorageError;
use crate::format::ENVIRONMENTS_FOLDER;
use crate::load::{LoadedCollection, load_collection};
use crate::{
    arrange, create, environment_file, remove, request_file, secrets_file, settings, temporary,
};

/// How removed items are discarded.
pub type Discard = fn(&Path) -> std::io::Result<()>;

/// Collections as folders of TOML files.
#[derive(Debug, Clone)]
pub struct FileStore {
    discard: Discard,
    /// Where unsaved collections live (see `temporary`).
    temporary_base: PathBuf,
}

impl Default for FileStore {
    fn default() -> Self {
        Self {
            discard: move_to_trash,
            temporary_base: std::env::temp_dir(),
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
        Self {
            discard,
            ..Self::default()
        }
    }

    /// The same store with unsaved collections below `base` instead of the
    /// system's temp folder (tests).
    #[must_use]
    pub fn with_temporary_base(mut self, base: impl Into<PathBuf>) -> Self {
        self.temporary_base = base.into();
        self
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

    fn create_request(&self, dir: &Path, request: &Request) -> Result<PathBuf, StoreError> {
        create::create_request(dir, request).map_err(store_error)
    }

    fn create_folder(&self, parent: &Path, name: &str) -> Result<(PathBuf, ItemId), StoreError> {
        create::create_folder(parent, name).map_err(store_error)
    }

    fn move_item(&self, path: &Path, to_dir: &Path, position: usize) -> Result<(), StoreError> {
        arrange::move_item(path, to_dir, position)
            .map(|_| ())
            .map_err(store_error)
    }

    fn rename_folder(&self, dir: &Path, id: ItemId, name: &str) -> Result<(), StoreError> {
        settings::save_folder_settings(dir, id, name).map_err(store_error)
    }

    /// An environment's secrets go to the trash with it.
    fn remove(&self, path: &Path) -> Result<(), StoreError> {
        remove::remove_item(path, self.discard).map_err(store_error)?;
        if is_environment_file(path) {
            self.discard_secrets(path)?;
        }
        Ok(())
    }

    fn save_environment(&self, file: &Path, environment: &Environment) -> Result<(), StoreError> {
        environment_file::save_environment(file, environment).map_err(store_error)
    }

    fn create_environment(&self, root: &Path, environment: &Environment) -> Result<(), StoreError> {
        environment_file::create_environment(root, environment)
            .map(|_| ())
            .map_err(store_error)
    }

    fn save_collection_defaults(
        &self,
        root: &Path,
        defaults: &RequestDefaults,
    ) -> Result<(), StoreError> {
        settings::save_collection_defaults(root, defaults).map_err(store_error)
    }

    fn save_folder_defaults(
        &self,
        dir: &Path,
        id: ItemId,
        name: &str,
        defaults: &RequestDefaults,
    ) -> Result<(), StoreError> {
        settings::save_folder_defaults(dir, id, name, defaults).map_err(store_error)
    }

    fn import_collection(
        &self,
        root: &Path,
        collection: &Collection,
        environments: &[Environment],
    ) -> Result<OpenedCollection, StoreError> {
        create::write_collection(root, collection, environments)
            .map(opened)
            .map_err(store_error)
    }

    fn read_tree(&self, dir: &Path, extensions: &[&str]) -> Result<Vec<TextFile>, StoreError> {
        let mut files = Vec::new();
        crate::tree::read_tree(dir, dir, extensions, &mut files).map_err(store_error)?;
        Ok(files)
    }

    fn read_text(&self, file: &Path) -> Result<String, StoreError> {
        std::fs::read_to_string(file)
            .map_err(|error| store_error(crate::error::StorageError::io(file, error)))
    }

    fn write_file(&self, file: &Path, bytes: &[u8]) -> Result<(), StoreError> {
        crate::document::write_atomically(file, bytes).map_err(store_error)
    }

    fn read_secrets(&self, environment_file: &Path) -> Result<Option<Vec<u8>>, StoreError> {
        secrets_file::read_secrets(environment_file).map_err(store_error)
    }

    fn write_secrets(
        &self,
        environment_file: &Path,
        data: Option<&[u8]>,
    ) -> Result<(), StoreError> {
        match data {
            Some(data) => secrets_file::write_secrets(environment_file, data).map_err(store_error),
            None => self.discard_secrets(environment_file),
        }
    }

    fn recipients(&self, root: &Path) -> Result<Vec<String>, StoreError> {
        secrets_file::read_recipients(root).map_err(store_error)
    }

    fn write_recipients(&self, root: &Path, recipients: &[String]) -> Result<(), StoreError> {
        secrets_file::write_recipients(root, recipients).map_err(store_error)
    }

    fn temporary_root(&self) -> Result<PathBuf, StoreError> {
        Ok(temporary::temporary_root(&self.temporary_base))
    }

    fn move_collection(&self, from: &Path, to: &Path) -> Result<(), StoreError> {
        temporary::move_collection(from, to).map_err(store_error)
    }

    fn discard_temporary(&self, root: &Path) -> Result<(), StoreError> {
        temporary::discard(&self.temporary_base, root).map_err(store_error)
    }
}

impl FileStore {
    /// Discards the secrets file of an environment, if there is one.
    fn discard_secrets(&self, environment_file: &Path) -> Result<(), StoreError> {
        let path = secrets_file::secrets_path(environment_file);
        if !path.exists() {
            return Ok(());
        }
        (self.discard)(&path)
            .map_err(|error| store_error(crate::error::StorageError::io(&path, error)))
    }
}

fn is_environment_file(path: &Path) -> bool {
    path.parent()
        .and_then(Path::file_name)
        .is_some_and(|name| name == ENVIRONMENTS_FOLDER)
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

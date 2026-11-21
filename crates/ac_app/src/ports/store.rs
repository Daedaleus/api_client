//! Reading and writing collections. Implemented by `ac_storage` (folders of
//! TOML files); the app addresses files only through the paths the store
//! hands out.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use ac_domain::collection::{Collection, ItemId, Request, RequestDefaults};
use ac_domain::environment::Environment;

use crate::text::{UserText, keys};

/// Access to collections on disk.
pub trait CollectionStore: Send + Sync + 'static {
    /// Loads the collection in `root`.
    ///
    /// # Errors
    /// If the collection itself cannot be opened; problems with single files
    /// are listed in [`OpenedCollection::problems`].
    fn open(&self, root: &Path) -> Result<OpenedCollection, StoreError>;

    /// Creates an empty collection named `name` in `root` and loads it.
    ///
    /// # Errors
    /// If `root` already holds a collection or cannot be written.
    fn create_collection(&self, root: &Path, name: &str) -> Result<OpenedCollection, StoreError>;

    /// Writes `request` to its file, changing only what differs.
    ///
    /// # Errors
    /// If the file is damaged, read-only or cannot be written.
    fn save_request(&self, file: &Path, request: &Request) -> Result<(), StoreError>;

    /// Creates a file for `request` in the collection or folder `dir`.
    ///
    /// # Errors
    /// If the file or the folder settings cannot be written.
    fn create_request(&self, dir: &Path, request: &Request) -> Result<(), StoreError>;

    /// Creates a folder named `name` in `parent`; returns its id.
    ///
    /// # Errors
    /// If the folder cannot be created.
    fn create_folder(&self, parent: &Path, name: &str) -> Result<ItemId, StoreError>;

    /// Sets the name of the folder `dir`, keeping its id.
    ///
    /// # Errors
    /// If the folder settings are damaged, read-only or cannot be written.
    fn rename_folder(&self, dir: &Path, id: ItemId, name: &str) -> Result<(), StoreError>;

    /// Moves a request file, folder or environment file to the trash (never
    /// deletes for good).
    ///
    /// # Errors
    /// If it cannot be moved.
    fn remove(&self, path: &Path) -> Result<(), StoreError>;

    /// Writes `environment` to its file, changing only what differs.
    ///
    /// # Errors
    /// If the file is damaged, read-only or cannot be written.
    fn save_environment(&self, file: &Path, environment: &Environment) -> Result<(), StoreError>;

    /// Creates a file for `environment` in the collection `root`.
    ///
    /// # Errors
    /// If the file cannot be written.
    fn create_environment(&self, root: &Path, environment: &Environment) -> Result<(), StoreError>;

    /// Writes what the collection in `root` passes on to its requests
    /// (variables, authentication).
    ///
    /// # Errors
    /// If its settings are damaged, read-only or cannot be written.
    fn save_collection_defaults(
        &self,
        root: &Path,
        defaults: &RequestDefaults,
    ) -> Result<(), StoreError>;

    /// Writes what the folder `dir` passes on to its requests (id and name
    /// keep a new settings file consistent).
    ///
    /// # Errors
    /// If its settings are damaged, read-only or cannot be written.
    fn save_folder_defaults(
        &self,
        dir: &Path,
        id: ItemId,
        name: &str,
        defaults: &RequestDefaults,
    ) -> Result<(), StoreError>;

    /// Writes `collection` as a new collection into `root` (imports) and
    /// loads it.
    ///
    /// # Errors
    /// If `root` already holds a collection or cannot be written.
    fn import_collection(
        &self,
        root: &Path,
        collection: &Collection,
    ) -> Result<OpenedCollection, StoreError>;

    /// The text of a file the user picked to import.
    ///
    /// # Errors
    /// If it cannot be read or is no UTF-8 text.
    fn read_text(&self, file: &Path) -> Result<String, StoreError>;

    /// The encrypted secrets of the environment in `environment_file`;
    /// `None` if it has none.
    ///
    /// # Errors
    /// If they exist but cannot be read.
    fn read_secrets(&self, environment_file: &Path) -> Result<Option<Vec<u8>>, StoreError>;

    /// Replaces the encrypted secrets of an environment; `None` moves them to
    /// the trash (the environment has no secrets any more).
    ///
    /// # Errors
    /// If they cannot be written or moved.
    fn write_secrets(&self, environment_file: &Path, data: Option<&[u8]>)
    -> Result<(), StoreError>;

    /// The public keys secrets in the collection `root` are encrypted for;
    /// empty if none are set up yet.
    ///
    /// # Errors
    /// If the list exists but cannot be read.
    fn recipients(&self, root: &Path) -> Result<Vec<String>, StoreError>;

    /// Sets the recipients of a collection (keeping comments of a list
    /// edited by hand).
    ///
    /// # Errors
    /// If the list cannot be read or written.
    fn write_recipients(&self, root: &Path, recipients: &[String]) -> Result<(), StoreError>;
}

impl<T: CollectionStore> CollectionStore for Arc<T> {
    fn open(&self, root: &Path) -> Result<OpenedCollection, StoreError> {
        T::open(self, root)
    }
    fn create_collection(&self, root: &Path, name: &str) -> Result<OpenedCollection, StoreError> {
        T::create_collection(self, root, name)
    }
    fn save_request(&self, file: &Path, request: &Request) -> Result<(), StoreError> {
        T::save_request(self, file, request)
    }
    fn create_request(&self, dir: &Path, request: &Request) -> Result<(), StoreError> {
        T::create_request(self, dir, request)
    }
    fn create_folder(&self, parent: &Path, name: &str) -> Result<ItemId, StoreError> {
        T::create_folder(self, parent, name)
    }
    fn rename_folder(&self, dir: &Path, id: ItemId, name: &str) -> Result<(), StoreError> {
        T::rename_folder(self, dir, id, name)
    }
    fn remove(&self, path: &Path) -> Result<(), StoreError> {
        T::remove(self, path)
    }
    fn save_environment(&self, file: &Path, environment: &Environment) -> Result<(), StoreError> {
        T::save_environment(self, file, environment)
    }
    fn create_environment(&self, root: &Path, environment: &Environment) -> Result<(), StoreError> {
        T::create_environment(self, root, environment)
    }
    fn save_collection_defaults(
        &self,
        root: &Path,
        defaults: &RequestDefaults,
    ) -> Result<(), StoreError> {
        T::save_collection_defaults(self, root, defaults)
    }
    fn save_folder_defaults(
        &self,
        dir: &Path,
        id: ItemId,
        name: &str,
        defaults: &RequestDefaults,
    ) -> Result<(), StoreError> {
        T::save_folder_defaults(self, dir, id, name, defaults)
    }
    fn import_collection(
        &self,
        root: &Path,
        collection: &Collection,
    ) -> Result<OpenedCollection, StoreError> {
        T::import_collection(self, root, collection)
    }
    fn read_text(&self, file: &Path) -> Result<String, StoreError> {
        T::read_text(self, file)
    }
    fn read_secrets(&self, environment_file: &Path) -> Result<Option<Vec<u8>>, StoreError> {
        T::read_secrets(self, environment_file)
    }
    fn write_secrets(
        &self,
        environment_file: &Path,
        data: Option<&[u8]>,
    ) -> Result<(), StoreError> {
        T::write_secrets(self, environment_file, data)
    }
    fn recipients(&self, root: &Path) -> Result<Vec<String>, StoreError> {
        T::recipients(self, root)
    }
    fn write_recipients(&self, root: &Path, recipients: &[String]) -> Result<(), StoreError> {
        T::write_recipients(self, root, recipients)
    }
}

/// A loaded collection with the place of every item.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OpenedCollection {
    /// The collection folder.
    pub root: PathBuf,
    /// The request tree.
    pub collection: Collection,
    /// Environments of the collection, sorted by name.
    pub environments: Vec<Environment>,
    /// Where each item (collection, folder, request, environment) lives.
    pub locations: HashMap<ItemId, Location>,
    /// Files that could not be loaded (they are left untouched).
    pub problems: Vec<StoreError>,
}

/// Where an item lives.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Location {
    /// Collection or folder directory, or request file.
    pub path: PathBuf,
    /// Never written (newer format version, damaged settings).
    pub read_only: bool,
}

/// What went wrong with a file or folder.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("{}: {detail}", path.display())]
pub struct StoreError {
    /// Kind, for the message shown.
    pub kind: StoreErrorKind,
    /// Affected file or folder.
    pub path: PathBuf,
    /// Technical detail (English).
    pub detail: String,
}

/// Kinds of [`StoreError`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StoreErrorKind {
    /// The file system refused.
    Io,
    /// Not valid TOML, wrong format or invalid values.
    Damaged,
    /// Written by a newer program version.
    ReadOnly,
    /// The folder has no `collection.toml`.
    NotACollection,
    /// A collection already exists there.
    AlreadyExists,
}

impl StoreError {
    /// The text to show the user.
    #[must_use]
    pub fn user_text(&self) -> UserText {
        let path = self.path.display().to_string();
        match self.kind {
            StoreErrorKind::Io => UserText::new(keys::STORE_IO)
                .with("path", path)
                .with("detail", self.detail.clone()),
            StoreErrorKind::Damaged => UserText::new(keys::STORE_DAMAGED)
                .with("path", path)
                .with("detail", self.detail.clone()),
            StoreErrorKind::ReadOnly => UserText::new(keys::STORE_READ_ONLY).with("path", path),
            StoreErrorKind::NotACollection => {
                UserText::new(keys::NOT_A_COLLECTION).with("path", path)
            }
            StoreErrorKind::AlreadyExists => {
                UserText::new(keys::COLLECTION_EXISTS).with("path", path)
            }
        }
    }
}

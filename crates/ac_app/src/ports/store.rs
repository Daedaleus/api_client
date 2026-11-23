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

    /// Creates a file for `request` in the collection or folder `dir`;
    /// returns its path.
    ///
    /// # Errors
    /// If the file or the folder settings cannot be written.
    fn create_request(&self, dir: &Path, request: &Request) -> Result<PathBuf, StoreError>;

    /// Creates a folder named `name` in `parent`; returns its path and id.
    ///
    /// # Errors
    /// If the folder cannot be created.
    fn create_folder(&self, parent: &Path, name: &str) -> Result<(PathBuf, ItemId), StoreError>;

    /// Moves the request file or folder `path` into the collection or folder
    /// `to_dir`, at `position` among its items (also within the same
    /// folder: reordering).
    ///
    /// # Errors
    /// If a folder is moved into itself, settings are damaged or read-only,
    /// or it cannot be moved.
    fn move_item(&self, path: &Path, to_dir: &Path, position: usize) -> Result<(), StoreError>;

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

    /// Writes `collection` (and its `environments`) as a new collection
    /// into `root` (imports) and loads it.
    ///
    /// # Errors
    /// If `root` already holds a collection or cannot be written.
    fn import_collection(
        &self,
        root: &Path,
        collection: &Collection,
        environments: &[Environment],
    ) -> Result<OpenedCollection, StoreError>;

    /// The text files below `dir` with one of `extensions` (hidden folders
    /// and `node_modules` left out), for importing a folder-based collection.
    ///
    /// # Errors
    /// If the folder or a file cannot be read.
    fn read_tree(&self, dir: &Path, extensions: &[&str]) -> Result<Vec<TextFile>, StoreError>;

    /// The text of a file the user picked to import.
    ///
    /// # Errors
    /// If it cannot be read or is no UTF-8 text.
    fn read_text(&self, file: &Path) -> Result<String, StoreError>;

    /// Writes an export or a response body to the file the user picked
    /// (replacing it, as the save dialog confirmed).
    ///
    /// # Errors
    /// If it cannot be written.
    fn write_file(&self, file: &Path, bytes: &[u8]) -> Result<(), StoreError>;

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

    /// A new, unused folder for a collection that is not saved yet
    /// (imports); see [`CollectionStore::discard_temporary`].
    ///
    /// # Errors
    /// If no such folder can be chosen.
    fn temporary_root(&self) -> Result<PathBuf, StoreError>;

    /// Moves the whole collection at `from` to `to` (saving an unsaved
    /// collection). `to` must not exist or be an empty folder.
    ///
    /// # Errors
    /// If `to` holds something or the collection cannot be moved.
    fn move_collection(&self, from: &Path, to: &Path) -> Result<(), StoreError>;

    /// Deletes an unsaved collection for good. Refuses any folder that
    /// [`CollectionStore::temporary_root`] did not hand out.
    ///
    /// # Errors
    /// If `root` is no unsaved collection or cannot be deleted.
    fn discard_temporary(&self, root: &Path) -> Result<(), StoreError>;
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
    fn create_request(&self, dir: &Path, request: &Request) -> Result<PathBuf, StoreError> {
        T::create_request(self, dir, request)
    }
    fn create_folder(&self, parent: &Path, name: &str) -> Result<(PathBuf, ItemId), StoreError> {
        T::create_folder(self, parent, name)
    }
    fn move_item(&self, path: &Path, to_dir: &Path, position: usize) -> Result<(), StoreError> {
        T::move_item(self, path, to_dir, position)
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
        environments: &[Environment],
    ) -> Result<OpenedCollection, StoreError> {
        T::import_collection(self, root, collection, environments)
    }
    fn read_tree(&self, dir: &Path, extensions: &[&str]) -> Result<Vec<TextFile>, StoreError> {
        T::read_tree(self, dir, extensions)
    }
    fn read_text(&self, file: &Path) -> Result<String, StoreError> {
        T::read_text(self, file)
    }
    fn write_file(&self, file: &Path, bytes: &[u8]) -> Result<(), StoreError> {
        T::write_file(self, file, bytes)
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
    fn temporary_root(&self) -> Result<PathBuf, StoreError> {
        T::temporary_root(self)
    }
    fn move_collection(&self, from: &Path, to: &Path) -> Result<(), StoreError> {
        T::move_collection(self, from, to)
    }
    fn discard_temporary(&self, root: &Path) -> Result<(), StoreError> {
        T::discard_temporary(self, root)
    }
}

/// A text file below a folder, by its path relative to that folder (with
/// `/` as separator on every platform).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextFile {
    /// Relative path.
    pub path: String,
    /// Its text.
    pub text: String,
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

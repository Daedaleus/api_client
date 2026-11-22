//! Reads a collection folder into the domain tree.
//!
//! Tolerant by design: a damaged file is reported as a [`LoadProblem`] and
//! skipped, the rest of the collection still opens. Nothing is written.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use ac_domain::collection::{Collection, Folder, Item, ItemId};
use ac_domain::environment::Environment;

use crate::document;
use crate::environment_file::read_environment;
use crate::error::{LoadProblem, StorageError};
use crate::format::{
    COLLECTION_FILE, COLLECTION_FORMAT, ENVIRONMENTS_FOLDER, EXTENSION, FOLDER_FILE, FOLDER_FORMAT,
    RESERVED_ROOT_FOLDERS, is_visible,
};
use crate::names::file_stem;
use crate::request_file::read_request;
use crate::{auth, scripts, tls, transport, variables};

/// A loaded collection with the file behind every item.
#[derive(Debug)]
pub struct LoadedCollection {
    /// The collection folder.
    pub root: PathBuf,
    /// The request tree.
    pub collection: Collection,
    /// Environments, sorted by name.
    pub environments: Vec<Environment>,
    /// Where each item (collection, folder, request, environment) lives.
    pub entries: HashMap<ItemId, Entry>,
    /// Files and folders that could not be loaded.
    pub problems: Vec<LoadProblem>,
}

/// Where an item lives on disk.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    /// The collection or folder directory, or the request file.
    pub path: PathBuf,
    /// What it is.
    pub kind: EntryKind,
    /// Written by a newer version or settings damaged: never written.
    pub read_only: bool,
}

/// Kind of an [`Entry`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntryKind {
    /// The collection root.
    Collection,
    /// A folder of the collection.
    Folder,
    /// A request file.
    Request,
    /// An environment file.
    Environment,
}

/// Loads the collection in `root`.
///
/// # Errors
/// Only if the collection itself cannot be opened (`collection.toml`
/// missing, damaged or of another format); problems with single files are
/// listed in [`LoadedCollection::problems`].
pub fn load_collection(root: &Path) -> Result<LoadedCollection, StorageError> {
    let settings = root.join(COLLECTION_FILE);
    if !settings.is_file() {
        return Err(StorageError::NotACollection {
            path: root.to_owned(),
        });
    }
    let parsed = document::read(&settings, COLLECTION_FORMAT)?;
    let table = parsed.doc.as_table();
    let id = document::id(table, &settings, root)?;
    let name = document::opt_str(table, "name", &settings)?.unwrap_or_else(|| file_stem(root));
    let order = document::order(table, &settings)?;
    let variables = variables::read(table.get(variables::KEY), &settings, variables::KEY)?;
    let auth = auth::read(table.get(auth::KEY), &settings, auth::KEY)?;
    let certificate_check = tls::read(table, &settings)?;
    let transport = transport::read(table, &settings)?;
    let scripts = scripts::read(table.get(scripts::KEY), &settings, scripts::KEY)?;

    let mut loader = Loader::default();
    loader.entries.insert(
        id,
        Entry {
            path: root.to_owned(),
            kind: EntryKind::Collection,
            read_only: parsed.read_only(),
        },
    );
    let items = loader.items(root, &order, true);
    let environments = loader.environments(&root.join(ENVIRONMENTS_FOLDER));
    Ok(LoadedCollection {
        root: root.to_owned(),
        collection: Collection {
            transport,
            scripts,
            id,
            name,
            items,
            variables,
            auth,
            certificate_check,
        },
        environments,
        entries: loader.entries,
        problems: loader.problems,
    })
}

/// What a `folder.toml` holds.
struct FolderSettings {
    id: ItemId,
    name: String,
    order: Vec<String>,
    variables: Vec<ac_domain::environment::Variable>,
    auth: ac_domain::auth::Auth,
    certificate_check: ac_domain::tls::CertificateCheck,
    transport: ac_domain::transport::TransportSettings,
    scripts: ac_domain::script::Scripts,
    read_only: bool,
}

#[derive(Default)]
struct Loader {
    entries: HashMap<ItemId, Entry>,
    problems: Vec<LoadProblem>,
}

impl Loader {
    fn problem(&mut self, error: StorageError) {
        self.problems.push(LoadProblem { error });
    }

    /// The items of `dir`: listed in `order` first, the rest by name.
    fn items(&mut self, dir: &Path, order: &[String], is_root: bool) -> Vec<Item> {
        let names = match Self::names(dir, is_root) {
            Ok(names) => names,
            Err(error) => {
                self.problem(error);
                return Vec::new();
            }
        };
        let mut ordered: Vec<&String> = Vec::with_capacity(names.len());
        for listed in order {
            if let Some(name) = names.iter().find(|name| *name == listed)
                && !ordered.contains(&name)
            {
                ordered.push(name);
            }
        }
        for name in &names {
            if !ordered.contains(&name) {
                ordered.push(name);
            }
        }
        ordered
            .into_iter()
            .filter_map(|name| {
                let path = dir.join(name);
                if path.is_dir() {
                    Some(self.folder(&path))
                } else {
                    self.request(&path)
                }
            })
            .collect()
    }

    /// Names of folders and request files in `dir`, sorted case-insensitively.
    fn names(dir: &Path, is_root: bool) -> Result<Vec<String>, StorageError> {
        entry_names(dir, is_root)
    }

    fn folder(&mut self, dir: &Path) -> Item {
        let settings = dir.join(FOLDER_FILE);
        let defaults = || FolderSettings {
            id: document::derived_id(dir),
            name: file_stem(dir),
            order: Vec::new(),
            variables: Vec::new(),
            auth: ac_domain::auth::Auth::Inherit,
            certificate_check: ac_domain::tls::CertificateCheck::Inherit,
            transport: ac_domain::transport::TransportSettings::default(),
            scripts: ac_domain::script::Scripts::default(),
            read_only: false,
        };
        let FolderSettings {
            id,
            name,
            order,
            variables,
            auth,
            certificate_check,
            transport,
            scripts,
            read_only,
        } = if settings.is_file() {
            match Self::folder_settings(&settings) {
                Ok(values) => values,
                Err(error) => {
                    // Show the contents anyway, but never overwrite the
                    // damaged settings.
                    self.problem(error);
                    FolderSettings {
                        read_only: true,
                        ..defaults()
                    }
                }
            }
        } else {
            defaults()
        };
        let id = self.unique(id, dir);
        let items = self.items(dir, &order, false);
        self.entries.insert(
            id,
            Entry {
                path: dir.to_owned(),
                kind: EntryKind::Folder,
                read_only,
            },
        );
        Item::Folder(Folder {
            transport,
            scripts,
            id,
            name,
            items,
            variables,
            auth,
            certificate_check,
        })
    }

    fn folder_settings(settings: &Path) -> Result<FolderSettings, StorageError> {
        let parsed = document::read(settings, FOLDER_FORMAT)?;
        let table = parsed.doc.as_table();
        let dir = settings.parent().unwrap_or(settings);
        Ok(FolderSettings {
            id: document::id(table, settings, dir)?,
            name: document::opt_str(table, "name", settings)?.unwrap_or_else(|| file_stem(dir)),
            order: document::order(table, settings)?,
            variables: variables::read(table.get(variables::KEY), settings, variables::KEY)?,
            auth: auth::read(table.get(auth::KEY), settings, auth::KEY)?,
            certificate_check: tls::read(table, settings)?,
            transport: transport::read(table, settings)?,
            scripts: scripts::read(table.get(scripts::KEY), settings, scripts::KEY)?,
            read_only: parsed.read_only(),
        })
    }

    /// The environment files in `dir` (missing folder = none), by name.
    fn environments(&mut self, dir: &Path) -> Vec<Environment> {
        if !dir.is_dir() {
            return Vec::new();
        }
        let names = match Self::names(dir, false) {
            Ok(names) => names,
            Err(error) => {
                self.problem(error);
                return Vec::new();
            }
        };
        let mut environments = Vec::new();
        for name in names {
            let path = dir.join(name);
            if path.is_dir() {
                continue;
            }
            match read_environment(&path) {
                Ok((mut environment, read_only)) => {
                    environment.id = self.unique(environment.id, &path);
                    self.entries.insert(
                        environment.id,
                        Entry {
                            path,
                            kind: EntryKind::Environment,
                            read_only,
                        },
                    );
                    environments.push(environment);
                }
                Err(error) => self.problem(error),
            }
        }
        environments
    }

    fn request(&mut self, path: &Path) -> Option<Item> {
        match read_request(path) {
            Ok((mut request, read_only)) => {
                request.id = self.unique(request.id, path);
                self.entries.insert(
                    request.id,
                    Entry {
                        path: path.to_owned(),
                        kind: EntryKind::Request,
                        read_only,
                    },
                );
                Some(Item::Request(request))
            }
            Err(error) => {
                self.problem(error);
                None
            }
        }
    }

    /// A copied file brings the id of its original; it gets one derived from
    /// its own path (stable across loads, written on its next save) so every
    /// item stays addressable.
    fn unique(&self, id: ItemId, item: &Path) -> ItemId {
        if self.entries.contains_key(&id) {
            document::derived_id(item)
        } else {
            id
        }
    }
}

/// Names of folders and request files in `dir`, sorted case-insensitively
/// (the collection folder leaves out its reserved folders).
pub(crate) fn entry_names(dir: &Path, is_root: bool) -> Result<Vec<String>, StorageError> {
    let reading = std::fs::read_dir(dir).map_err(|error| StorageError::io(dir, error))?;
    let mut names = Vec::new();
    for entry in reading {
        let entry = entry.map_err(|error| StorageError::io(dir, error))?;
        let name = entry.file_name().to_string_lossy().into_owned();
        let path = entry.path();
        let settings = if is_root {
            COLLECTION_FILE
        } else {
            FOLDER_FILE
        };
        let wanted = if path.is_dir() {
            !(is_root && RESERVED_ROOT_FOLDERS.contains(&name.as_str()))
        } else {
            name != settings
                && path
                    .extension()
                    .is_some_and(|extension| extension == EXTENSION)
        };
        if wanted && is_visible(&name) {
            names.push(name);
        }
    }
    names.sort_by_key(|name| name.to_lowercase());
    Ok(names)
}

#[cfg(test)]
#[path = "tests/load.rs"]
mod tests;

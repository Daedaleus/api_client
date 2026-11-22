//! Test double for the collection store: collections kept in memory.
//!
//! Paths are made up: an item lives at its parent's path joined with its
//! id, so a path names an item unambiguously.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use ac_domain::collection::{Collection, Folder, Item, ItemId, Request, RequestDefaults};
use ac_domain::environment::Environment;

use super::lock;
use crate::ports::{
    CollectionStore, Location, OpenedCollection, StoreError, StoreErrorKind, TextFile,
};

/// A [`CollectionStore`] in memory.
#[derive(Debug, Default)]
pub struct MemoryStore {
    collections: Mutex<HashMap<PathBuf, Collection>>,
    environments: Mutex<HashMap<PathBuf, Vec<Environment>>>,
    /// Encrypted secrets by environment file.
    secrets: Mutex<HashMap<PathBuf, Vec<u8>>>,
    /// Recipients by collection root.
    recipients: Mutex<HashMap<PathBuf, Vec<String>>>,
    /// Text files to import, by path.
    files: Mutex<HashMap<PathBuf, String>>,
    read_only: Mutex<HashSet<ItemId>>,
    problems: Mutex<Vec<StoreError>>,
    fail_next: Mutex<Option<StoreError>>,
}

impl MemoryStore {
    /// An empty store.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds a collection at `root`.
    #[must_use]
    pub fn with(self, root: impl Into<PathBuf>, collection: Collection) -> Self {
        lock(&self.collections).insert(root.into(), collection);
        self
    }

    /// Adds an environment to the collection at `root`.
    #[must_use]
    pub fn with_environment(self, root: impl Into<PathBuf>, environment: Environment) -> Self {
        lock(&self.environments)
            .entry(root.into())
            .or_default()
            .push(environment);
        self
    }

    /// The current environments of the collection at `root`.
    #[must_use]
    pub fn environments(&self, root: &Path) -> Vec<Environment> {
        lock(&self.environments)
            .get(root)
            .cloned()
            .unwrap_or_default()
    }

    /// The encrypted secrets stored for the environment `id` in `root`.
    #[must_use]
    pub fn secrets(&self, root: &Path, id: ItemId) -> Option<Vec<u8>> {
        lock(&self.secrets)
            .get(&environment_path(root, id))
            .cloned()
    }

    /// Stores encrypted secrets for the environment `id` in `root`.
    pub fn set_secrets(&self, root: &Path, id: ItemId, data: Vec<u8>) {
        lock(&self.secrets).insert(environment_path(root, id), data);
    }

    /// The recipients set up for the collection at `root`.
    #[must_use]
    pub fn recipients_of(&self, root: &Path) -> Vec<String> {
        lock(&self.recipients)
            .get(root)
            .cloned()
            .unwrap_or_default()
    }

    /// Sets the recipients of the collection at `root`.
    pub fn set_recipients(&self, root: &Path, recipients: Vec<String>) {
        lock(&self.recipients).insert(root.to_owned(), recipients);
    }

    /// Offers a text file to import at `path`.
    pub fn add_file(&self, path: impl Into<PathBuf>, text: impl Into<String>) {
        lock(&self.files).insert(path.into(), text.into());
    }

    /// Marks an item as read-only (e.g. from a newer format version).
    pub fn mark_read_only(&self, id: ItemId) {
        lock(&self.read_only).insert(id);
    }

    /// Reports `problem` with every open.
    pub fn add_problem(&self, problem: StoreError) {
        lock(&self.problems).push(problem);
    }

    /// Makes the next store call fail with `error`.
    pub fn fail_next(&self, error: StoreError) {
        *lock(&self.fail_next) = Some(error);
    }

    /// The current state of the collection at `root`.
    ///
    /// # Panics
    /// If there is none.
    #[must_use]
    pub fn collection(&self, root: &Path) -> Collection {
        lock(&self.collections)[root].clone()
    }

    fn check_failure(&self) -> Result<(), StoreError> {
        lock(&self.fail_next).take().map_or(Ok(()), Err)
    }

    /// Runs `change` on the items of the collection or folder at `dir`.
    fn with_items<T>(
        &self,
        dir: &Path,
        change: impl FnOnce(&mut Vec<Item>) -> T,
    ) -> Result<T, StoreError> {
        self.check_failure()?;
        let mut collections = lock(&self.collections);
        for (root, collection) in collections.iter_mut() {
            if let Some(items) = items_at(&mut collection.items, root, dir) {
                return Ok(change(items));
            }
        }
        Err(not_found(dir))
    }
}

impl CollectionStore for MemoryStore {
    fn open(&self, root: &Path) -> Result<OpenedCollection, StoreError> {
        self.check_failure()?;
        let collection = lock(&self.collections)
            .get(root)
            .cloned()
            .ok_or_else(|| StoreError {
                kind: StoreErrorKind::NotACollection,
                path: root.to_owned(),
                detail: "no collection".into(),
            })?;
        let read_only = lock(&self.read_only).clone();
        let mut locations = HashMap::new();
        let mut add = |id: ItemId, path: PathBuf| {
            locations.insert(
                id,
                Location {
                    path,
                    read_only: read_only.contains(&id),
                },
            );
        };
        add(collection.id, root.to_owned());
        collect_locations(&collection.items, root, &mut add);
        let environments = self.environments(root);
        for environment in &environments {
            add(environment.id, environment_path(root, environment.id));
        }
        Ok(OpenedCollection {
            root: root.to_owned(),
            collection,
            environments,
            locations,
            problems: lock(&self.problems).clone(),
        })
    }

    fn create_collection(&self, root: &Path, name: &str) -> Result<OpenedCollection, StoreError> {
        self.check_failure()?;
        {
            let mut collections = lock(&self.collections);
            if collections.contains_key(root) {
                return Err(StoreError {
                    kind: StoreErrorKind::AlreadyExists,
                    path: root.to_owned(),
                    detail: "exists".into(),
                });
            }
            collections.insert(
                root.to_owned(),
                Collection {
                    scripts: ac_domain::script::Scripts::default(),
                    certificate_check: ac_domain::tls::CertificateCheck::Inherit,
                    auth: ac_domain::auth::Auth::Inherit,
                    variables: Vec::new(),
                    id: ItemId::new(),
                    name: name.to_owned(),
                    items: Vec::new(),
                },
            );
        }
        self.open(root)
    }

    fn save_request(&self, file: &Path, request: &Request) -> Result<(), StoreError> {
        let id = id_of(file)?;
        self.with_items(parent_of(file), |items| {
            for item in items.iter_mut() {
                if item.id() == id {
                    *item = Item::Request(request.clone());
                }
            }
        })
    }

    fn create_request(&self, dir: &Path, request: &Request) -> Result<(), StoreError> {
        self.with_items(dir, |items| items.push(Item::Request(request.clone())))
    }

    fn create_folder(&self, parent: &Path, name: &str) -> Result<ItemId, StoreError> {
        let id = ItemId::new();
        self.with_items(parent, |items| {
            items.push(Item::Folder(Folder {
                scripts: ac_domain::script::Scripts::default(),
                certificate_check: ac_domain::tls::CertificateCheck::Inherit,
                auth: ac_domain::auth::Auth::Inherit,
                variables: Vec::new(),
                id,
                name: name.to_owned(),
                items: Vec::new(),
            }));
        })?;
        Ok(id)
    }

    fn rename_folder(&self, dir: &Path, id: ItemId, name: &str) -> Result<(), StoreError> {
        self.with_items(parent_of(dir), |items| {
            for item in items.iter_mut() {
                if let Item::Folder(folder) = item
                    && folder.id == id
                {
                    name.clone_into(&mut folder.name);
                }
            }
        })
    }

    fn save_collection_defaults(
        &self,
        root: &Path,
        defaults: &RequestDefaults,
    ) -> Result<(), StoreError> {
        self.check_failure()?;
        let mut collections = lock(&self.collections);
        let collection = collections.get_mut(root).ok_or_else(|| not_found(root))?;
        collection.variables.clone_from(&defaults.variables);
        collection.auth = defaults.auth.clone();
        collection.certificate_check = defaults.certificate_check;
        collection.scripts.clone_from(&defaults.scripts);
        Ok(())
    }

    fn save_folder_defaults(
        &self,
        dir: &Path,
        id: ItemId,
        _name: &str,
        defaults: &RequestDefaults,
    ) -> Result<(), StoreError> {
        self.check_failure()?;
        self.with_items(parent_of(dir), |items| {
            for item in items.iter_mut() {
                if let Item::Folder(folder) = item
                    && folder.id == id
                {
                    folder.variables.clone_from(&defaults.variables);
                    folder.auth = defaults.auth.clone();
                    folder.certificate_check = defaults.certificate_check;
                    folder.scripts.clone_from(&defaults.scripts);
                }
            }
        })
    }

    fn remove(&self, path: &Path) -> Result<(), StoreError> {
        let id = id_of(path)?;
        if let Some(environments) = self.environments_at(path) {
            if let Some(list) = lock(&self.environments).get_mut(&environments) {
                list.retain(|environment| environment.id != id);
            }
            lock(&self.secrets).remove(path);
            return Ok(());
        }
        self.with_items(parent_of(path), |items| {
            items.retain(|item| item.id() != id);
        })
    }

    fn save_environment(&self, file: &Path, environment: &Environment) -> Result<(), StoreError> {
        self.check_failure()?;
        let root = self.environments_at(file).ok_or_else(|| not_found(file))?;
        let id = id_of(file)?;
        for existing in lock(&self.environments).entry(root).or_default() {
            if existing.id == id {
                *existing = environment.clone();
            }
        }
        Ok(())
    }

    fn create_environment(&self, root: &Path, environment: &Environment) -> Result<(), StoreError> {
        self.check_failure()?;
        lock(&self.environments)
            .entry(root.to_owned())
            .or_default()
            .push(environment.clone());
        Ok(())
    }

    fn import_collection(
        &self,
        root: &Path,
        collection: &Collection,
        environments: &[Environment],
    ) -> Result<OpenedCollection, StoreError> {
        self.check_failure()?;
        if lock(&self.collections).contains_key(root) {
            return Err(StoreError {
                kind: StoreErrorKind::AlreadyExists,
                path: root.to_owned(),
                detail: "collection exists".into(),
            });
        }
        lock(&self.collections).insert(root.to_owned(), collection.clone());
        lock(&self.environments).insert(root.to_owned(), environments.to_vec());
        self.open(root)
    }

    fn read_tree(&self, dir: &Path, extensions: &[&str]) -> Result<Vec<TextFile>, StoreError> {
        self.check_failure()?;
        let files = lock(&self.files);
        let mut found: Vec<TextFile> = files
            .iter()
            .filter_map(|(path, text)| {
                let relative = path.strip_prefix(dir).ok()?;
                let extension = relative.extension()?.to_str()?;
                extensions.contains(&extension).then(|| TextFile {
                    path: relative.to_string_lossy().replace('\\', "/"),
                    text: text.clone(),
                })
            })
            .collect();
        if found.is_empty() {
            return Err(not_found(dir));
        }
        found.sort_by(|a, b| a.path.cmp(&b.path));
        Ok(found)
    }

    fn read_text(&self, file: &Path) -> Result<String, StoreError> {
        self.check_failure()?;
        lock(&self.files)
            .get(file)
            .cloned()
            .ok_or_else(|| not_found(file))
    }

    fn read_secrets(&self, environment_file: &Path) -> Result<Option<Vec<u8>>, StoreError> {
        self.check_failure()?;
        Ok(lock(&self.secrets).get(environment_file).cloned())
    }

    fn write_secrets(
        &self,
        environment_file: &Path,
        data: Option<&[u8]>,
    ) -> Result<(), StoreError> {
        self.check_failure()?;
        let mut secrets = lock(&self.secrets);
        match data {
            Some(data) => secrets.insert(environment_file.to_owned(), data.to_vec()),
            None => secrets.remove(environment_file),
        };
        Ok(())
    }

    fn recipients(&self, root: &Path) -> Result<Vec<String>, StoreError> {
        self.check_failure()?;
        Ok(self.recipients_of(root))
    }

    fn write_recipients(&self, root: &Path, recipients: &[String]) -> Result<(), StoreError> {
        self.check_failure()?;
        self.set_recipients(root, recipients.to_vec());
        Ok(())
    }

    fn temporary_root(&self) -> Result<PathBuf, StoreError> {
        self.check_failure()?;
        Ok(Path::new(TEMPORARY).join(ItemId::new().to_string()))
    }

    fn move_collection(&self, from: &Path, to: &Path) -> Result<(), StoreError> {
        self.check_failure()?;
        if lock(&self.collections).contains_key(to) {
            return Err(StoreError {
                kind: StoreErrorKind::AlreadyExists,
                path: to.to_owned(),
                detail: "collection exists".into(),
            });
        }
        let moved = |path: &Path| -> Option<PathBuf> {
            path.strip_prefix(from).ok().map(|rest| to.join(rest))
        };
        rekey(&self.collections, moved);
        rekey(&self.environments, moved);
        rekey(&self.secrets, moved);
        rekey(&self.recipients, moved);
        Ok(())
    }

    fn discard_temporary(&self, root: &Path) -> Result<(), StoreError> {
        self.check_failure()?;
        if !root.starts_with(TEMPORARY) {
            return Err(StoreError {
                kind: StoreErrorKind::Io,
                path: root.to_owned(),
                detail: "not an unsaved collection".into(),
            });
        }
        let inside = |path: &PathBuf| path.starts_with(root);
        lock(&self.collections).retain(|path, _| !inside(path));
        lock(&self.environments).retain(|path, _| !inside(path));
        lock(&self.secrets).retain(|path, _| !inside(path));
        lock(&self.recipients).retain(|path, _| !inside(path));
        Ok(())
    }
}

/// Where [`MemoryStore`] puts unsaved collections.
pub const TEMPORARY: &str = "unsaved";

/// Moves the entries of `map` whose path `moved` maps elsewhere.
fn rekey<T>(map: &Mutex<HashMap<PathBuf, T>>, moved: impl Fn(&Path) -> Option<PathBuf>) {
    let mut map = lock(map);
    let keys: Vec<PathBuf> = map
        .keys()
        .filter(|path| moved(path).is_some())
        .cloned()
        .collect();
    for key in keys {
        if let (Some(target), Some(value)) = (moved(&key), map.remove(&key)) {
            map.insert(target, value);
        }
    }
}

impl MemoryStore {
    /// The collection root if `path` is an environment file of a collection.
    fn environments_at(&self, path: &Path) -> Option<PathBuf> {
        let root = path.parent()?.parent()?;
        let is_environment = path.parent() == Some(&root.join(ENVIRONMENTS))
            && lock(&self.collections).contains_key(root);
        is_environment.then(|| root.to_owned())
    }
}

/// Folder of the made-up environment paths.
const ENVIRONMENTS: &str = "environments";

fn environment_path(root: &Path, id: ItemId) -> PathBuf {
    root.join(ENVIRONMENTS).join(id.to_string())
}

fn collect_locations(items: &[Item], base: &Path, add: &mut impl FnMut(ItemId, PathBuf)) {
    for item in items {
        let path = base.join(item.id().to_string());
        if let Item::Folder(folder) = item {
            collect_locations(&folder.items, &path, add);
        }
        add(item.id(), path);
    }
}

fn items_at<'a>(items: &'a mut Vec<Item>, base: &Path, target: &Path) -> Option<&'a mut Vec<Item>> {
    if base == target {
        return Some(items);
    }
    for item in items.iter_mut() {
        if let Item::Folder(folder) = item {
            let path = base.join(folder.id.to_string());
            if target.starts_with(&path) {
                return items_at(&mut folder.items, &path, target);
            }
        }
    }
    None
}

fn id_of(path: &Path) -> Result<ItemId, StoreError> {
    path.file_name()
        .and_then(|name| name.to_str())
        .and_then(|name| name.parse().ok())
        .ok_or_else(|| not_found(path))
}

fn parent_of(path: &Path) -> &Path {
    path.parent().unwrap_or(path)
}

fn not_found(path: &Path) -> StoreError {
    StoreError {
        kind: StoreErrorKind::Io,
        path: path.to_owned(),
        detail: "not found".into(),
    }
}

use ac_domain::collection::Item;

use super::*;
use crate::test_support::{read, request, shop_api, tree_names};

/// Tests must not fill the real trash: discarded items are renamed to the
/// hidden `.trashed-<name>` next to them (hidden entries are not loaded).
fn rename_aside(path: &Path) -> std::io::Result<()> {
    let name = path.file_name().unwrap().to_string_lossy();
    std::fs::rename(path, path.with_file_name(format!(".trashed-{name}")))
}

fn store() -> FileStore {
    FileStore::with_discard(rename_aside)
}

#[test]
fn the_port_reads_and_changes_collection_files() {
    let (_temp, root) = shop_api();
    let store = store();
    let opened = store.open(&root).unwrap();
    assert_eq!(opened.problems.len(), 2);
    assert_eq!(opened.problems[0].kind, StoreErrorKind::Damaged);
    let health = request(&opened.collection, "Health").clone();
    let location = &opened.locations[&health.id];
    assert_eq!(location.path, root.join("health.toml"));
    assert!(!location.read_only);

    let renamed = Request {
        name: "Ping".into(),
        ..health
    };
    store.save_request(&location.path, &renamed).unwrap();
    store
        .create_request(&root, &Request::new("Status"))
        .unwrap();
    let folder = store.create_folder(&root, "Orders").unwrap();
    store
        .rename_folder(&root.join("orders"), folder, "Orders and payments")
        .unwrap();
    store.remove(&root.join("auth")).unwrap();
    assert!(root.join(".trashed-auth").is_dir());

    let reopened = store.open(&root).unwrap();
    assert_eq!(
        tree_names(&reopened.collection.items),
        ["Products/", "Ping", "Status", "Orders and payments/"]
    );
    assert!(matches!(
        reopened.collection.find(folder),
        Some(Item::Folder(_))
    ));
}

#[test]
fn environments_go_through_the_port() {
    let (_temp, root) = shop_api();
    let store = store();
    let opened = store.open(&root).unwrap();
    let mut dev = opened.environments[0].clone();
    dev.name = "Development".into();
    store
        .save_environment(&opened.locations[&dev.id].path, &dev)
        .unwrap();
    store
        .create_environment(&root, &Environment::new("Staging"))
        .unwrap();

    let reopened = store.open(&root).unwrap();
    let names: Vec<_> = reopened
        .environments
        .iter()
        .map(|e| e.name.as_str())
        .collect();
    assert_eq!(names, ["Development", "Staging"]);

    store.remove(&reopened.locations[&dev.id].path).unwrap();
    assert_eq!(store.open(&root).unwrap().environments.len(), 1);

    let error = store
        .create_environment(&root.join("health.toml"), &Environment::new("x"))
        .unwrap_err();
    assert_eq!(error.kind, StoreErrorKind::Io);
    let error = store
        .save_environment(&root.join("products").join("broken.toml"), &dev)
        .unwrap_err();
    assert_eq!(error.kind, StoreErrorKind::Damaged);
}

#[test]
fn collections_are_created_through_the_port() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("new");
    let opened = store().create_collection(&root, "New").unwrap();
    assert_eq!(opened.collection.name, "New");
    assert_eq!(opened.locations[&opened.collection.id].path, root);
    let error = store().create_collection(&root, "Again").unwrap_err();
    assert_eq!(error.kind, StoreErrorKind::AlreadyExists);
}

#[test]
fn storage_errors_map_to_port_errors() {
    let (_temp, root) = shop_api();
    let store = store();

    let error = store.open(&root.join("products")).unwrap_err();
    assert_eq!(error.kind, StoreErrorKind::NotACollection);
    assert_eq!(error.path, root.join("products"));

    let newer = root.join("products").join("newer.toml");
    let error = store.save_request(&newer, &Request::new("x")).unwrap_err();
    assert_eq!(error.kind, StoreErrorKind::ReadOnly);
    assert!(error.detail.contains("version 2"), "{error}");

    let broken = root.join("products").join("broken.toml");
    let error = store.save_request(&broken, &Request::new("x")).unwrap_err();
    assert_eq!(error.kind, StoreErrorKind::Damaged);

    let wrong = root.join("products").join("wrong.toml");
    let error = store.save_request(&wrong, &Request::new("x")).unwrap_err();
    assert!(error.detail.contains("expected format"), "{error}");

    std::fs::write(
        root.join("bad.toml"),
        "format = \"api-client/request\"\nversion = 1\nname = 1\n",
    )
    .unwrap();
    let error = store
        .save_request(&root.join("bad.toml"), &Request::new("x"))
        .unwrap_err();
    assert_eq!(error.detail, "name: must be a string");

    let error = store.remove(&root.join("missing.toml")).unwrap_err();
    assert_eq!(error.kind, StoreErrorKind::Io);
    assert_eq!(read(&root.join("collection.toml")).lines().count(), 6);
}

#[test]
fn the_default_store_uses_the_system_trash() {
    // Only checks the wiring; moving files to the real trash is not tested.
    let store = FileStore::new();
    assert!(std::ptr::fn_addr_eq(
        store.discard,
        move_to_trash as Discard
    ));
}

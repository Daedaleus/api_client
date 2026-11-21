use crate::StorageError;
use crate::test_support::{read, shop_api, tree_names};

use super::*;

#[test]
fn new_folders_get_settings_and_join_the_order() {
    let (_temp, root) = shop_api();
    let (dir, id) = create_folder(&root, "Orders & Payments").unwrap();
    assert_eq!(dir, root.join("orders-payments"));
    let settings = read(&dir.join("folder.toml"));
    assert!(
        settings.contains("format = \"api-client/folder\""),
        "{settings}"
    );
    assert!(settings.contains(&format!("id = \"{id}\"")), "{settings}");
    assert!(read(&root.join("collection.toml")).contains("\"orders-payments\"]"));

    let (second, _) = create_folder(&root, "Orders & Payments").unwrap();
    assert_eq!(second, root.join("orders-payments-2"));
    let (unnamed, _) = create_folder(&root, "???").unwrap();
    assert_eq!(unnamed, root.join("folder"));

    let loaded = load_collection(&root).unwrap();
    assert_eq!(loaded.entries[&id].path, dir);
}

#[test]
fn folders_cannot_be_created_in_missing_parents() {
    let (_temp, root) = shop_api();
    let error = create_folder(&root.join("nope"), "x").unwrap_err();
    assert!(matches!(error, StorageError::Io { .. }), "{error}");
}

#[test]
fn new_requests_get_unique_files_and_join_the_order() {
    let (_temp, root) = shop_api();
    let first = create_request(&root, &Request::new("Health")).unwrap();
    let second = create_request(&root, &Request::new("Health")).unwrap();
    assert_eq!(first, root.join("health-2.toml"));
    assert_eq!(second, root.join("health-3.toml"));

    let settings = read(&root.join("collection.toml"));
    assert!(
        settings.contains("order = [\"products\", \"health.toml\", \"missing.toml\", \"health-2.toml\", \"health-3.toml\"]"),
        "{settings}"
    );
    // The comment of the settings file survives.
    assert!(settings.starts_with("# Example collection"));

    let loaded = load_collection(&root).unwrap();
    assert_eq!(
        tree_names(&loaded.collection.items),
        ["Products/", "Health", "Health", "Health", "auth/"]
    );
}

#[test]
fn folders_without_settings_or_order_stay_unchanged() {
    let (_temp, root) = shop_api();
    let path = create_request(&root.join("auth"), &Request::new("Logout")).unwrap();
    assert_eq!(path, root.join("auth").join("logout.toml"));
    assert!(!root.join("auth").join("folder.toml").exists());

    std::fs::write(
        root.join("auth").join("folder.toml"),
        "format = \"api-client/folder\"\nversion = 1\nname = \"Auth\"\n",
    )
    .unwrap();
    create_request(&root.join("auth"), &Request::new("Refresh")).unwrap();
    assert!(!read(&root.join("auth").join("folder.toml")).contains("order"));
}

#[test]
fn order_of_read_only_settings_is_not_touched() {
    let (_temp, root) = shop_api();
    let settings = root.join("products").join("folder.toml");
    let newer = read(&settings).replace("version = 1", "version = 9");
    std::fs::write(&settings, &newer).unwrap();
    create_request(&root.join("products"), &Request::new("Delete product")).unwrap();
    assert_eq!(read(&settings), newer);
}

#[test]
fn listed_names_are_not_added_twice() {
    let (_temp, root) = shop_api();
    std::fs::remove_file(root.join("products").join("list-products.toml")).unwrap();
    create_request(&root.join("products"), &Request::new("List products")).unwrap();
    let settings = read(&root.join("products").join("folder.toml"));
    assert_eq!(
        settings.matches("list-products.toml").count(),
        1,
        "{settings}"
    );
}

#[test]
fn collections_are_created_once() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("new-api");
    let loaded = create_collection(&root, "New API").unwrap();
    assert_eq!(loaded.collection.name, "New API");
    assert_eq!(loaded.collection.items, vec![]);
    assert_eq!(loaded.problems.len(), 0);

    let error = create_collection(&root, "Again").unwrap_err();
    assert!(
        matches!(error, StorageError::AlreadyExists { .. }),
        "{error}"
    );
}

#[test]
fn folders_never_take_the_reserved_names_of_the_collection_root() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("api");
    create_collection(&root, "API").unwrap();
    let (environments, _) = create_folder(&root, "Environments").unwrap();
    assert_eq!(environments, root.join("environments-folder"));
    let (secrets, _) = create_folder(&root, "secrets").unwrap();
    assert_eq!(secrets, root.join("secrets-folder"));
    // Below the root the names are free.
    let (nested, _) = create_folder(&environments, "Environments").unwrap();
    assert_eq!(nested, environments.join("environments"));
    let loaded = load_collection(&root).unwrap();
    assert_eq!(loaded.collection.items.len(), 2);
}

#[test]
fn whole_collections_are_written_with_folders_requests_and_settings() {
    use ac_domain::auth::Auth;
    use ac_domain::collection::{Collection, Folder, Item};
    use ac_domain::environment::Variable;
    use ac_domain::tls::CertificateCheck;

    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("imported");
    let login = Request::new("Login");
    let collection = Collection {
        id: ItemId::new(),
        name: "Imported".into(),
        items: vec![
            Item::Folder(Folder {
                id: ItemId::new(),
                name: "Auth".into(),
                items: vec![Item::Request(login.clone())],
                variables: vec![Variable::plain("area", "auth")],
                auth: Auth::None,
                certificate_check: CertificateCheck::Off,
            }),
            Item::Request(Request::new("Health")),
        ],
        variables: vec![Variable::plain("baseUrl", "https://x.example")],
        auth: Auth::Bearer {
            token: "{{token}}".into(),
        },
        certificate_check: CertificateCheck::Inherit,
    };
    let loaded = write_collection(&root, &collection).unwrap();
    assert_eq!(loaded.collection.name, "Imported");
    assert_eq!(loaded.collection.variables, collection.variables);
    assert_eq!(loaded.collection.auth, collection.auth);
    let names: Vec<_> = loaded.collection.items.iter().map(Item::name).collect();
    assert_eq!(names, ["Auth", "Health"]);
    let Item::Folder(auth) = &loaded.collection.items[0] else {
        panic!("expected the Auth folder");
    };
    assert_eq!(auth.variables, [Variable::plain("area", "auth")]);
    assert_eq!(auth.auth, Auth::None);
    assert_eq!(auth.certificate_check, CertificateCheck::Off);
    assert_eq!(auth.items[0].id(), login.id);

    let error = write_collection(&root, &collection).unwrap_err();
    assert!(
        matches!(error, StorageError::AlreadyExists { .. }),
        "{error}"
    );
}

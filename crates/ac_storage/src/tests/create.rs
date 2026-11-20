use super::*;
use crate::test_support::{names, read, shop_api};

#[test]
fn slugs_are_readable_file_names() {
    assert_eq!(slug("Get product"), "get-product");
    assert_eq!(slug("Größe ändern!"), "groesse-aendern");
    assert_eq!(slug("  --A  B--  "), "a-b");
    assert_eq!(slug("????"), "request");
    assert_eq!(slug(""), "request");
    assert_eq!(slug("CON"), "request-con");
    assert_eq!(slug(&"x".repeat(100)).len(), 60);
    assert_eq!(slug(&format!("{} y", "x".repeat(59))), "x".repeat(59));
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
        names(&loaded.collection.items),
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

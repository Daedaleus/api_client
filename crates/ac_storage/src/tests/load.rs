use ac_domain::collection::{Body, HeaderEntry};
use ac_domain::protocol::HttpMethod;

use super::*;
use crate::test_support::{request, shop_api, tree_names};

fn folder<'a>(items: &'a [Item], name: &str) -> &'a Folder {
    items
        .iter()
        .find_map(|item| match item {
            Item::Folder(folder) if folder.name == name => Some(folder),
            _ => None,
        })
        .unwrap_or_else(|| panic!("no folder {name:?}"))
}

#[test]
fn tree_follows_order_then_names_and_skips_hidden_reserved_and_other_files() {
    let (_temp, root) = shop_api();
    let loaded = load_collection(&root).unwrap();
    let collection = &loaded.collection;

    assert_eq!(collection.name, "Shop API");
    assert_eq!(
        collection.id.to_string(),
        "0193a8f2-7c1e-7b40-9d3a-5f2c8e1b4a70"
    );
    // Listed first ("missing.toml" does not exist), then the rest by name.
    assert_eq!(
        tree_names(&collection.items),
        ["Products/", "Health", "auth/"]
    );
    let products = folder(&collection.items, "Products");
    assert_eq!(
        tree_names(&products.items),
        ["List products", "Get product", "From the future"]
    );
    assert_eq!(
        tree_names(&folder(&collection.items, "auth").items),
        ["Login"]
    );
}

#[test]
fn damaged_and_foreign_files_are_problems_not_failures() {
    let (_temp, root) = shop_api();
    let loaded = load_collection(&root).unwrap();
    let mut problems: Vec<String> = loaded
        .problems
        .iter()
        .map(|problem| problem.error.to_string())
        .collect();
    problems.sort();
    assert_eq!(problems.len(), 2, "{problems:?}");
    assert!(problems[0].contains("broken.toml") && problems[0].contains("not valid TOML"));
    assert!(problems[1].contains("wrong.toml") && problems[1].contains("expected format"));
}

#[test]
fn entries_know_paths_kinds_and_read_only_files() {
    let (_temp, root) = shop_api();
    let loaded = load_collection(&root).unwrap();
    let entry = |name: &str| &loaded.entries[&request(&loaded.collection, name).id];

    assert_eq!(entry("Login").path, root.join("auth").join("login.toml"));
    assert_eq!(entry("Login").kind, EntryKind::Request);
    assert!(!entry("Login").read_only);
    assert!(entry("From the future").read_only);

    let root_entry = &loaded.entries[&loaded.collection.id];
    assert_eq!(root_entry.kind, EntryKind::Collection);
    assert_eq!(root_entry.path, root);
    let products = folder(&loaded.collection.items, "Products");
    assert_eq!(loaded.entries[&products.id].kind, EntryKind::Folder);
    // 1 collection + 2 folders + 5 requests + 1 environment.
    assert_eq!(loaded.entries.len(), 9);
}

#[test]
fn requests_are_read_with_their_details_and_defaults() {
    let (_temp, root) = shop_api();
    let loaded = load_collection(&root).unwrap();

    let list = request(&loaded.collection, "List products");
    assert_eq!(list.url, "{{baseUrl}}/products?limit=20");
    assert_eq!(
        list.headers,
        [
            HeaderEntry {
                name: "Accept".into(),
                value: "application/json".into(),
                enabled: true
            },
            HeaderEntry {
                name: "X-Debug".into(),
                value: "1".into(),
                enabled: false
            },
        ]
    );

    let login = request(&loaded.collection, "Login");
    assert_eq!(login.method, HttpMethod::Post);
    assert_eq!(login.headers[0].name, "Content-Type");
    assert_eq!(login.body, Body::Json("{ \"user\": \"demo\" }\n".into()));

    let health = request(&loaded.collection, "Health");
    assert_eq!(health.method, HttpMethod::Get);
    assert_eq!(health.url, "");
    assert_eq!(health.body, Body::None);
}

#[test]
fn only_folders_with_collection_toml_are_collections() {
    let (_temp, root) = shop_api();
    let error = load_collection(&root.join("products")).unwrap_err();
    assert!(
        matches!(error, StorageError::NotACollection { .. }),
        "{error}"
    );

    std::fs::write(
        root.join("collection.toml"),
        "format = \"api-client/folder\"\n",
    )
    .unwrap();
    let error = load_collection(&root).unwrap_err();
    assert!(matches!(error, StorageError::WrongFormat { .. }), "{error}");
}

#[test]
fn items_without_id_keep_the_same_id_across_loads() {
    let (_temp, root) = shop_api();
    let first = load_collection(&root).unwrap();
    let second = load_collection(&root).unwrap();
    let ids = |loaded: &LoadedCollection| {
        let health = request(&loaded.collection, "Health").id;
        let auth = folder(&loaded.collection.items, "auth").id;
        (health, auth)
    };
    assert_eq!(ids(&first), ids(&second));
}

#[test]
fn copied_files_get_their_own_id() {
    let (_temp, root) = shop_api();
    std::fs::copy(
        root.join("auth").join("login.toml"),
        root.join("auth").join("login-copy.toml"),
    )
    .unwrap();
    let loaded = load_collection(&root).unwrap();
    let auth = folder(&loaded.collection.items, "auth");
    assert_eq!(auth.items.len(), 2);
    assert_ne!(auth.items[0].id(), auth.items[1].id());
    assert_eq!(loaded.entries.len(), 10);
}

#[test]
fn damaged_folder_settings_still_show_the_folder_read_only() {
    let (_temp, root) = shop_api();
    std::fs::write(root.join("products").join("folder.toml"), "not toml [").unwrap();
    let loaded = load_collection(&root).unwrap();
    let products = folder(&loaded.collection.items, "products");
    assert!(loaded.entries[&products.id].read_only);
    assert_eq!(products.items.len(), 3);
    assert_eq!(loaded.problems.len(), 3);
}

#[test]
fn invalid_values_in_folder_settings_are_problems() {
    let (_temp, root) = shop_api();
    std::fs::write(
        root.join("products").join("folder.toml"),
        "format = \"api-client/folder\"\nversion = 1\norder = [1, 2]\n",
    )
    .unwrap();
    let loaded = load_collection(&root).unwrap();
    assert!(
        loaded
            .problems
            .iter()
            .any(|problem| problem.error.to_string().contains("order"))
    );
}

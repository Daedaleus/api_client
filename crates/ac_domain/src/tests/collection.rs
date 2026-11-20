use super::*;

#[test]
fn ids_are_unique_and_round_trip_as_text() {
    let first = ItemId::new();
    let second = ItemId::default();
    assert_ne!(first, second);

    let text = first.to_string();
    assert_eq!(text.len(), 36);
    assert_eq!(text.parse::<ItemId>().unwrap(), first);
}

#[test]
fn derived_ids_are_stable_per_key() {
    assert_eq!(
        ItemId::derived("auth/login.toml"),
        ItemId::derived("auth/login.toml")
    );
    assert_ne!(
        ItemId::derived("auth/login.toml"),
        ItemId::derived("auth/logout.toml")
    );
}

#[test]
fn ids_reject_other_text() {
    let error = "not-a-uuid".parse::<ItemId>().unwrap_err();
    assert_eq!(error, InvalidItemId("not-a-uuid".into()));
    assert!(error.to_string().contains("UUID"));
}

#[test]
fn new_requests_are_empty_gets() {
    let request = Request::new("Ping");
    assert_eq!(request.name, "Ping");
    assert_eq!(request.method, HttpMethod::Get);
    assert_eq!(request.url, "");
    assert_eq!(request.headers, vec![]);
    assert_eq!(request.body, Body::None);
}

#[test]
fn items_are_found_anywhere_in_the_tree() {
    let nested = Request::new("Nested");
    let top = Request::new("Top");
    let folder = Folder {
        variables: Vec::new(),
        id: ItemId::new(),
        name: "Folder".into(),
        items: vec![Item::Request(nested.clone())],
    };
    let collection = Collection {
        variables: Vec::new(),
        id: ItemId::new(),
        name: "C".into(),
        items: vec![Item::Folder(folder.clone()), Item::Request(top.clone())],
    };
    assert_eq!(collection.find(nested.id).map(Item::name), Some("Nested"));
    assert_eq!(collection.find(top.id).map(Item::name), Some("Top"));
    assert_eq!(collection.find(folder.id).map(Item::name), Some("Folder"));
    assert_eq!(collection.find(ItemId::new()), None);
}

#[test]
fn items_expose_id_and_name() {
    let request = Request::new("Login");
    let folder = Folder {
        variables: Vec::new(),
        id: ItemId::new(),
        name: "Auth".into(),
        items: vec![Item::Request(request.clone())],
    };
    let items = [Item::Folder(folder.clone()), Item::Request(request.clone())];
    assert_eq!(items[0].id(), folder.id);
    assert_eq!(items[0].name(), "Auth");
    assert_eq!(items[1].id(), request.id);
    assert_eq!(items[1].name(), "Login");
}

#[test]
fn folders_around_an_item_lead_from_outside_in() {
    let login = Request::new("Login");
    let inner = Folder {
        id: ItemId::new(),
        name: "v2".into(),
        items: vec![Item::Request(login.clone())],
        variables: Vec::new(),
    };
    let outer = Folder {
        id: ItemId::new(),
        name: "Auth".into(),
        items: vec![Item::Folder(inner.clone())],
        variables: Vec::new(),
    };
    let health = Request::new("Health");
    let collection = Collection {
        id: ItemId::new(),
        name: "Shop".into(),
        items: vec![Item::Folder(outer.clone()), Item::Request(health.clone())],
        variables: Vec::new(),
    };
    let names: Vec<_> = collection
        .folders_around(login.id)
        .iter()
        .map(|folder| folder.name.as_str())
        .collect();
    assert_eq!(names, ["Auth", "v2"]);
    assert_eq!(collection.folders_around(health.id).len(), 0);
    assert_eq!(collection.folders_around(ItemId::new()).len(), 0);
}

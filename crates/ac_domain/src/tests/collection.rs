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
fn items_expose_id_and_name() {
    let request = Request::new("Login");
    let folder = Folder {
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

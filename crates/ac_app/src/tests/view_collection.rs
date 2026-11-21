use ac_domain::collection::ItemId;
use ac_domain::protocol::HttpMethod;

use ac_domain::collection::Body;

use super::*;

fn request() -> Request {
    let mut request = Request::new("Login");
    request.method = HttpMethod::Post;
    request.url = "{{baseUrl}}/login".into();
    request.headers = vec![HeaderEntry {
        name: "Accept".into(),
        value: "application/json".into(),
        enabled: false,
    }];
    request.body = Body::Text("hi".into());
    request
}

#[test]
fn drafts_round_trip_to_requests() {
    for body in [Body::None, Body::Json("{}".into()), Body::Text("hi".into())] {
        let original = Request { body, ..request() };
        let draft = RequestDraft::from_request(&original, true);
        assert!(draft.read_only);
        assert_eq!(draft.to_request().unwrap(), original);
    }
}

#[test]
fn drafts_match_the_ui_contract() {
    let id = ItemId::new();
    let json = serde_json::json!({
        "id": id.to_string(),
        "name": "Login",
        "method": "POST",
        "url": "x",
        "headers": [{ "name": "A", "value": "b", "enabled": true }],
        "body": { "kind": "json", "content": "{}" }
    });
    // `readOnly` is optional when the UI sends a draft.
    let draft: RequestDraft = serde_json::from_value(json).unwrap();
    assert!(!draft.read_only);
    assert_eq!(draft.to_request().unwrap().id, id);
    assert_eq!(
        serde_json::to_value(&draft).unwrap()["body"],
        serde_json::json!({ "kind": "json", "content": "{}" })
    );
    assert_eq!(
        serde_json::to_value(BodyDraft::None).unwrap(),
        serde_json::json!({ "kind": "none" })
    );
}

#[test]
fn tree_items_match_the_ui_contract() {
    let item = TreeItemView::Request {
        id: "1".into(),
        name: "Login".into(),
        method: "POST".into(),
        read_only: false,
    };
    assert_eq!(
        serde_json::to_value(item).unwrap(),
        serde_json::json!({ "kind": "request", "id": "1", "name": "Login", "method": "POST", "readOnly": false })
    );
}

#[test]
fn names_are_trimmed_and_must_not_be_empty() {
    assert_eq!(non_empty("  a b ").unwrap(), "a b");
    assert_eq!(non_empty(" \t ").unwrap_err(), AppError::EmptyName);
}

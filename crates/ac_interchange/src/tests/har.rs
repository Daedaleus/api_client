use ac_domain::collection::{Body, FormField, HeaderEntry, Item, MultipartField, Request};
use ac_domain::protocol::HttpMethod;

use super::*;
use crate::import::{ImportError, import_collection};

const SHOP: &str = include_str!("fixtures/shop.har");

fn requests(imported: &Imported) -> Vec<&Request> {
    imported
        .collection
        .items
        .iter()
        .map(|item| match item {
            Item::Request(request) => request,
            Item::Folder(_) => panic!("HAR imports are flat"),
        })
        .collect()
}

fn header(name: &str, value: &str) -> HeaderEntry {
    HeaderEntry {
        name: name.into(),
        value: value.into(),
        enabled: true,
    }
}

#[test]
fn only_api_calls_are_taken_over_in_recorded_order() {
    let imported = import_collection(SHOP).unwrap();
    assert_eq!(imported.collection.name, "Tea shop");
    let requests = requests(&imported);
    let names: Vec<&str> = requests.iter().map(|r| r.name.as_str()).collect();
    assert_eq!(
        names,
        ["GET /teas", "POST /orders", "POST /login", "PUT /avatar"]
    );
    assert_eq!(
        imported.skipped,
        [
            Skipped {
                item: "Tea shop".into(),
                what: SkippedKind::Resources {
                    kind: "document".into(),
                    count: 1
                },
            },
            Skipped {
                item: "Tea shop".into(),
                what: SkippedKind::Resources {
                    kind: "image".into(),
                    count: 2
                },
            },
        ]
    );
}

#[test]
fn headers_set_by_the_client_are_left_out_and_bodies_kept() {
    let imported = import_collection(SHOP).unwrap();
    let requests = requests(&imported);
    let teas = requests[0];
    assert_eq!(teas.method, HttpMethod::Get);
    assert_eq!(teas.url, "https://api.example.com/teas?sort=name");
    assert_eq!(teas.headers, [header("Accept", "application/json")]);
    assert_eq!(teas.body, Body::None);

    let order = requests[1];
    assert_eq!(order.method, HttpMethod::Post);
    assert_eq!(order.headers, [header("Content-Type", "application/json")]);
    assert_eq!(order.body, Body::Json(r#"{"tea":"oolong"}"#.into()));

    assert_eq!(
        requests[2].body,
        Body::Form(vec![FormField {
            name: "user".into(),
            value: "ada".into(),
            enabled: true,
        }])
    );

    let avatar = requests[3];
    // The recorded boundary would not fit the new body.
    assert_eq!(avatar.headers, []);
    assert_eq!(
        avatar.body,
        Body::Multipart(vec![
            MultipartField {
                name: "title".into(),
                value: "Me".into(),
                enabled: true,
                file: false,
            },
            MultipartField {
                name: "image".into(),
                value: "me.png".into(),
                enabled: true,
                file: true,
            },
        ])
    );
}

#[test]
fn files_without_resource_types_keep_every_entry() {
    let har = r#"{"log": {"entries": [
        {"request": {"method": "DELETE", "url": "not a url"}},
        {"request": {"method": "POST", "url": "https://x.test/f",
            "postData": {"mimeType": "application/x-www-form-urlencoded", "text": "a=1&b=two%20words"}}},
        {"request": {"method": "POST", "url": "https://x.test/t",
            "postData": {"mimeType": "text/plain", "text": "hello"}}},
        {"request": {"method": "POST", "url": "https://x.test/e",
            "postData": {"mimeType": "text/plain", "text": ""}}},
        {"response": {}}
    ]}}"#;
    let imported = import_collection(har).unwrap();
    assert_eq!(imported.collection.name, "HAR");
    assert_eq!(imported.skipped, []);
    let requests = requests(&imported);
    assert_eq!(requests.len(), 4);
    assert_eq!(requests[0].name, "DELETE not a url");
    assert_eq!(
        requests[1].body,
        Body::Form(vec![
            FormField {
                name: "a".into(),
                value: "1".into(),
                enabled: true,
            },
            FormField {
                name: "b".into(),
                value: "two words".into(),
                enabled: true,
            },
        ])
    );
    assert_eq!(requests[2].body, Body::Text("hello".into()));
    assert_eq!(requests[3].body, Body::None);
}

#[test]
fn a_log_without_entries_is_no_har_file() {
    assert_eq!(
        import_collection(r#"{"log": {"version": "1.2"}}"#),
        Err(ImportError::Unknown)
    );
}

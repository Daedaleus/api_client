use ac_domain::auth::{Auth, KeyPlacement, OAuth2};
use ac_domain::collection::{Body, Folder, FormField, HeaderEntry, Item, MultipartField, Request};
use ac_domain::params::QueryParam;
use ac_domain::protocol::HttpMethod;
use ac_domain::transport::{Cookies, Redirects};

use super::*;
use crate::import::{ImportError, import_collection};

const SHOP: &str = include_str!("fixtures/shop.insomnia.json");

fn shop() -> Imported {
    import_collection(SHOP).unwrap()
}

fn folder<'a>(items: &'a [Item], name: &str) -> &'a Folder {
    items
        .iter()
        .find_map(|item| match item {
            Item::Folder(folder) if folder.name == name => Some(folder),
            _ => None,
        })
        .unwrap()
}

fn request<'a>(items: &'a [Item], name: &str) -> &'a Request {
    items
        .iter()
        .find_map(|item| match item {
            Item::Request(request) if request.name == name => Some(request),
            _ => None,
        })
        .unwrap()
}

fn names(items: &[Item]) -> Vec<&str> {
    items
        .iter()
        .map(|item| match item {
            Item::Folder(folder) => folder.name.as_str(),
            Item::Request(request) => request.name.as_str(),
        })
        .collect()
}

fn variable_pairs(variables: &[Variable]) -> Vec<(&str, &str)> {
    variables
        .iter()
        .map(|v| (v.name.as_str(), v.value.as_str()))
        .collect()
}

#[test]
fn the_tree_follows_parents_and_sort_keys() {
    let imported = shop();
    let collection = &imported.collection;
    assert_eq!(collection.name, "Tea shop");
    assert_eq!(
        names(&collection.items),
        ["Orders", "Login", "Upload", "Raw file", "Graph"]
    );
    let orders = folder(&collection.items, "Orders");
    assert_eq!(names(&orders.items), ["List orders", "Create order"]);
    assert_eq!(variable_pairs(&orders.variables), [("shop", "main")]);
    assert_eq!(
        orders.auth,
        Auth::Bearer {
            token: "{{token}}".into()
        }
    );
    assert_eq!(
        orders.scripts.pre_request,
        "insomnia.environment.set('x', 1);"
    );
}

#[test]
fn environments_become_variables_and_environments() {
    let imported = shop();
    assert_eq!(
        variable_pairs(&imported.collection.variables),
        [
            ("baseUrl", "https://api.example.com"),
            ("retries", "3"),
            ("nested", r#"{"a":1}"#),
        ]
    );
    assert_eq!(imported.environments.len(), 1);
    let staging = &imported.environments[0];
    assert_eq!(staging.name, "Staging");
    assert_eq!(
        variable_pairs(&staging.variables),
        [
            ("baseUrl", "https://staging.example.com"),
            ("token", "{{secret}}")
        ]
    );
}

#[test]
fn requests_keep_url_parameters_headers_body_and_settings() {
    let imported = shop();
    let orders = folder(&imported.collection.items, "Orders");
    let create = request(&orders.items, "Create order");
    assert_eq!(create.method, HttpMethod::Post);
    assert_eq!(create.url, "{{baseUrl}}/orders?dry=true");
    assert_eq!(
        create.disabled_query,
        [QueryParam {
            name: "debug".into(),
            value: "1".into()
        }]
    );
    assert_eq!(
        create.headers,
        [
            HeaderEntry {
                name: "Content-Type".into(),
                value: "application/json".into(),
                enabled: true,
            },
            HeaderEntry {
                name: "X-Old".into(),
                value: "1".into(),
                enabled: false,
            },
        ]
    );
    assert_eq!(create.body, Body::Json(r#"{"tea": "{{tea}}"}"#.into()));
    assert_eq!(create.auth, Auth::Inherit);
    assert_eq!(create.transport.redirects, Redirects::Stop);
    assert_eq!(create.transport.cookies, Cookies::Off);
    assert!(
        create
            .scripts
            .post_response
            .starts_with("insomnia.test('ok'")
    );

    let list = request(&orders.items, "List orders");
    // A switched-off auth sends nothing; the template tag stays as written.
    assert_eq!(list.auth, Auth::None);
    assert!(list.url.contains("{% response 'body'"));
    assert_eq!(list.transport.redirects, Redirects::Inherit);
    assert_eq!(list.transport.cookies, Cookies::Inherit);
}

#[test]
fn bodies_and_auth_types_are_converted() {
    let imported = shop();
    let items = &imported.collection.items;
    let login = request(items, "Login");
    assert_eq!(
        login.body,
        Body::Form(vec![
            FormField {
                name: "user".into(),
                value: "ada".into(),
                enabled: true,
            },
            FormField {
                name: "old".into(),
                value: "x".into(),
                enabled: false,
            },
        ])
    );
    assert_eq!(
        login.auth,
        Auth::OAuth2(OAuth2 {
            token_url: "https://auth.example.com/token".into(),
            client_id: "app".into(),
            client_secret: "s".into(),
            scope: "read".into(),
            ..Default::default()
        })
    );
    let upload = request(items, "Upload");
    assert_eq!(
        upload.body,
        Body::Multipart(vec![
            MultipartField {
                name: "title".into(),
                value: "Me".into(),
                enabled: true,
                file: false,
            },
            MultipartField {
                name: "image".into(),
                value: "/tmp/me.png".into(),
                enabled: true,
                file: true,
            },
        ])
    );
    assert_eq!(
        upload.auth,
        Auth::ApiKey {
            name: "X-Key".into(),
            value: "k".into(),
            placement: KeyPlacement::Query
        }
    );
    let raw = request(items, "Raw file");
    assert_eq!(raw.body, Body::Binary("data.bin".into()));
    assert_eq!(raw.auth, Auth::Inherit);
    assert_eq!(request(items, "Graph").body, Body::None);
}

#[test]
fn what_has_no_counterpart_is_reported() {
    let skipped: Vec<(String, SkippedKind)> = shop()
        .skipped
        .into_iter()
        .map(|skipped| (skipped.item, skipped.what))
        .collect();
    assert_eq!(
        skipped,
        [
            (
                "List orders".to_owned(),
                SkippedKind::TemplateTag("response".into())
            ),
            ("Raw file".to_owned(), SkippedKind::Auth("ntlm".into())),
            ("Graph".to_owned(), SkippedKind::Body("graphql".into())),
            ("Stream".to_owned(), SkippedKind::RequestKind("gRPC".into())),
        ]
    );
}

#[test]
fn authorization_code_and_exports_without_workspace() {
    let export = r#"{"_type": "export", "resources": [
        {"_id": "fld", "parentId": "x", "_type": "request_group", "name": "F", "metaSortKey": 2},
        {"_id": "r1", "parentId": "fld", "_type": "request", "name": "Inner", "url": "u"},
        {"_id": "r2", "parentId": "x", "_type": "request", "name": "Top", "metaSortKey": 1,
         "authentication": {"type": "oauth2", "grantType": "authorization_code",
            "authorizationUrl": "https://a/authorize", "accessTokenUrl": "https://a/token",
            "redirectUrl": "http://127.0.0.1:8123/cb", "clientId": "c"},
         "body": {"mimeType": "text/plain", "text": "hi {{ name }}"}},
        {"_id": "w", "parentId": "x", "_type": "websocket_request", "name": "Socket", "metaSortKey": 3}
    ]}"#;
    let imported = import_collection(export).unwrap();
    assert_eq!(imported.collection.name, "Insomnia");
    assert_eq!(names(&imported.collection.items), ["Top", "F"]);
    let top = request(&imported.collection.items, "Top");
    assert_eq!(top.body, Body::Text("hi {{name}}".into()));
    let Auth::OAuth2(oauth) = &top.auth else {
        panic!("{:?}", top.auth)
    };
    assert_eq!(oauth.auth_url, "https://a/authorize");
    assert_eq!(oauth.redirect_url, "http://127.0.0.1:8123/cb");
    assert_eq!(
        imported.skipped[0].what,
        SkippedKind::RequestKind("WebSocket".into())
    );
}

#[test]
fn digest_and_aws_iam_auth_are_imported() {
    let export = r#"{"_type": "export", "resources": [
        {"_id": "w", "_type": "workspace", "name": "W"},
        {"_id": "r1", "parentId": "w", "_type": "request", "name": "Digest", "metaSortKey": 1,
         "authentication": {"type": "digest", "username": "bob", "password": "{{ _.pw }}"}},
        {"_id": "r2", "parentId": "w", "_type": "request", "name": "Aws", "metaSortKey": 2,
         "authentication": {"type": "iam", "accessKeyId": "AKID", "secretAccessKey": "{{ _.awsSecret }}",
            "sessionToken": "", "region": "eu-central-1", "service": "s3"}}
    ]}"#;
    let imported = import_collection(export).unwrap();
    let items = &imported.collection.items;
    assert_eq!(
        request(items, "Digest").auth,
        Auth::Digest {
            username: "bob".into(),
            password: "{{pw}}".into(),
        }
    );
    assert_eq!(
        request(items, "Aws").auth,
        Auth::AwsSigV4(ac_domain::auth::AwsSigV4 {
            access_key_id: "AKID".into(),
            secret_access_key: "{{awsSecret}}".into(),
            session_token: String::new(),
            region: "eu-central-1".into(),
            service: "s3".into(),
        })
    );
    assert_eq!(imported.skipped, []);
}

#[test]
fn templates_convert_variables_and_find_tags() {
    assert_eq!(
        convert_template("{{ _.a }}/{{b}}/{% uuid 'v4' %}/{{ _.c"),
        (
            "{{a}}/{{b}}/{% uuid 'v4' %}/{{ _.c".to_owned(),
            vec!["uuid".to_owned()]
        )
    );
    assert_eq!(
        convert_template("{% now %} {% broken"),
        ("{% now %} {% broken".to_owned(), vec!["now".to_owned()])
    );
}

#[test]
fn format_5_gets_its_own_error() {
    assert_eq!(
        import_collection("type: collection.insomnia.rest/5.0\nname: x\n"),
        Err(ImportError::InsomniaV5)
    );
}

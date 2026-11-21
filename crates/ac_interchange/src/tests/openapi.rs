use ac_domain::collection::{Body, FormField, HeaderEntry, MultipartField, Request};
use ac_domain::protocol::HttpMethod;

use super::*;
use crate::import::{ImportError, SkippedKind, import_collection};

const SHOP: &str = include_str!("fixtures/shop.openapi.yaml");

fn shop() -> Imported {
    import_collection(SHOP).unwrap()
}

fn requests(items: &[Item]) -> Vec<&Request> {
    items
        .iter()
        .filter_map(|item| match item {
            Item::Request(request) => Some(request),
            Item::Folder(_) => None,
        })
        .collect()
}

fn folder<'a>(collection: &'a Collection, name: &str) -> &'a Folder {
    collection
        .items
        .iter()
        .find_map(|item| match item {
            Item::Folder(folder) if folder.name == name => Some(folder),
            _ => None,
        })
        .unwrap()
}

fn find<'a>(items: &'a [Item], name: &str) -> &'a Request {
    requests(items)
        .into_iter()
        .find(|request| request.name == name)
        .unwrap()
}

#[test]
fn operations_are_grouped_by_tag_in_the_declared_order() {
    let collection = shop().collection;
    assert_eq!(collection.name, "Shop API");
    let names: Vec<_> = collection.items.iter().map(Item::name).collect();
    assert_eq!(
        names,
        [
            "products",
            "auth",
            "Statistics",
            "OAuth only",
            "POST /proto"
        ]
    );
    let products: Vec<_> = folder(&collection, "products")
        .items
        .iter()
        .map(Item::name)
        .collect();
    assert_eq!(
        products,
        [
            "List products",
            "createProduct",
            "Get a product",
            "Replace the image",
            "Upload raw data"
        ]
    );
}

#[test]
fn the_server_becomes_base_url_and_security_the_collection_auth() {
    let collection = shop().collection;
    assert_eq!(
        collection.variables,
        [Variable::plain("baseUrl", "https://eu.api.example.com/v1")]
    );
    assert_eq!(
        collection.auth,
        Auth::Bearer {
            token: "{{token}}".into()
        }
    );
}

#[test]
fn parameters_become_variables_and_headers() {
    let collection = shop().collection;
    let products = &folder(&collection, "products").items;
    let list = find(products, "List products");
    assert_eq!(list.method, HttpMethod::Get);
    assert_eq!(list.url, "{{baseUrl}}/products?q={{q}}");
    assert_eq!(list.variables, [Variable::plain("q", "tea")]);
    assert_eq!(
        list.headers,
        [HeaderEntry {
            name: "X-Request-Id".into(),
            value: String::new(),
            enabled: false
        }]
    );
    assert_eq!(list.auth, Auth::Inherit);

    let get = find(products, "Get a product");
    assert_eq!(get.url, "{{baseUrl}}/products/{{productId}}");
    assert_eq!(get.variables, [Variable::plain("productId", "42")]);
    // The operation's own parameter replaces the path item's.
    let raw = find(products, "Upload raw data");
    assert_eq!(raw.variables, [Variable::plain("productId", "")]);
    assert_eq!(raw.body, Body::Binary(String::new()));
}

#[test]
fn bodies_get_examples_from_their_schemas() {
    let collection = shop().collection;
    let products = &folder(&collection, "products").items;
    let Body::Json(json) = &find(products, "createProduct").body else {
        panic!("expected a JSON body");
    };
    let value: serde_json::Value = serde_json::from_str(json).unwrap();
    assert_eq!(
        value,
        serde_json::json!({
            "name": "Teapot",
            "price": 0,
            "tags": ["string"],
            "createdAt": "2024-01-01T00:00:00Z",
            "status": "draft"
        })
    );
    assert_eq!(
        find(products, "Replace the image").body,
        Body::Multipart(vec![
            MultipartField {
                name: "title".into(),
                value: "Teapot".into(),
                enabled: true,
                file: false
            },
            MultipartField {
                name: "image".into(),
                value: String::new(),
                enabled: true,
                file: true
            },
        ])
    );
    let login = find(&folder(&collection, "auth").items, "Log in");
    assert_eq!(
        login.body,
        Body::Form(vec![
            FormField {
                name: "user".into(),
                value: "alice".into(),
                enabled: true
            },
            FormField {
                name: "password".into(),
                value: "string".into(),
                enabled: true
            },
        ])
    );
    assert_eq!(login.auth, Auth::None);
}

#[test]
fn other_security_and_unknown_bodies() {
    let imported = shop();
    let stats = find(&imported.collection.items, "Statistics");
    assert_eq!(
        stats.auth,
        Auth::ApiKey {
            name: "X-API-Key".into(),
            value: "{{apiKey}}".into(),
            placement: ac_domain::auth::KeyPlacement::Header,
        }
    );
    assert_eq!(
        find(&imported.collection.items, "OAuth only").auth,
        Auth::Inherit
    );
    assert_eq!(
        imported.skipped,
        [
            skipped("OAuth only", SkippedKind::Auth("oauth2".into())),
            skipped(
                "POST /proto",
                SkippedKind::Body("application/x-protobuf".into())
            ),
        ]
    );
}

#[test]
fn json_descriptions_3_1_and_odd_ones() {
    let imported = import_collection(
        r##"{"openapi": "3.1.0", "info": {}, "paths": {"/a": {"post": {
            "requestBody": {"content": {
                "application/json": {"examples": {"first": {"value": {"x": 1}}}}}},
            "security": [{"basic": []}]},
            "get": {"requestBody": {"content": {"text/plain": {"example": "hi"}}},
                    "parameters": [{"in": "query", "required": true, "name": "n",
                                    "schema": {"enum": ["a", "b"]}}]},
            "delete": {"requestBody": {"content": {"application/json": {}}}},
            "patch": {"requestBody": {"content": {"application/json": {"schema":
                {"type": ["null", "object"], "properties": {"self": {"$ref": "#/components/schemas/Loop"}}}}}}},
            "head": {"security": [{"key": []}]}}},
            "components": {"securitySchemes": {
                "basic": {"type": "http", "scheme": "basic"},
                "key": {"type": "apiKey", "in": "cookie", "name": "k"}},
              "schemas": {"Loop": {"type": "object", "properties": {"self": {"$ref": "#/components/schemas/Loop"}}}}}}"##,
    )
    .unwrap();
    let items = &imported.collection.items;
    assert_eq!(imported.collection.name, "OpenAPI");
    assert_eq!(
        imported.collection.variables,
        [Variable::plain("baseUrl", "http://localhost")]
    );
    let post = requests(items)[1];
    assert_eq!(post.body, Body::Json("{\n  \"x\": 1\n}".into()));
    assert_eq!(
        post.auth,
        Auth::Basic {
            username: "{{username}}".into(),
            password: "{{password}}".into()
        }
    );
    let get = requests(items)[0];
    assert_eq!(get.body, Body::Text("hi".into()));
    assert_eq!(get.variables, [Variable::plain("n", "a")]);
    assert_eq!(requests(items)[3].body, Body::Json(String::new()));
    assert!(matches!(&requests(items)[2].body, Body::Json(json) if json.contains("\"self\"")));
    assert_eq!(
        imported.skipped,
        [skipped(
            "HEAD /a",
            SkippedKind::Auth("apiKey in cookie".into())
        )]
    );

    assert_eq!(
        import_collection("swagger: '2.0'\ninfo: {}"),
        Err(ImportError::Swagger2)
    );
    assert_eq!(
        import_collection("openapi: 2.0\ninfo: {}"),
        Err(ImportError::Unknown)
    );
}

#[test]
fn client_credentials_flows_become_oauth2() {
    let imported = import_collection(
        r#"{"openapi": "3.0.3", "info": {"title": "T"}, "paths": {},
            "security": [{"oauth": ["read", "write"]}],
            "components": {"securitySchemes": {"oauth": {"type": "oauth2", "flows": {
                "clientCredentials": {"tokenUrl": "https://auth.example.com/token", "scopes": {}}}}}}}"#,
    )
    .unwrap();
    assert_eq!(
        imported.collection.auth,
        Auth::OAuth2(ac_domain::auth::OAuth2 {
            token_url: "https://auth.example.com/token".into(),
            client_id: "{{clientId}}".into(),
            client_secret: "{{clientSecret}}".into(),
            scope: "read write".into(),
            ..Default::default()
        })
    );
    assert_eq!(imported.skipped.len(), 0);
}

#[test]
fn authorization_code_flows_become_oauth2_sign_ins() {
    let imported = import_collection(
        r#"{"openapi": "3.0.3", "info": {"title": "T"}, "paths": {},
            "security": [{"oauth": ["openid"]}],
            "components": {"securitySchemes": {"oauth": {"type": "oauth2", "flows": {
                "authorizationCode": {"authorizationUrl": "https://auth.example.com/authorize",
                    "tokenUrl": "https://auth.example.com/token", "scopes": {}}}}}}}"#,
    )
    .unwrap();
    assert_eq!(
        imported.collection.auth,
        Auth::OAuth2(ac_domain::auth::OAuth2 {
            grant: ac_domain::auth::OAuth2Grant::AuthorizationCode,
            token_url: "https://auth.example.com/token".into(),
            auth_url: "https://auth.example.com/authorize".into(),
            client_id: "{{clientId}}".into(),
            client_secret: "{{clientSecret}}".into(),
            scope: "openid".into(),
            ..Default::default()
        })
    );
}

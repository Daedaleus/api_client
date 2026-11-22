use ac_domain::params::{PathParam, QueryParam};
use ac_domain::protocol::HttpMethod;

use super::*;
use crate::import::{ImportError, import_collection};

fn from_postman(text: &str) -> Result<Imported, ImportError> {
    import_collection(text)
}

const SHOP: &str = include_str!("fixtures/shop.postman_collection.json");

fn shop() -> Imported {
    from_postman(SHOP).unwrap()
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

#[test]
fn the_tree_keeps_folders_requests_and_order() {
    let imported = shop();
    let collection = &imported.collection;
    assert_eq!(collection.name, "Shop API");
    let names: Vec<_> = collection.items.iter().map(Item::name).collect();
    assert_eq!(names, ["Auth", "Products", "Health", "Upload binary"]);
    let products = folder(&collection.items, "Products");
    let names: Vec<_> = products.items.iter().map(Item::name).collect();
    assert_eq!(
        names,
        ["List products", "Upload image", "Search", "GraphQL"]
    );
}

#[test]
fn variables_and_auth_of_collection_and_folders() {
    let collection = shop().collection;
    assert_eq!(
        collection.variables,
        [
            Variable::plain("baseUrl", "https://api.example.com"),
            Variable::plain("retries", "3"),
            Variable {
                enabled: false,
                ..Variable::plain("legacy", "x")
            },
        ]
    );
    assert_eq!(
        collection.auth,
        Auth::Bearer {
            token: "{{token}}".into()
        }
    );
    let auth = folder(&collection.items, "Auth");
    assert_eq!(auth.variables, [Variable::plain("area", "auth")]);
    assert_eq!(auth.auth, Auth::Inherit);
    assert_eq!(
        folder(&collection.items, "Products").auth,
        Auth::ApiKey {
            name: "X-API-Key".into(),
            value: "{{apiKey}}".into(),
            placement: KeyPlacement::Header,
        }
    );
}

#[test]
fn requests_with_their_bodies() {
    let collection = shop().collection;
    let login = request(&folder(&collection.items, "Auth").items, "Login");
    assert_eq!(login.method, HttpMethod::Post);
    assert_eq!(login.url, "{{baseUrl}}/auth/login");
    assert_eq!(login.auth, Auth::None);
    assert_eq!(
        login.headers,
        [HeaderEntry {
            name: "Accept".into(),
            value: "application/json".into(),
            enabled: true
        }]
    );
    assert!(matches!(&login.body, Body::Json(json) if json.contains("{{password}}")));

    let products = &folder(&collection.items, "Products").items;
    let list = request(products, "List products");
    assert_eq!(list.url, "{{baseUrl}}/products?page=1");
    assert!(!list.headers[0].enabled);
    assert_eq!(list.body, Body::None);
    assert_eq!(list.auth, Auth::Inherit);

    assert_eq!(
        request(products, "Upload image").body,
        Body::Multipart(vec![
            MultipartField {
                name: "title".into(),
                value: "Teapot".into(),
                enabled: true,
                file: false
            },
            MultipartField {
                name: "image".into(),
                value: "/Users/alice/teapot.png".into(),
                enabled: true,
                file: true
            },
            MultipartField {
                name: "note".into(),
                value: "x".into(),
                enabled: false,
                file: false
            },
        ])
    );
    assert_eq!(
        request(products, "Search").body,
        Body::Form(vec![FormField {
            name: "q".into(),
            value: "tea".into(),
            enabled: true
        }])
    );
    let health = request(&collection.items, "Health");
    assert_eq!(health.url, "{{baseUrl}}/health");
    assert_eq!(health.method, HttpMethod::Get);
    assert_eq!(
        request(&collection.items, "Upload binary").body,
        Body::Binary("data/payload.bin".into())
    );
}

#[test]
fn what_has_no_counterpart_is_reported() {
    let imported = shop();
    assert_eq!(
        imported.skipped,
        [
            Skipped {
                item: "GraphQL".into(),
                what: SkippedKind::Body("graphql".into())
            },
            Skipped {
                item: "GraphQL".into(),
                what: SkippedKind::Auth("oauth2".into())
            },
        ]
    );
    // The collection's empty pre-request script is no script.
    let graphql = request(
        &folder(&imported.collection.items, "Products").items,
        "GraphQL",
    );
    assert_eq!(graphql.body, Body::None);
    assert_eq!(graphql.auth, Auth::Inherit);
}

#[test]
fn scripts_are_taken_over() {
    let imported = shop();
    let login = request(&folder(&imported.collection.items, "Auth").items, "Login");
    assert_eq!(
        login.scripts.post_response,
        "pm.environment.set(\"token\", pm.response.json().token);"
    );
    assert_eq!(login.scripts.pre_request, "");
    // The collection's pre-request script is empty: none.
    assert!(imported.collection.scripts.is_empty());

    let text = from_postman(
        r#"{"info": {}, "event": [{"listen": "prerequest", "script": {"exec": "log(1);"}}], "item": [{"name": "F", "event": [{"listen": "test", "script": {"exec": ["a();", "b();"]}}, {"listen": "test", "script": {"exec": ["c();"]}}, {"listen": "other", "script": {"exec": ["x"]}}], "item": []}]}"#,
    )
    .unwrap();
    assert_eq!(text.collection.scripts.pre_request, "log(1);");
    let Item::Folder(folder) = &text.collection.items[0] else {
        panic!("expected a folder");
    };
    assert_eq!(
        folder.scripts.post_response,
        "a();
b();

c();"
    );
    assert_eq!(folder.scripts.pre_request, "");
}

#[test]
fn path_variables_and_switched_off_query_parameters() {
    let imported = from_postman(
        r#"{"info": {}, "item": [{"name": "User", "request": {"url": {"raw": "{{base}}/users/:id?page=2", "query": [{"key": "page", "value": "2"}, {"key": "debug", "value": "1", "disabled": true}], "variable": [{"key": "id", "value": "7"}]}}}]}"#,
    )
    .unwrap();
    let user = request(&imported.collection.items, "User");
    assert_eq!(user.url, "{{base}}/users/:id?page=2");
    assert_eq!(
        user.path_params,
        [PathParam {
            name: "id".into(),
            value: "7".into()
        }]
    );
    assert_eq!(
        user.disabled_query,
        [QueryParam {
            name: "debug".into(),
            value: "1".into()
        }]
    );
}

#[test]
fn small_and_odd_exports() {
    let minimal = from_postman(r#"{"info": {}, "item": [{"name": "Ping"}, {"name": "Raw", "request": {"method": "BAD METHOD", "body": {"mode": "raw", "raw": "hi"}}}]}"#)
        .unwrap();
    assert_eq!(minimal.collection.name, "Postman");
    let ping = request(&minimal.collection.items, "Ping");
    assert_eq!(ping.url, "");
    let raw = request(&minimal.collection.items, "Raw");
    assert_eq!(raw.method, HttpMethod::Get);
    assert_eq!(raw.body, Body::Text("hi".into()));
    assert_eq!(raw.url, "");

    let files = from_postman(
        r#"{"info": {"name": "F"}, "item": [{"name": "A", "request": {"body": {"mode": "file", "file": {}}, "auth": {"type": "basic", "basic": [{"key": "username", "value": "u"}]}}}, {"name": "B", "request": {"auth": {"type": "apikey", "apikey": [{"key": "in", "value": "query"}]}, "body": {"mode": "formdata", "formdata": [{"key": "f", "type": "file", "src": 3}]}}}], "variable": "nope"}"#,
    )
    .unwrap();
    let a = request(&files.collection.items, "A");
    assert_eq!(a.body, Body::Binary(String::new()));
    assert_eq!(
        a.auth,
        Auth::Basic {
            username: "u".into(),
            password: String::new()
        }
    );
    let b = request(&files.collection.items, "B");
    assert!(matches!(
        &b.auth,
        Auth::ApiKey {
            placement: KeyPlacement::Query,
            ..
        }
    ));
    assert!(matches!(&b.body, Body::Multipart(fields) if fields[0].value.is_empty()));
    assert_eq!(files.collection.variables, []);

    assert!(matches!(from_postman("{nope"), Err(ImportError::Parse(_))));
    assert_eq!(from_postman("{}"), Err(ImportError::Unknown));
    assert_eq!(
        from_postman(r#"{"info": {}, "item": 1}"#)
            .unwrap()
            .collection
            .items,
        []
    );
}

#[test]
fn oauth2_client_credentials_are_imported() {
    let imported = from_postman(
        r#"{"info": {"name": "T", "schema": "https://schema.getpostman.com/json/collection/v2.1.0/collection.json"},
            "auth": {"type": "oauth2", "oauth2": [
                {"key": "grant_type", "value": "client_credentials"},
                {"key": "accessTokenUrl", "value": "{{auth}}/token"},
                {"key": "clientId", "value": "app"},
                {"key": "clientSecret", "value": "{{secret}}"},
                {"key": "scope", "value": "read"}]},
            "item": []}"#,
    )
    .unwrap();
    assert_eq!(
        imported.collection.auth,
        Auth::OAuth2(ac_domain::auth::OAuth2 {
            token_url: "{{auth}}/token".into(),
            client_id: "app".into(),
            client_secret: "{{secret}}".into(),
            scope: "read".into(),
            ..Default::default()
        })
    );
    assert_eq!(imported.skipped.len(), 0);
}

#[test]
fn oauth2_authorization_code_is_imported_with_a_loopback_redirect_only() {
    let collection = |redirect: &str| {
        format!(
            r#"{{"info": {{"name": "T", "schema": "https://schema.getpostman.com/json/collection/v2.1.0/collection.json"}},
            "auth": {{"type": "oauth2", "oauth2": [
                {{"key": "grant_type", "value": "authorization_code_with_pkce"}},
                {{"key": "authUrl", "value": "https://auth.example.com/authorize"}},
                {{"key": "accessTokenUrl", "value": "https://auth.example.com/token"}},
                {{"key": "redirect_uri", "value": "{redirect}"}},
                {{"key": "clientId", "value": "app"}}]}},
            "item": []}}"#
        )
    };
    let imported = from_postman(&collection("http://localhost:8080/cb")).unwrap();
    assert_eq!(
        imported.collection.auth,
        Auth::OAuth2(ac_domain::auth::OAuth2 {
            grant: ac_domain::auth::OAuth2Grant::AuthorizationCode,
            token_url: "https://auth.example.com/token".into(),
            auth_url: "https://auth.example.com/authorize".into(),
            redirect_url: "http://localhost:8080/cb".into(),
            client_id: "app".into(),
            ..Default::default()
        })
    );
    // Postman's own web callback cannot reach the app: any free port instead.
    let web = from_postman(&collection("https://oauth.pstmn.io/v1/callback")).unwrap();
    let Auth::OAuth2(oauth) = web.collection.auth else {
        panic!("expected OAuth 2");
    };
    assert_eq!(oauth.redirect_url, "");
}

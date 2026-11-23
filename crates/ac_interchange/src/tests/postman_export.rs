use ac_domain::auth::{Auth, KeyPlacement, OAuth2, OAuth2Grant};
use ac_domain::collection::{
    Body, Collection, Folder, FormField, HeaderEntry, Item, ItemId, MultipartField, Request,
};
use ac_domain::environment::Variable;
use ac_domain::params::{PathParam, QueryParam};
use ac_domain::script::Scripts;
use ac_domain::tls::CertificateCheck;
use ac_domain::transport::{Cookies, Redirects, TransportSettings};
use serde_json::{Value, json};

use super::*;
use crate::import::{ItemSetting, import_collection};

const SHOP: &str = include_str!("fixtures/shop.postman_collection.json");

fn exported(collection: &Collection) -> Value {
    serde_json::from_str(&to_postman(collection).text).unwrap()
}

fn login() -> Request {
    let mut login = Request::new("Login");
    login.method = "POST".parse().unwrap();
    login.url = "{{baseUrl}}/users/:id?verbose=1&flag#top".into();
    login.path_params = vec![PathParam {
        name: "id".into(),
        value: "7".into(),
    }];
    login.disabled_query = vec![QueryParam {
        name: "debug".into(),
        value: "1".into(),
    }];
    login.headers = vec![HeaderEntry {
        name: "X-Old".into(),
        value: "1".into(),
        enabled: false,
    }];
    login.body = Body::Json(r#"{"a":1}"#.into());
    login.auth = Auth::Basic {
        username: "ada".into(),
        password: "{{password}}".into(),
    };
    login.docs = "Signs in.".into();
    login.examples = vec![ac_domain::example::ResponseExample {
        name: "OK".into(),
        status: ac_domain::protocol::StatusCode::new(200).unwrap(),
        headers: vec![ac_domain::exchange::Header::new(
            "Content-Type",
            "application/json",
        )],
        body: "{\"token\": \"t\"}".into(),
    }];
    login.scripts = Scripts {
        pre_request: "pm.variables.set('a', 1);\nconsole.log('x');".into(),
        post_response: String::new(),
    };
    login.transport = TransportSettings {
        redirects: Redirects::Stop,
        timeout_seconds: Some(5),
        cookies: Cookies::Off,
    };
    login.certificate_check = CertificateCheck::Off;
    login
}

fn sample() -> Collection {
    let login = login();
    let mut upload = Request::new("Upload");
    upload.body = Body::Multipart(vec![
        MultipartField {
            name: "title".into(),
            value: "Me".into(),
            enabled: true,
            file: false,
        },
        MultipartField {
            name: "image".into(),
            value: "img/me.png".into(),
            enabled: false,
            file: true,
        },
    ]);
    upload.auth = Auth::ApiKey {
        name: "X-Key".into(),
        value: "k".into(),
        placement: KeyPlacement::Query,
    };
    let mut form = Request::new("Form");
    form.body = Body::Form(vec![FormField {
        name: "user".into(),
        value: "ada".into(),
        enabled: true,
    }]);
    form.auth = Auth::None;
    let mut file = Request::new("File");
    file.body = Body::Binary("data.bin".into());
    file.auth = Auth::Bearer {
        token: "{{token}}".into(),
    };
    let mut text = Request::new("Text");
    text.body = Body::Text("hello".into());
    text.auth = Auth::OAuth2(OAuth2 {
        grant: OAuth2Grant::AuthorizationCode,
        token_url: "https://a/token".into(),
        auth_url: "https://a/authorize".into(),
        redirect_url: "http://127.0.0.1:8123/cb".into(),
        client_id: "c".into(),
        ..Default::default()
    });

    let folder = Folder {
        id: ItemId::new(),
        name: "Users".into(),
        items: vec![Item::Request(login), Item::Request(upload)],
        variables: vec![Variable::plain("shop", "main")],
        auth: Auth::Inherit,
        certificate_check: CertificateCheck::On,
        transport: TransportSettings {
            redirects: Redirects::Follow,
            timeout_seconds: None,
            cookies: Cookies::Inherit,
        },
        scripts: Scripts::default(),
        docs: "Folder docs.".into(),
    };
    Collection {
        id: ItemId::new(),
        name: "Tea shop".into(),
        items: vec![
            Item::Folder(folder),
            Item::Request(form),
            Item::Request(file),
            Item::Request(text),
        ],
        variables: vec![
            Variable::plain("baseUrl", "https://api.example.com"),
            Variable {
                secret: true,
                ..Variable::plain("password", "hunter2")
            },
            Variable {
                enabled: false,
                ..Variable::plain("old", "x")
            },
        ],
        auth: Auth::Inherit,
        certificate_check: CertificateCheck::Off,
        transport: TransportSettings {
            redirects: Redirects::Stop,
            timeout_seconds: None,
            cookies: Cookies::Inherit,
        },
        scripts: Scripts {
            pre_request: String::new(),
            post_response: "pm.test('ok', () => {});".into(),
        },
        docs: "# Tea shop\n\nThe API.".into(),
    }
}

#[test]
fn an_imported_collection_exports_and_imports_unchanged() {
    let first = import_collection(SHOP).unwrap().collection;
    let text = to_postman(&first).text;
    let second = import_collection(&text).unwrap().collection;
    assert_eq!(to_postman(&second).text, text);
    assert!(text.ends_with("}\n"));

    let sample = sample();
    let text = to_postman(&sample).text;
    let again = import_collection(&text).unwrap().collection;
    assert_eq!(to_postman(&again).text, text);
}

#[test]
fn the_collection_carries_schema_variables_and_scripts() {
    let root = exported(&sample());
    assert_eq!(root["info"]["name"], "Tea shop");
    assert_eq!(root["info"]["schema"], SCHEMA);
    assert!(root.get("auth").is_none());
    assert_eq!(
        root["variable"],
        json!([
            { "key": "baseUrl", "value": "https://api.example.com", "type": "string" },
            // Never the secret value.
            { "key": "password", "value": "", "type": "secret" },
            { "key": "old", "value": "x", "disabled": true, "type": "string" },
        ])
    );
    assert_eq!(
        root["event"],
        json!([{ "listen": "test", "script": {
            "type": "text/javascript", "exec": ["pm.test('ok', () => {});"]
        } }])
    );
    assert!(!to_postman(&sample()).text.contains("hunter2"));
}

#[test]
fn requests_carry_url_parts_headers_body_auth_and_settings() {
    let root = exported(&sample());
    let folder = &root["item"][0];
    assert_eq!(folder["name"], "Users");
    assert_eq!(
        folder["protocolProfileBehavior"],
        json!({ "followRedirects": true, "strictSSL": true })
    );
    let login = &folder["item"][0];
    assert_eq!(
        login["protocolProfileBehavior"],
        json!({ "followRedirects": false, "strictSSL": false, "disableCookies": true })
    );
    assert_eq!(
        login["event"][0]["script"]["exec"],
        json!(["pm.variables.set('a', 1);", "console.log('x');"])
    );
    let request = &login["request"];
    assert_eq!(request["method"], "POST");
    assert_eq!(
        request["url"],
        json!({
            "raw": "{{baseUrl}}/users/:id?verbose=1&flag#top",
            "query": [
                { "key": "verbose", "value": "1" },
                { "key": "flag", "value": "" },
                { "key": "debug", "value": "1", "disabled": true },
            ],
            "variable": [{ "key": "id", "value": "7" }],
        })
    );
    assert_eq!(
        request["header"],
        json!([{ "key": "X-Old", "value": "1", "disabled": true }])
    );
    assert_eq!(request["body"]["mode"], "raw");
    assert_eq!(request["body"]["options"]["raw"]["language"], "json");
    assert_eq!(request["auth"]["type"], "basic");
    assert_eq!(request["auth"]["basic"][1]["value"], "{{password}}");

    let upload = &folder["item"][1]["request"];
    assert_eq!(
        upload["body"]["formdata"],
        json!([
            { "key": "title", "type": "text", "value": "Me" },
            { "key": "image", "type": "file", "src": "img/me.png", "disabled": true },
        ])
    );
    assert_eq!(upload["auth"]["apikey"][2]["value"], "query");
    assert!(upload["url"].get("query").is_none());

    assert_eq!(
        root["item"][1]["request"]["auth"],
        json!({ "type": "noauth" })
    );
    assert_eq!(root["item"][1]["request"]["body"]["mode"], "urlencoded");
    assert_eq!(
        root["item"][2]["request"]["body"],
        json!({ "mode": "file", "file": { "src": "data.bin" } })
    );
    let text = &root["item"][3]["request"];
    assert_eq!(text["body"]["options"]["raw"]["language"], "text");
    assert_eq!(text["auth"]["oauth2"][0]["value"], "authorization_code");
}

#[test]
fn what_postman_has_no_place_for_is_reported() {
    let left_out: Vec<(String, SkippedKind)> = to_postman(&sample())
        .left_out
        .into_iter()
        .map(|skipped| (skipped.item, skipped.what))
        .collect();
    assert_eq!(
        left_out,
        [
            (
                "Login".to_owned(),
                SkippedKind::Setting(ItemSetting::Timeout)
            ),
            (
                "Tea shop".to_owned(),
                SkippedKind::SecretValue("password".into())
            ),
            (
                "Tea shop".to_owned(),
                SkippedKind::Setting(ItemSetting::Redirects)
            ),
            (
                "Tea shop".to_owned(),
                SkippedKind::Setting(ItemSetting::CertificateCheck)
            ),
        ]
    );
}

#[test]
fn docs_become_descriptions() {
    let root = exported(&sample());
    assert_eq!(root["info"]["description"], "# Tea shop\n\nThe API.");
    assert_eq!(root["item"][0]["description"], "Folder docs.");
    assert_eq!(
        root["item"][0]["item"][0]["request"]["description"],
        "Signs in."
    );
    assert!(root["item"][1].get("description").is_none());
}

#[test]
fn examples_become_saved_responses() {
    let root = exported(&sample());
    let login = &root["item"][0]["item"][0];
    assert_eq!(
        login["response"],
        json!([{
            "name": "OK",
            "code": 200,
            "header": [{ "key": "Content-Type", "value": "application/json" }],
            "body": "{\"token\": \"t\"}",
        }])
    );
    assert!(root["item"][0]["item"][1].get("response").is_none());
}

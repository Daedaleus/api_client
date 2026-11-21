use crate::command::{Query, QueryResult};
use crate::error::AppError;
use crate::test_support::Fixture;
use crate::text::keys;
use crate::view::{AuthDraft, BodyDraft, OutgoingRequest, SendContext};

fn curl(fixture: &Fixture, request: OutgoingRequest) -> Result<String, AppError> {
    match fixture.app.query(Query::Curl {
        request: Box::new(request),
    })? {
        QueryResult::Curl { command } => Ok(command),
        other => panic!("expected a command, got {other:?}"),
    }
}

#[tokio::test]
async fn the_curl_command_is_what_would_be_sent() {
    let fixture = Fixture::new();
    let mut request = OutgoingRequest::get("{{baseUrl}}/login");
    request.method = "POST".into();
    request.body = BodyDraft::Json {
        content: "{}".into(),
    };
    request.auth = AuthDraft::Bearer {
        token: "t0ken".into(),
    };
    request.context = Some(SendContext {
        collection_id: fixture.collection.clone(),
        request_id: Some(fixture.login.clone()),
        environment_id: Some(fixture.dev.clone()),
    });
    let command = curl(&fixture, request).unwrap();
    assert!(
        command.contains("'https://dev.example.com/login'"),
        "{command}"
    );
    assert!(
        command.contains("--header 'authorization: Bearer t0ken'"),
        "{command}"
    );
    assert!(command.contains("--data-raw '{}'"), "{command}");
    assert!(!command.contains("--request"), "{command}");

    // Errors are those of sending.
    let error = curl(&fixture, OutgoingRequest::get("{{nope}}")).unwrap_err();
    assert_eq!(error.user_text().key, keys::VARIABLE_UNKNOWN);
    assert_eq!(fixture.http.sent().len(), 0);
}

#[tokio::test]
async fn a_pasted_curl_command_becomes_an_unsaved_request() {
    let fixture = Fixture::new();
    let result = fixture
        .app
        .query(Query::ParseCurl {
            text: "curl -X PUT https://x.example -H 'A: 1' --compressed".into(),
        })
        .unwrap();
    let QueryResult::ParsedCurl { request, ignored } = result else {
        panic!("expected a parsed request");
    };
    assert_eq!(request.id, "");
    assert_eq!(request.method, "PUT");
    assert_eq!(request.url, "https://x.example");
    assert_eq!(request.headers.len(), 1);
    assert_eq!(ignored, ["--compressed"]);

    let error = fixture
        .app
        .query(Query::ParseCurl {
            text: "wget x".into(),
        })
        .unwrap_err();
    assert_eq!(error.user_text().key, keys::CURL_INVALID);
    assert_eq!(error.user_text().args["detail"], "not a curl command");
}

const POSTMAN: &str =
    include_str!("../../../ac_interchange/src/tests/fixtures/shop.postman_collection.json");

#[tokio::test]
async fn postman_exports_become_new_open_collections() {
    use crate::command::{Command, CommandAck};

    let fixture = Fixture::new();
    fixture.store.add_file("export.json", POSTMAN);
    let ack = fixture
        .app
        .execute(Command::ImportCollection {
            file: "export.json".into(),
            path: "imported".into(),
        })
        .unwrap();
    let CommandAck::CollectionImported {
        collection,
        skipped,
    } = ack
    else {
        panic!("expected an imported collection");
    };
    assert_eq!(collection.name, "Shop API");
    assert_eq!(collection.items.len(), 4);
    let keys: Vec<_> = skipped.iter().map(|text| text.key).collect();
    assert_eq!(
        keys,
        [
            keys::IMPORT_SKIPPED_SCRIPTS,
            keys::IMPORT_SKIPPED_BODY,
            keys::IMPORT_SKIPPED_AUTH
        ]
    );
    assert_eq!(skipped[2].args["kind"], "oauth2");
    assert_eq!(skipped[1].args["item"], "GraphQL");
    // It is open like any other collection.
    let tree = fixture
        .app
        .query(Query::Collection {
            collection_id: collection.id.clone(),
        })
        .unwrap();
    assert!(matches!(tree, QueryResult::Collection { .. }));

    let error = fixture
        .app
        .execute(Command::ImportCollection {
            file: "export.json".into(),
            path: "imported".into(),
        })
        .unwrap_err();
    assert_eq!(error.user_text().key, keys::COLLECTION_EXISTS);
    fixture.store.add_file("broken.json", "{");
    let error = fixture
        .app
        .execute(Command::ImportCollection {
            file: "broken.json".into(),
            path: "other".into(),
        })
        .unwrap_err();
    assert_eq!(error.user_text().key, keys::IMPORT_INVALID);
    let error = fixture
        .app
        .execute(Command::ImportCollection {
            file: "missing.json".into(),
            path: "other".into(),
        })
        .unwrap_err();
    assert_eq!(error.user_text().key, keys::STORE_IO);
}

const OPENAPI: &str = include_str!("../../../ac_interchange/src/tests/fixtures/shop.openapi.yaml");

#[tokio::test]
async fn openapi_descriptions_are_imported_the_same_way() {
    use crate::command::{Command, CommandAck};

    let fixture = Fixture::new();
    fixture.store.add_file("shop.yaml", OPENAPI);
    let ack = fixture
        .app
        .execute(Command::ImportCollection {
            file: "shop.yaml".into(),
            path: "from-openapi".into(),
        })
        .unwrap();
    let CommandAck::CollectionImported {
        collection,
        skipped,
    } = ack
    else {
        panic!("expected an imported collection");
    };
    assert_eq!(collection.name, "Shop API");
    assert_eq!(collection.variables[0].name, "baseUrl");
    assert_eq!(skipped.len(), 2);

    fixture.store.add_file("old.yaml", "swagger: '2.0'");
    let error = fixture
        .app
        .execute(Command::ImportCollection {
            file: "old.yaml".into(),
            path: "old".into(),
        })
        .unwrap_err();
    assert_eq!(error.user_text().key, keys::IMPORT_INVALID);
    assert!(error.user_text().args["detail"].contains("Swagger 2.0"));
}

#[tokio::test]
async fn bruno_collections_are_imported_from_their_folder() {
    use crate::command::{Command, CommandAck};

    let fixture = Fixture::new();
    let source = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../ac_interchange/src/tests/fixtures/bruno-shop");
    for relative in [
        "bruno.json",
        "collection.bru",
        "environments/Local.bru",
        "auth/folder.bru",
        "auth/login.bru",
        "health.bru",
    ] {
        let text = std::fs::read_to_string(source.join(relative)).unwrap();
        fixture
            .store
            .add_file(std::path::Path::new("bruno").join(relative), text);
    }
    let ack = fixture
        .app
        .execute(Command::ImportCollection {
            file: std::path::Path::new("bruno")
                .join("bruno.json")
                .display()
                .to_string(),
            path: "from-bruno".into(),
        })
        .unwrap();
    let CommandAck::CollectionImported {
        collection,
        skipped,
    } = ack
    else {
        panic!("expected an imported collection");
    };
    assert_eq!(collection.name, "Shop (Bruno)");
    assert_eq!(collection.environments.len(), 1);
    assert_eq!(collection.environments[0].name, "Local");
    let keys: Vec<_> = skipped.iter().map(|text| text.key).collect();
    assert!(keys.contains(&keys::IMPORT_SKIPPED_HEADERS), "{keys:?}");

    let error = fixture
        .app
        .execute(Command::ImportCollection {
            file: std::path::Path::new("empty")
                .join("bruno.json")
                .display()
                .to_string(),
            path: "other".into(),
        })
        .unwrap_err();
    assert_eq!(error.user_text().key, keys::STORE_IO);

    // A folder with requests but without bruno.json is no Bruno collection.
    fixture
        .store
        .add_file(std::path::Path::new("loose").join("health.bru"), "get {\n}");
    let error = fixture
        .app
        .execute(Command::ImportCollection {
            file: std::path::Path::new("loose")
                .join("bruno.json")
                .display()
                .to_string(),
            path: "other".into(),
        })
        .unwrap_err();
    assert_eq!(error.user_text().key, keys::IMPORT_INVALID);
    assert!(error.to_string().contains("bruno.json"), "{error}");
}

#[tokio::test]
async fn the_curl_command_holds_a_placeholder_for_an_oauth_token() {
    let fixture = Fixture::new();
    let mut request = OutgoingRequest::get("https://api.example.com");
    request.auth = AuthDraft::OAuth2 {
        grant: crate::view::OAuth2GrantView::ClientCredentials,
        auth_url: String::new(),
        redirect_url: String::new(),
        token_url: "https://auth.example.com/token".into(),
        client_id: "app".into(),
        client_secret: "pw".into(),
        scope: String::new(),
    };
    let command = curl(&fixture, request).unwrap();
    assert!(
        command.contains("--header 'authorization: Bearer <access token>'"),
        "{command}"
    );
    assert!(!command.contains("pw"), "{command}");
    assert_eq!(fixture.http.sent().len(), 0);
}

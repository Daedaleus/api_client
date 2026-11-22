use super::skipped_text;
use ac_interchange::{Skipped, SkippedKind};

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
            path: Some("imported".into()),
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
    assert_eq!(keys, [keys::IMPORT_SKIPPED_BODY, keys::IMPORT_SKIPPED_AUTH]);
    assert_eq!(skipped[1].args["kind"], "oauth2");
    assert_eq!(skipped[0].args["item"], "GraphQL");
    let assertion = skipped_text(&Skipped {
        item: "Login".into(),
        what: SkippedKind::Assertion("res.status: between 1 2".into()),
    });
    assert_eq!(assertion.key, keys::IMPORT_SKIPPED_ASSERTION);
    assert_eq!(assertion.args["assertion"], "res.status: between 1 2");
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
            path: Some("imported".into()),
        })
        .unwrap_err();
    assert_eq!(error.user_text().key, keys::COLLECTION_EXISTS);
    fixture.store.add_file("broken.json", "{");
    let error = fixture
        .app
        .execute(Command::ImportCollection {
            file: "broken.json".into(),
            path: Some("other".into()),
        })
        .unwrap_err();
    assert_eq!(error.user_text().key, keys::IMPORT_INVALID);
    let error = fixture
        .app
        .execute(Command::ImportCollection {
            file: "missing.json".into(),
            path: Some("other".into()),
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
            path: Some("from-openapi".into()),
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
            path: Some("old".into()),
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
            path: Some("from-bruno".into()),
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
            path: Some("other".into()),
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
            path: Some("other".into()),
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

/// The imported collection without a target folder, and its file.
fn import_unsaved(fixture: &Fixture) -> crate::view::CollectionView {
    use crate::command::{Command, CommandAck};
    fixture.store.add_file(
        "export.json",
        r#"{"info": {"name": "Shop", "schema": "https://schema.getpostman.com/json/collection/v2.1.0/collection.json"}, "item": []}"#,
    );
    let ack = fixture
        .app
        .execute(Command::ImportCollection {
            file: "export.json".into(),
            path: None,
        })
        .unwrap();
    let CommandAck::CollectionImported { collection, .. } = ack else {
        panic!("expected an import, got {ack:?}");
    };
    *collection
}

#[tokio::test]
async fn imports_stay_unsaved_until_saved_as() {
    use crate::command::{Command, CommandAck};
    use crate::testing::store::TEMPORARY;

    let fixture = Fixture::new();
    let collection = import_unsaved(&fixture);
    assert!(collection.temporary);
    assert!(
        collection.root.starts_with(TEMPORARY),
        "{}",
        collection.root
    );
    // Edits work as in any collection and keep it unsaved.
    let ack = fixture
        .app
        .execute(Command::CreateRequest {
            collection_id: collection.id.clone(),
            parent_id: collection.id.clone(),
            name: "Health".into(),
        })
        .unwrap();
    let CommandAck::CollectionChanged {
        collection: changed,
        ..
    } = ack
    else {
        panic!("expected a change");
    };
    assert!(changed.temporary);

    let ack = fixture
        .app
        .execute(Command::SaveCollectionAs {
            collection_id: collection.id.clone(),
            path: "saved/shop".into(),
        })
        .unwrap();
    let CommandAck::CollectionChanged {
        collection: saved, ..
    } = ack
    else {
        panic!("expected a change");
    };
    assert!(!saved.temporary);
    assert_eq!(
        saved.root,
        std::path::Path::new("saved/shop").display().to_string()
    );
    assert_eq!(saved.items.len(), 1);
    assert_eq!(
        fixture
            .store
            .collection(std::path::Path::new("saved/shop"))
            .name,
        "Shop"
    );

    // Saved is saved: no second "save as", and closing keeps the files.
    let error = fixture
        .app
        .execute(Command::SaveCollectionAs {
            collection_id: collection.id.clone(),
            path: "elsewhere".into(),
        })
        .unwrap_err();
    assert_eq!(error.user_text().key, keys::ALREADY_SAVED);
    fixture
        .app
        .execute(Command::CloseCollection {
            collection_id: collection.id.clone(),
        })
        .unwrap();
    assert_eq!(
        fixture
            .store
            .collection(std::path::Path::new("saved/shop"))
            .name,
        "Shop"
    );
}

#[tokio::test]
async fn unsaved_collections_vanish_when_closed_or_when_the_app_ends() {
    use crate::command::{Command, Query};
    use crate::ports::CollectionStore;

    let fixture = Fixture::new();
    let closed = import_unsaved(&fixture);
    fixture
        .app
        .execute(Command::CloseCollection {
            collection_id: closed.id.clone(),
        })
        .unwrap();
    assert!(
        fixture
            .store
            .open(std::path::Path::new(&closed.root))
            .is_err()
    );

    let left_open = import_unsaved(&fixture);
    assert_eq!(fixture.app.discard_unsaved(), []);
    assert!(
        fixture
            .store
            .open(std::path::Path::new(&left_open.root))
            .is_err()
    );
    // The collection itself is still listed; asking about it fails cleanly.
    assert!(
        fixture
            .app
            .query(Query::Collection {
                collection_id: left_open.id.clone()
            })
            .is_err()
    );

    // Saving to an occupied folder fails and keeps the collection unsaved.
    let blocked = import_unsaved(&fixture);
    let error = fixture
        .app
        .execute(Command::SaveCollectionAs {
            collection_id: blocked.id.clone(),
            path: fixture.root.display().to_string(),
        })
        .unwrap_err();
    assert_eq!(error.user_text().key, keys::COLLECTION_EXISTS);
    let unknown = fixture.app.execute(Command::SaveCollectionAs {
        collection_id: crate::view::collection::non_empty("x").unwrap().into(),
        path: "y".into(),
    });
    assert!(unknown.is_err());
}

#[tokio::test]
async fn descriptions_are_imported_from_a_url_with_their_relative_server() {
    use crate::command::CommandAck;
    use crate::ports::TransportError;
    use crate::testing::{FakeHttpSender, response};

    let petstore = r#"{"openapi": "3.0.4", "info": {"title": "Petstore"},
        "servers": [{"url": "/api/v3"}],
        "paths": {"/pet": {"post": {"summary": "Add a pet"}}}}"#;
    let fixture = Fixture::with_http(
        FakeHttpSender::new()
            .reply(Ok(response(200, "application/json", petstore)))
            .reply(Ok(response(404, "text/plain", "not here")))
            .reply(Err(TransportError::Timeout))
            .reply(Ok(response(200, "text/plain", "plain text"))),
    );
    let url = "https://petstore3.swagger.io/api/v3/openapi.json";
    let CommandAck::CollectionImported {
        collection,
        skipped,
    } = fixture.app.import_from_url(url).await.unwrap()
    else {
        panic!("expected an import");
    };
    assert!(collection.temporary);
    assert_eq!(collection.name, "Petstore");
    assert_eq!(
        collection.variables[0].value,
        "https://petstore3.swagger.io/api/v3"
    );
    assert_eq!(skipped.len(), 0);
    let sent = &fixture.http.sent()[0];
    assert_eq!(sent.method.as_str(), "GET");
    assert_eq!(sent.url.as_str(), url);
    assert!(sent.verify_tls);
    assert!(sent.headers.iter().any(|header| header.name == "accept"));

    let status = fixture.app.import_from_url(url).await.unwrap_err();
    assert_eq!(status.user_text().key, keys::IMPORT_DOWNLOAD);
    assert_eq!(status.user_text().args["detail"], "status 404");
    let timeout = fixture.app.import_from_url(url).await.unwrap_err();
    assert_eq!(timeout.user_text().key, keys::IMPORT_DOWNLOAD);
    let unknown = fixture.app.import_from_url(url).await.unwrap_err();
    assert_eq!(unknown.user_text().key, keys::IMPORT_INVALID);
    let empty = fixture.app.import_from_url("  ").await.unwrap_err();
    assert_eq!(empty.user_text().key, keys::URL_EMPTY);
    assert_eq!(fixture.http.sent().len(), 4);
}

#[tokio::test]
async fn snippets_are_what_would_be_sent_in_every_language() {
    use crate::view::SnippetLanguageView;
    let fixture = Fixture::new();
    let mut request = OutgoingRequest::get("{{baseUrl}}/health");
    request.context = Some(SendContext {
        collection_id: fixture.collection.clone(),
        request_id: Some(fixture.health.clone()),
        environment_id: Some(fixture.dev.clone()),
    });
    for language in [
        SnippetLanguageView::Curl,
        SnippetLanguageView::Fetch,
        SnippetLanguageView::Axios,
        SnippetLanguageView::Python,
        SnippetLanguageView::Go,
        SnippetLanguageView::Httpie,
    ] {
        let result = fixture
            .app
            .query(Query::Snippet {
                language,
                request: Box::new(request.clone()),
            })
            .unwrap();
        let QueryResult::Snippet { code } = result else {
            panic!("expected code");
        };
        assert!(
            code.contains("https://dev.example.com/health"),
            "{language:?}: {code}"
        );
    }
    // An unknown variable is reported, not exported half-filled.
    let error = fixture
        .app
        .query(Query::Snippet {
            language: SnippetLanguageView::Python,
            request: Box::new(OutgoingRequest::get("{{nope}}/x")),
        })
        .unwrap_err();
    assert_eq!(error.user_text().key, keys::VARIABLE_UNKNOWN);
}

#[tokio::test]
async fn collections_export_as_postman_collections() {
    use crate::command::{Command, CommandAck};
    use crate::ports::{CollectionStore, StoreError, StoreErrorKind};
    use crate::view::ExportFormatView;

    let fixture = Fixture::new();
    let export = |file: &str| {
        fixture.app.execute(Command::ExportCollection {
            collection_id: fixture.collection.clone(),
            format: ExportFormatView::Postman,
            file: file.into(),
        })
    };
    let CommandAck::CollectionExported { left_out } = export("shop.postman.json").unwrap() else {
        panic!("expected an export");
    };
    assert_eq!(left_out, []);
    let text = fixture
        .store
        .read_text(std::path::Path::new("shop.postman.json"))
        .unwrap();
    let imported = ac_interchange::import_collection(&text).unwrap();
    assert_eq!(imported.collection.name, fixture.tree().name);

    fixture.store.fail_next(StoreError {
        kind: StoreErrorKind::Io,
        path: "out.json".into(),
        detail: "disk full".into(),
    });
    assert_eq!(
        export("out.json").unwrap_err().user_text().key,
        keys::STORE_IO
    );
    let error = fixture
        .app
        .execute(Command::ExportCollection {
            collection_id: "nope".into(),
            format: ExportFormatView::Postman,
            file: "x.json".into(),
        })
        .unwrap_err();
    assert_eq!(error.user_text().key, keys::UNKNOWN_COLLECTION);

    let secret = skipped_text(&Skipped {
        item: "Shop".into(),
        what: SkippedKind::SecretValue("token".into()),
    });
    assert_eq!(secret.key, keys::EXPORT_SECRET_VALUE);
    assert_eq!(secret.args["name"], "token");
    for (setting, key) in [
        (ac_interchange::ItemSetting::Timeout, keys::EXPORT_TIMEOUT),
        (
            ac_interchange::ItemSetting::Redirects,
            keys::EXPORT_REDIRECTS,
        ),
        (
            ac_interchange::ItemSetting::CertificateCheck,
            keys::EXPORT_CERTIFICATE_CHECK,
        ),
    ] {
        let text = skipped_text(&Skipped {
            item: "Login".into(),
            what: SkippedKind::Setting(setting),
        });
        assert_eq!(text.key, key);
        assert_eq!(text.args["item"], "Login");
    }
}

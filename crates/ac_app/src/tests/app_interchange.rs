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

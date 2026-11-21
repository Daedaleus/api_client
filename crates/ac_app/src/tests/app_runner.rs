use ac_domain::collection::ItemId;

use super::*;
use crate::ports::TransportError;
use crate::test_support::Fixture;
use crate::testing::{FakeHttpSender, response};
use crate::text::keys;

/// Gives the fixture's Health request a URL.
fn with_health_url(fixture: &Fixture) {
    let mut health = fixture.draft(&fixture.health);
    health.url = "https://health.example.com".into();
    fixture.save(health).unwrap();
}

fn plan(environment: Option<&str>, only: Option<&str>) -> RunPlan {
    RunPlan {
        environment: environment.map(Into::into),
        only: only.map(Into::into),
    }
}

#[tokio::test]
async fn runs_every_request_in_tree_order_with_the_environment() {
    let fixture = Fixture::with_http(
        FakeHttpSender::new()
            .reply(Ok(response(201, "text/plain", "")))
            .reply(Err(TransportError::Timeout)),
    );
    with_health_url(&fixture);
    let report = fixture
        .app
        .run(&fixture.collection, &plan(Some("Dev"), None))
        .await
        .unwrap();
    let paths: Vec<_> = report.results.iter().map(|r| r.path.join("/")).collect();
    assert_eq!(paths, ["Auth/Login", "Health"]);
    assert_eq!(report.results[0].method, "POST");
    assert!(matches!(
        report.results[0].outcome,
        RunResultOutcome::Response { status: 201, .. }
    ));
    assert!(matches!(
        &report.results[1].outcome,
        RunResultOutcome::Failed { error } if error.key == keys::TIMEOUT
    ));
    assert_eq!(
        fixture.http.sent()[0].url.as_str(),
        "https://dev.example.com/login"
    );
    assert_eq!(report.failed(true), 1);
}

#[tokio::test]
async fn requests_that_cannot_be_sent_fail_and_the_run_goes_on() {
    let fixture =
        Fixture::with_http(FakeHttpSender::new().reply(Ok(response(500, "text/plain", ""))));
    with_health_url(&fixture);
    // Without the environment, Login's {{baseUrl}} is unknown.
    let report = fixture
        .app
        .run(&fixture.collection, &plan(None, None))
        .await
        .unwrap();
    assert!(matches!(
        &report.results[0].outcome,
        RunResultOutcome::Failed { error } if error.key == keys::VARIABLE_UNKNOWN
    ));
    assert!(matches!(
        report.results[1].outcome,
        RunResultOutcome::Response { status: 500, .. }
    ));
    assert_eq!(report.failed(true), 2);
    assert_eq!(report.failed(false), 1);
    assert!(report.results[1].passed(false));
}

#[tokio::test]
async fn only_runs_a_folder_or_request() {
    let fixture =
        Fixture::with_http(FakeHttpSender::new().reply(Ok(response(200, "text/plain", ""))));
    let report = fixture
        .app
        .run(&fixture.collection, &plan(Some("Dev"), Some("/Auth/")))
        .await
        .unwrap();
    assert_eq!(report.results.len(), 1);
    assert_eq!(report.results[0].path, ["Auth", "Login"]);

    let error = fixture
        .app
        .run(&fixture.collection, &plan(None, Some("Nope")))
        .await
        .unwrap_err();
    assert_eq!(error.user_text().key, keys::NOTHING_TO_RUN);
    let error = fixture
        .app
        .run(&fixture.collection, &plan(Some("Prod"), None))
        .await
        .unwrap_err();
    assert_eq!(error.user_text().key, keys::UNKNOWN_ENVIRONMENT);
    assert_eq!(error.user_text().args["name"], "Prod");
    let error = fixture
        .app
        .run(&ItemId::new().to_string(), &plan(None, None))
        .await
        .unwrap_err();
    assert_eq!(error, AppError::UnknownCollection);
}

#[test]
fn reports_serialize_for_machines() {
    let report = RunReport {
        results: vec![RunResult {
            path: vec!["Health".into()],
            method: "GET".into(),
            outcome: RunResultOutcome::Response {
                status: 200,
                elapsed_ms: 12,
            },
        }],
    };
    assert_eq!(
        serde_json::to_value(&report).unwrap(),
        serde_json::json!({"results": [{"path": ["Health"], "method": "GET",
            "outcome": {"kind": "response", "status": 200, "elapsedMs": 12}}]})
    );
}

#[tokio::test]
async fn fetches_an_oauth_token_once_and_sends_it() {
    let token = r#"{"access_token":"abc","expires_in":3600}"#;
    let fixture = Fixture::with_http(
        FakeHttpSender::new()
            .reply(Ok(response(200, "application/json", token)))
            .reply(Ok(response(200, "text/plain", "")))
            .reply(Ok(response(200, "text/plain", ""))),
    );
    let mut health = fixture.draft(&fixture.health);
    health.url = "https://health.example.com".into();
    health.auth = crate::view::AuthDraft::OAuth2 {
        token_url: "{{baseUrl}}/token".into(),
        client_id: "app".into(),
        client_secret: "pw".into(),
        scope: String::new(),
    };
    fixture.save(health).unwrap();
    for _ in 0..2 {
        let report = fixture
            .app
            .run(&fixture.collection, &plan(Some("Dev"), Some("Health")))
            .await
            .unwrap();
        assert_eq!(report.failed(true), 0);
    }
    let sent = fixture.http.sent();
    let urls: Vec<_> = sent.iter().map(|request| request.url.as_str()).collect();
    assert_eq!(
        urls,
        [
            "https://dev.example.com/token",
            "https://health.example.com/",
            "https://health.example.com/"
        ]
    );
    for request in &sent[1..] {
        assert!(
            request
                .headers
                .iter()
                .any(|header| header.name == "authorization" && header.value == "Bearer abc")
        );
    }
}

#[tokio::test]
async fn a_request_without_an_oauth_token_fails() {
    let fixture = Fixture::with_http(FakeHttpSender::new().reply(Ok(response(
        401,
        "application/json",
        "{}",
    ))));
    let mut health = fixture.draft(&fixture.health);
    health.url = "https://health.example.com".into();
    health.auth = crate::view::AuthDraft::OAuth2 {
        token_url: "https://auth.example.com/token".into(),
        client_id: "app".into(),
        client_secret: "pw".into(),
        scope: String::new(),
    };
    fixture.save(health).unwrap();
    let report = fixture
        .app
        .run(&fixture.collection, &plan(None, Some("Health")))
        .await
        .unwrap();
    assert!(matches!(
        &report.results[0].outcome,
        RunResultOutcome::Failed { error } if error.key == keys::OAUTH_TOKEN
    ));
    assert_eq!(fixture.http.sent().len(), 1);
}

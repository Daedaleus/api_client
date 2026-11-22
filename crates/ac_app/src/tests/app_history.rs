use super::*;
use crate::command::Command;
use crate::test_support::Fixture;
use crate::testing::{FakeHttpSender, FakeScriptRunner, response};
use crate::text::keys;
use crate::view::{AuthDraft, HeaderDraft, SendContext};

/// The health request of the fixture with bearer auth and a JSON body.
fn health(fixture: &Fixture) -> OutgoingRequest {
    let mut request = OutgoingRequest::get("https://example.com/health?q=1");
    request.method = "POST".into();
    request.headers = vec![HeaderDraft {
        name: "Content-Type".into(),
        value: "application/json".into(),
        enabled: true,
    }];
    request.body = crate::view::BodyDraft::Json {
        content: "{\"a\":1}".into(),
    };
    request.auth = AuthDraft::Bearer {
        token: "t0ken".into(),
    };
    request.context = Some(SendContext {
        collection_id: fixture.collection.clone(),
        request_id: Some(fixture.health.clone()),
        environment_id: None,
    });
    request
}

#[tokio::test]
async fn sent_requests_are_recorded_listed_opened_and_cleared() {
    let fixture = Fixture::with_scripts(
        FakeHttpSender::new().reply(Ok(response(200, "application/json", "{\"ok\":true}"))),
        FakeScriptRunner::new(),
    );
    let mut events = fixture.app.subscribe();
    let execution = fixture.app.run_with_scripts(&health(&fixture)).await;
    assert!(execution.response.is_ok());
    assert!(matches!(
        events.recv().await.unwrap(),
        AppEvent::HistoryChanged { error: None }
    ));
    // Not sent at all (empty URL): not recorded.
    let _ = fixture
        .app
        .run_with_scripts(&OutgoingRequest::get(""))
        .await;

    let QueryResult::History { entries } =
        fixture.app.query(crate::command::Query::History).unwrap()
    else {
        panic!("expected the history");
    };
    assert_eq!(entries.len(), 1);
    let item = &entries[0];
    assert_eq!(
        (item.method.as_str(), item.url.as_str(), item.status),
        ("POST", "https://example.com/health?q=1", Some(200))
    );
    assert_eq!(item.collection.as_deref(), Some("Shop"));
    assert_eq!(item.at_ms, 1);

    let QueryResult::HistoryEntry { request, outcome } = fixture
        .app
        .query(crate::command::Query::HistoryEntry {
            id: item.id.clone(),
        })
        .unwrap()
    else {
        panic!("expected an entry");
    };
    assert_eq!(request.id, "");
    assert_eq!(request.method, "POST");
    let authorization = request
        .headers
        .iter()
        .find(|header| header.name.eq_ignore_ascii_case("authorization"))
        .unwrap();
    assert_eq!(authorization.value, crate::history::MASK);
    assert!(matches!(
        &request.body,
        crate::view::BodyDraft::Json { content } if content == "{\"a\":1}"
    ));
    let RunOutcomeView::Completed(view) = *outcome else {
        panic!("expected a response");
    };
    assert_eq!(view.status, 200);
    assert_eq!(view.size_bytes, 11);

    assert!(matches!(
        fixture
            .app
            .query(crate::command::Query::HistoryEntry { id: "nope".into() }),
        Err(AppError::UnknownItem)
    ));
    assert_eq!(
        fixture.run(Command::ClearHistory),
        CommandAck::HistoryCleared
    );
    let QueryResult::History { entries } =
        fixture.app.query(crate::command::Query::History).unwrap()
    else {
        panic!("expected the history");
    };
    assert_eq!(entries, []);
}

#[tokio::test]
async fn failures_are_recorded_and_write_errors_reported() {
    let fixture = Fixture::with_scripts(FakeHttpSender::new(), FakeScriptRunner::new());
    let _ = fixture.app.run_with_scripts(&health(&fixture)).await;
    let QueryResult::History { entries } =
        fixture.app.query(crate::command::Query::History).unwrap()
    else {
        panic!("expected the history");
    };
    assert_eq!(entries[0].status, None);
    assert_eq!(entries[0].error.as_ref().unwrap().key, keys::REQUEST);
    let QueryResult::HistoryEntry { outcome, .. } = fixture
        .app
        .query(crate::command::Query::HistoryEntry {
            id: entries[0].id.clone(),
        })
        .unwrap()
    else {
        panic!("expected an entry");
    };
    assert!(matches!(*outcome, RunOutcomeView::Failed { .. }));

    fixture.history.failing("disk full");
    let mut events = fixture.app.subscribe();
    let _ = fixture.app.run_with_scripts(&health(&fixture)).await;
    let AppEvent::HistoryChanged { error: Some(error) } = events.recv().await.unwrap() else {
        panic!("expected a history error");
    };
    assert_eq!(error.key, keys::HISTORY);
    assert_eq!(error.args["detail"], "disk full");
    let error = fixture.fail(Command::ClearHistory);
    assert_eq!(error.user_text().key, keys::HISTORY);
    assert!(fixture.app.query(crate::command::Query::History).is_err());
}

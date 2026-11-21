use ac_domain::script::ScriptPhase;

use super::*;
use crate::command::Command;
use crate::ports::{ScriptError, ScriptOutcome};
use crate::test_support::Fixture;
use crate::testing::{FakeHttpSender, FakeScriptRunner, response};
use crate::text::keys;
use crate::view::{RunOutcomeView, ScriptsDraft, SendContext};

/// A request in the fixture's collection with the Dev environment.
fn outgoing(fixture: &Fixture, url: &str, pre: &str, post: &str) -> OutgoingRequest {
    let mut request = OutgoingRequest::get(url);
    request.headers = vec![HeaderDraft {
        name: "X-Old".into(),
        value: "1".into(),
        enabled: true,
    }];
    request.body = BodyDraft::Json {
        content: "{}".into(),
    };
    request.scripts = ScriptsDraft {
        pre_request: pre.into(),
        post_response: post.into(),
    };
    request.context = Some(SendContext {
        collection_id: fixture.collection.clone(),
        request_id: None,
        environment_id: Some(fixture.dev.clone()),
    });
    request
}

fn pair(name: &str, value: &str) -> (String, String) {
    (name.into(), value.into())
}

#[tokio::test]
async fn the_pre_request_script_changes_the_request_before_variables_are_filled_in() {
    let changed = ScriptRequest {
        method: "POST".into(),
        url: "{{baseUrl}}/items?id={{id}}".into(),
        headers: vec![pair("X-New", "{{id}}")],
        body: Some("{\"id\": \"{{id}}\"}".into()),
    };
    let fixture = Fixture::with_scripts(
        FakeHttpSender::new().reply(Ok(response(200, "text/plain", "ok"))),
        FakeScriptRunner::new().result(Ok(ScriptOutcome {
            request: changed,
            variables: vec![pair("id", "42")],
            logs: vec!["pre ran".into()],
        })),
    );
    let request = outgoing(&fixture, "{{baseUrl}}/old", "req.method = 'POST'", "");
    let execution = fixture.app.run_with_scripts(&request, None).await;

    let job = &fixture.scripts.jobs()[0];
    assert_eq!(job.phase, ScriptPhase::PreRequest);
    assert_eq!(job.source, "req.method = 'POST'");
    assert_eq!(job.request.url, "{{baseUrl}}/old");
    assert_eq!(job.request.headers, [pair("X-Old", "1")]);
    assert_eq!(job.request.body.as_deref(), Some("{}"));
    assert!(
        job.variables
            .contains(&pair("baseUrl", "https://dev.example.com"))
    );

    let sent = &fixture.http.sent()[0];
    assert_eq!(sent.method.as_str(), "POST");
    assert_eq!(sent.url.as_str(), "https://dev.example.com/items?id=42");
    assert!(
        sent.headers
            .iter()
            .any(|h| h.name == "X-New" && h.value == "42")
    );
    assert!(!sent.headers.iter().any(|h| h.name == "X-Old"));
    assert_eq!(sent.body_bytes(), Some(&b"{\"id\": \"42\"}"[..]));

    let RunOutcomeView::Completed(view) = execution.into_view() else {
        panic!("expected a response");
    };
    assert_eq!(view.logs, ["pre ran"]);
    assert_eq!(view.script_error, None);
}

#[tokio::test]
async fn a_failing_pre_request_script_stops_the_run() {
    let fixture = Fixture::with_scripts(
        FakeHttpSender::new(),
        FakeScriptRunner::new().result(Err(ScriptError::Syntax {
            line: 2,
            message: "Unexpected token".into(),
        })),
    );
    let request = outgoing(&fixture, "{{baseUrl}}/x", "nope(", "");
    let RunOutcomeView::Failed { error, logs } = fixture
        .app
        .run_with_scripts(&request, None)
        .await
        .into_view()
    else {
        panic!("expected a failure");
    };
    assert_eq!(error.key, keys::SCRIPT_PRE_REQUEST);
    assert_eq!(
        error.args["detail"],
        "syntax error in line 2: Unexpected token"
    );
    assert_eq!(logs.len(), 0);
    assert_eq!(fixture.http.sent().len(), 0);

    // A request the script left invalid fails like any other.
    let fixture = Fixture::with_scripts(
        FakeHttpSender::new(),
        FakeScriptRunner::new().result(Ok(ScriptOutcome {
            request: ScriptRequest {
                method: "GET".into(),
                url: String::new(),
                ..ScriptRequest::default()
            },
            logs: vec!["cleared the URL".into()],
            ..ScriptOutcome::default()
        })),
    );
    let RunOutcomeView::Failed { error, logs } = fixture
        .app
        .run_with_scripts(&outgoing(&fixture, "x", "req.url = ''", ""), None)
        .await
        .into_view()
    else {
        panic!("expected a failure");
    };
    assert_eq!(error.key, keys::URL_EMPTY);
    assert_eq!(logs, ["cleared the URL"]);
}

#[tokio::test]
async fn the_post_response_script_reads_the_response_and_sets_variables_for_later() {
    let fixture = Fixture::with_scripts(
        FakeHttpSender::new()
            .reply(Ok(response(201, "application/json", "{\"token\":\"t\"}")))
            .reply(Ok(response(200, "text/plain", ""))),
        FakeScriptRunner::new().result(Ok(ScriptOutcome {
            variables: vec![pair("token", "t")],
            logs: vec!["got token".into()],
            ..ScriptOutcome::default()
        })),
    );
    let request = outgoing(
        &fixture,
        "{{baseUrl}}/login",
        "",
        "env.set('token', res.json().token)",
    );
    let RunOutcomeView::Completed(view) = fixture
        .app
        .run_with_scripts(&request, None)
        .await
        .into_view()
    else {
        panic!("expected a response");
    };
    assert_eq!(view.status, 201);
    assert_eq!(view.logs, ["got token"]);

    let job = &fixture.scripts.jobs()[0];
    assert_eq!(job.phase, ScriptPhase::PostResponse);
    assert_eq!(job.request.url, "https://dev.example.com/login");
    let received = job.response.as_ref().unwrap();
    assert_eq!(received.status, 201);
    assert_eq!(received.body, "{\"token\":\"t\"}");

    // The next request of the collection sees the variable, strongest of all.
    let later = outgoing(&fixture, "{{baseUrl}}/me?t={{token}}", "", "");
    fixture.app.run_with_scripts(&later, None).await;
    assert_eq!(
        fixture.http.sent()[1].url.as_str(),
        "https://dev.example.com/me?t=t"
    );

    // Closing the collection forgets it.
    fixture.run(Command::CloseCollection {
        collection_id: fixture.collection.clone(),
    });
    assert_eq!(fixture.app.runtime_variables(&later).len(), 0);
}

#[tokio::test]
async fn a_failing_post_response_script_keeps_the_response() {
    let fixture = Fixture::with_scripts(
        FakeHttpSender::new().reply(Ok(response(200, "text/plain", "ok"))),
        FakeScriptRunner::new().result(Err(ScriptError::TimedOut { seconds: 5 })),
    );
    let request = outgoing(&fixture, "{{baseUrl}}/x", "", "while (true) {}");
    let RunOutcomeView::Completed(view) = fixture
        .app
        .run_with_scripts(&request, None)
        .await
        .into_view()
    else {
        panic!("expected a response");
    };
    let error = view.script_error.unwrap();
    assert_eq!(error.key, keys::SCRIPT_POST_RESPONSE);
    assert_eq!(error.args["detail"], "stopped after 5 s");
}

#[tokio::test]
async fn scripts_also_run_from_send_and_errors_explain_themselves() {
    let fixture = Fixture::with_scripts(
        FakeHttpSender::new().reply(Ok(response(200, "text/plain", "ok"))),
        FakeScriptRunner::new(),
    );
    let mut events = fixture.app.subscribe();
    let request = outgoing(&fixture, "{{nope}}", "log('x')", "");
    // With a pre-request script the request is checked only after it.
    fixture.run(Command::SendRequest { request });
    let crate::event::AppEvent::RunFinished { outcome, .. } = events.recv().await.unwrap();
    let RunOutcomeView::Failed { error, .. } = outcome else {
        panic!("expected a failure");
    };
    assert_eq!(error.key, keys::VARIABLE_UNKNOWN);
    assert_eq!(
        ScriptError::OutOfMemory { megabytes: 64 }.to_string(),
        "used more than 64 MB of memory"
    );
    assert_eq!(ScriptError::Failed("boom".into()).to_string(), "boom");
}

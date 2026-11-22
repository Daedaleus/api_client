use std::sync::Arc;
use std::time::Duration;

use tokio::runtime::Handle;
use tokio::sync::broadcast;
use tokio::time::timeout;

use super::*;
use crate::app::Ports;
use crate::command::Command;
use crate::ports::TransportError;
use crate::testing::{
    FakeBrowserLogin, FakeHttpSender, FakeScriptRunner, FakeVault, MemoryStore, response,
};
use crate::text::{UserText, keys};
use crate::view::BodyDraft;
use crate::view::RunOutcomeView;

type TestApp = App<
    Arc<FakeHttpSender>,
    MemoryStore,
    FakeVault,
    FakeBrowserLogin,
    FakeScriptRunner,
    crate::ports::NoAssist,
    crate::ports::NoHistory,
>;

fn app(fake: FakeHttpSender) -> (TestApp, Arc<FakeHttpSender>) {
    let fake = Arc::new(fake);
    let app = App::new(
        Ports {
            http: Arc::clone(&fake),
            store: MemoryStore::new(),
            vault: FakeVault::default(),
            login: FakeBrowserLogin::new(),
            scripts: FakeScriptRunner::new(),
            assist: crate::ports::NoAssist,
            history: crate::ports::NoHistory,
        },
        Handle::current(),
    );
    (app, fake)
}

fn run_id(ack: &CommandAck) -> RunId {
    let CommandAck::RunStarted { run_id } = ack else {
        panic!("expected a started run, got {ack:?}");
    };
    *run_id
}

fn send(method: &str, url: &str) -> Command {
    Command::SendRequest {
        request: crate::view::OutgoingRequest {
            path_params: Vec::new(),
            certificate_check: crate::view::CertificateCheckView::Inherit,
            scripts: crate::view::ScriptsDraft::default(),
            method: method.into(),
            url: url.into(),
            headers: Vec::new(),
            body: BodyDraft::None,
            variables: Vec::new(),
            auth: crate::view::AuthDraft::Inherit,
            context: None,
        },
    }
}

#[tokio::test]
async fn headers_and_body_are_sent() {
    let (app, fake) = app(FakeHttpSender::new().reply(Ok(response(201, "text/plain", ""))));
    app.execute(Command::SendRequest {
        request: crate::view::OutgoingRequest {
            path_params: Vec::new(),
            certificate_check: crate::view::CertificateCheckView::Inherit,
            scripts: crate::view::ScriptsDraft::default(),
            method: "POST".into(),
            url: "example.com/items".into(),
            headers: vec![HeaderDraft {
                name: "X-Test".into(),
                value: "yes".into(),
                enabled: true,
            }],
            body: BodyDraft::Json {
                content: "{\"a\":1}".into(),
            },
            variables: Vec::new(),
            auth: crate::view::AuthDraft::Inherit,
            context: None,
        },
    })
    .unwrap();
    tokio::task::yield_now().await;

    let sent = fake.sent();
    assert_eq!(sent[0].body_bytes(), Some(b"{\"a\":1}".as_slice()));
    assert_eq!(sent[0].headers.len(), 2);
    assert_eq!(sent[0].headers[0].name, "X-Test");
}

/// The next finished run (history notices are skipped).
async fn next_event(events: &mut broadcast::Receiver<AppEvent>) -> AppEvent {
    loop {
        let event = timeout(Duration::from_secs(5), events.recv())
            .await
            .expect("event within 5 s")
            .expect("channel open");
        if matches!(event, AppEvent::RunFinished { .. }) {
            return event;
        }
    }
}

#[tokio::test]
async fn sending_reports_the_response_as_event() {
    let (app, fake) = app(FakeHttpSender::new().reply(Ok(response(200, "text/plain", "hi"))));
    let mut events = app.subscribe();

    let ack = app.execute(send("GET", "example.com/a")).unwrap();
    let run_id = run_id(&ack);

    let AppEvent::RunFinished {
        run_id: finished,
        outcome,
    } = next_event(&mut events).await
    else {
        unreachable!("next_event returns finished runs");
    };
    assert_eq!(finished, run_id);
    let RunOutcomeView::Completed(view) = outcome else {
        panic!("expected a response, got {outcome:?}");
    };
    assert_eq!(view.status, 200);
    assert_eq!(view.body.text, "hi");

    let sent = fake.sent();
    assert_eq!(sent.len(), 1);
    assert_eq!(sent[0].url.as_str(), "http://example.com/a");
}

/// Regression: Tauri runs synchronous commands on the main thread, which has
/// no Tokio runtime. Sending from there must work, not panic.
#[tokio::test]
async fn sending_works_from_a_thread_without_runtime() {
    let (app, _) = app(FakeHttpSender::new().reply(Ok(response(204, "text/plain", ""))));
    let mut events = app.subscribe();

    let caller = app.clone();
    let ack = std::thread::spawn(move || caller.execute(send("GET", "example.com")))
        .join()
        .expect("execute must not panic outside a runtime")
        .unwrap();
    let run_id = run_id(&ack);

    let AppEvent::RunFinished {
        run_id: finished, ..
    } = next_event(&mut events).await
    else {
        unreachable!("next_event returns finished runs");
    };
    assert_eq!(finished, run_id);
}

#[tokio::test]
async fn transport_errors_are_a_normal_outcome() {
    let (app, _) = app(FakeHttpSender::new().reply(Err(TransportError::Timeout)));
    let mut events = app.clone().subscribe();
    app.execute(send("GET", "https://slow.example")).unwrap();

    let AppEvent::RunFinished { outcome, .. } = next_event(&mut events).await else {
        unreachable!("next_event returns finished runs");
    };
    assert_eq!(
        outcome,
        RunOutcomeView::Failed {
            error: UserText::new(keys::TIMEOUT),
            logs: Vec::new(),
            tests: Vec::new(),
        }
    );
}

#[tokio::test]
async fn invalid_commands_start_no_run() {
    let (app, fake) = app(FakeHttpSender::new());
    let error = app.execute(send("GET", "")).unwrap_err();
    assert_eq!(error, AppError::EmptyUrl);
    assert_eq!(fake.sent(), vec![]);
}

#[tokio::test]
async fn run_ids_increase() {
    let (app, _) = app(FakeHttpSender::new());
    let first = app.execute(send("GET", "a.example")).unwrap();
    let second = app.execute(send("GET", "b.example")).unwrap();
    assert_eq!(first, CommandAck::RunStarted { run_id: RunId(1) });
    assert_eq!(second, CommandAck::RunStarted { run_id: RunId(2) });
}

#[tokio::test]
async fn unanswered_fake_and_transport_texts() {
    let (app, _) = app(FakeHttpSender::new());
    let mut events = app.subscribe();
    app.execute(send("GET", "a.example")).unwrap();
    let AppEvent::RunFinished { outcome, .. } = next_event(&mut events).await else {
        unreachable!("next_event returns finished runs");
    };
    let RunOutcomeView::Failed { error, .. } = outcome else {
        panic!("expected a failure");
    };
    assert_eq!(error.key, keys::REQUEST);

    let connect = TransportError::Connect("refused".into()).user_text();
    assert_eq!(connect.key, keys::CONNECT);
    assert_eq!(connect.args["detail"], "refused");
}

#[test]
fn commands_and_events_match_the_ui_contract() {
    let command: Command = serde_json::from_value(
        serde_json::json!({ "type": "sendRequest", "method": "GET", "url": "x" }),
    )
    .unwrap();
    assert_eq!(command, send("GET", "x"));

    let ack = CommandAck::RunStarted { run_id: RunId(3) };
    assert_eq!(
        serde_json::to_value(ack).unwrap(),
        serde_json::json!({ "type": "runStarted", "runId": 3 })
    );

    let event = AppEvent::RunFinished {
        run_id: RunId(3),
        outcome: RunOutcomeView::Failed {
            error: UserText::new(keys::TIMEOUT),
            logs: Vec::new(),
            tests: Vec::new(),
        },
    };
    assert_eq!(
        serde_json::to_value(event).unwrap()["type"],
        serde_json::json!("runFinished")
    );
}

use std::time::Duration;

use tokio::time::timeout;

use super::*;
use crate::ports::TransportError;
use crate::testing::{FakeHttpSender, response};
use crate::text::{UserText, keys};

fn app(fake: FakeHttpSender) -> (App<Arc<FakeHttpSender>>, Arc<FakeHttpSender>) {
    let fake = Arc::new(fake);
    let app = App::new(Ports {
        http: Arc::clone(&fake),
    });
    (app, fake)
}

fn send(method: &str, url: &str) -> Command {
    Command::SendRequest {
        method: method.into(),
        url: url.into(),
    }
}

async fn next_event(events: &mut broadcast::Receiver<AppEvent>) -> AppEvent {
    timeout(Duration::from_secs(5), events.recv())
        .await
        .expect("event within 5 s")
        .expect("channel open")
}

#[tokio::test]
async fn sending_reports_the_response_as_event() {
    let (app, fake) = app(FakeHttpSender::new().reply(Ok(response(200, "text/plain", "hi"))));
    let mut events = app.subscribe();

    let ack = app.execute(send("GET", "example.com/a")).unwrap();
    let CommandAck::RunStarted { run_id } = ack;

    let AppEvent::RunFinished {
        run_id: finished,
        outcome,
    } = next_event(&mut events).await;
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

#[tokio::test]
async fn transport_errors_are_a_normal_outcome() {
    let (app, _) = app(FakeHttpSender::new().reply(Err(TransportError::Timeout)));
    let mut events = app.clone().subscribe();
    app.execute(send("GET", "https://slow.example")).unwrap();

    let AppEvent::RunFinished { outcome, .. } = next_event(&mut events).await;
    assert_eq!(
        outcome,
        RunOutcomeView::Failed {
            error: UserText::new(keys::TIMEOUT)
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
    let AppEvent::RunFinished { outcome, .. } = next_event(&mut events).await;
    let RunOutcomeView::Failed { error } = outcome else {
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
        },
    };
    assert_eq!(
        serde_json::to_value(event).unwrap()["type"],
        serde_json::json!("runFinished")
    );
}

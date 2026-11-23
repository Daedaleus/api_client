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
    crate::ports::DefaultSettings,
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
            settings: crate::ports::DefaultSettings,
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
            transport: crate::view::TransportDraft::default(),
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
            transport: crate::view::TransportDraft::default(),
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

#[tokio::test]
async fn requests_of_a_collection_share_their_cookies() {
    use ac_domain::exchange::Header;

    use crate::test_support::Fixture;
    use crate::view::{CookiesView, SendContext, TransportDraft};

    let mut login = response(200, "text/plain", "");
    login
        .headers
        .push(Header::new("Set-Cookie", "session=s3cret; Path=/"));
    let mut fake = FakeHttpSender::new().reply(Ok(login));
    for _ in 0..4 {
        fake = fake.reply(Ok(response(200, "text/plain", "")));
    }
    let fixture = Fixture::with_http(fake);
    let send = |in_collection: bool, cookies: CookiesView| {
        let mut events = fixture.app.subscribe();
        let Command::SendRequest { mut request } = send("GET", "https://x.example/login") else {
            unreachable!()
        };
        request.transport = TransportDraft {
            cookies,
            ..TransportDraft::default()
        };
        request.context = in_collection.then(|| SendContext {
            collection_id: fixture.collection.clone(),
            request_id: None,
            environment_id: None,
        });
        fixture
            .app
            .execute(Command::SendRequest { request })
            .unwrap();
        async move { next_event(&mut events).await }
    };
    send(true, CookiesView::Inherit).await;
    send(true, CookiesView::On).await;
    // Switched off, outside the collection: none.
    send(true, CookiesView::Off).await;
    send(false, CookiesView::Inherit).await;
    assert_eq!(
        fixture.http.cookies_sent(),
        [None, Some("session=s3cret".into()), None, None]
    );

    // Closing the collection discards its cookies.
    fixture
        .app
        .execute(Command::CloseCollection {
            collection_id: fixture.collection.clone(),
        })
        .unwrap();
    let jar = fixture.app.cookie_jar(Some(fixture.collection.clone()));
    assert_eq!(
        jar.lock()
            .unwrap()
            .cookies(crate::testing::FAKE_NOW)
            .count(),
        0
    );
}

#[tokio::test]
async fn digest_answers_the_challenge_and_aws_signs() {
    use ac_domain::exchange::Header;

    use crate::view::AuthDraft;

    let mut challenge = response(401, "text/plain", "");
    challenge.headers.push(Header::new(
        "WWW-Authenticate",
        r#"Digest realm="api", nonce="n0nce", qop="auth", algorithm=SHA-256"#,
    ));
    let mut plain_401 = response(401, "text/plain", "");
    plain_401
        .headers
        .push(Header::new("WWW-Authenticate", r#"Basic realm="api""#));
    let (app, fake) = app(FakeHttpSender::new()
        .reply(Ok(challenge))
        .reply(Ok(response(200, "text/plain", "")))
        .reply(Ok(response(200, "text/plain", "")))
        .reply(Ok(plain_401))
        .reply(Ok(response(200, "text/plain", ""))));
    let send = |auth: AuthDraft| {
        let mut events = app.subscribe();
        let Command::SendRequest { mut request } = send("GET", "https://x.example/a?b=1") else {
            unreachable!()
        };
        request.auth = auth;
        app.execute(Command::SendRequest { request }).unwrap();
        async move { next_event(&mut events).await }
    };
    let digest = AuthDraft::Digest {
        username: "bob".into(),
        password: "pw".into(),
    };
    // Challenge, then the answer.
    send(digest.clone()).await;
    // No challenge: one request; a 401 without Digest challenge stays.
    send(digest.clone()).await;
    send(digest).await;
    let requests = fake.sent();
    let authorization = |index: usize| {
        requests[index]
            .headers
            .iter()
            .find(|h| h.name.eq_ignore_ascii_case("authorization"))
            .map(|h| h.value.clone())
    };
    assert_eq!(requests.len(), 4);
    assert_eq!(authorization(0), None);
    let answer = authorization(1).unwrap();
    assert!(
        answer
            .starts_with(r#"Digest username="bob", realm="api", uri="/a?b=1", algorithm=SHA-256"#),
        "{answer}"
    );
    assert!(
        answer.contains("qop=auth") && answer.contains("nc=00000001"),
        "{answer}"
    );
    assert_eq!(authorization(2), None);
    assert_eq!(authorization(3), None);

    send(AuthDraft::AwsSigV4 {
        access_key_id: " AKID ".into(),
        secret_access_key: "secret".into(),
        session_token: String::new(),
        region: "eu-central-1".into(),
        service: "execute-api".into(),
    })
    .await;
    let aws = fake.sent().pop().unwrap();
    let signature = aws
        .headers
        .iter()
        .find(|h| h.name.eq_ignore_ascii_case("authorization"))
        .unwrap();
    assert!(
        signature
            .value
            .starts_with("AWS4-HMAC-SHA256 Credential=AKID/"),
        "{signature:?}"
    );
    assert!(
        signature
            .value
            .contains("/eu-central-1/execute-api/aws4_request")
    );
    assert!(aws.headers.iter().any(|h| h.name == "x-amz-date"));
}

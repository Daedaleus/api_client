use std::time::Duration;

use super::*;
use crate::command::Command;
use crate::ports::{StoreError, StoreErrorKind};
use crate::test_support::Fixture;
use crate::testing::{FakeHttpSender, response};
use crate::text::keys;
use crate::view::OutgoingRequest;

/// Sends one request and returns its run.
async fn run(fixture: &Fixture) -> RunId {
    let mut events = fixture.app.subscribe();
    let CommandAck::RunStarted { run_id } = fixture
        .app
        .execute(Command::SendRequest {
            request: OutgoingRequest::get("https://x.example/file"),
        })
        .unwrap()
    else {
        panic!("expected a run");
    };
    tokio::time::timeout(Duration::from_secs(5), events.recv())
        .await
        .expect("run finishes")
        .expect("channel open");
    run_id
}

#[tokio::test]
async fn bodies_are_saved_byte_for_byte_while_kept() {
    let mut binary = response(200, "application/octet-stream", "");
    binary.body = vec![0, 159, 146, 150, 255];
    let mut fake = FakeHttpSender::new().reply(Ok(binary));
    for _ in 0..KEPT_RESPONSES {
        fake = fake.reply(Ok(response(200, "text/plain", "later")));
    }
    let fixture = Fixture::with_http(fake);
    let first = run(&fixture).await;
    let save = |run_id: RunId, file: &str| {
        fixture.app.execute(Command::SaveResponseBody {
            run_id,
            file: file.into(),
        })
    };
    assert_eq!(save(first, "out.bin").unwrap(), CommandAck::ResponseSaved);
    assert_eq!(
        fixture.store.written("out.bin"),
        Some(vec![0, 159, 146, 150, 255])
    );

    fixture.store.fail_next(StoreError {
        kind: StoreErrorKind::Io,
        path: "out.bin".into(),
        detail: "disk full".into(),
    });
    assert_eq!(
        save(first, "out.bin").unwrap_err().user_text().key,
        keys::STORE_IO
    );

    // Only the last runs are kept.
    let mut last = first;
    for _ in 0..KEPT_RESPONSES {
        last = run(&fixture).await;
    }
    assert_eq!(
        save(first, "old.bin").unwrap_err().user_text().key,
        keys::RESPONSE_GONE
    );
    save(last, "last.txt").unwrap();
    assert_eq!(fixture.store.written("last.txt"), Some(b"later".to_vec()));
    assert_eq!(
        save(RunId(999), "x").unwrap_err().user_text().key,
        keys::RESPONSE_GONE
    );
}

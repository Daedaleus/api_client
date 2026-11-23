use std::time::Duration;

use super::*;
use crate::command::{Command, Query};
use crate::test_support::Fixture;
use crate::testing::{FakeHttpSender, response};
use crate::text::keys;
use crate::view::{OutgoingRequest, SendContext};

/// Sends the fixture's login request and returns the run.
async fn run(fixture: &Fixture) -> RunId {
    let mut events = fixture.app.subscribe();
    let mut request = OutgoingRequest::get("https://x.example/login");
    request.context = Some(SendContext {
        collection_id: fixture.collection.clone(),
        request_id: Some(fixture.login.clone()),
        environment_id: None,
    });
    let CommandAck::RunStarted { run_id } = fixture
        .app
        .execute(Command::SendRequest { request })
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

fn examples(ack: CommandAck) -> Vec<ExampleView> {
    match ack {
        CommandAck::ExamplesChanged { examples } => examples,
        other => panic!("expected examples, got {other:?}"),
    }
}

#[tokio::test]
async fn responses_are_kept_listed_and_removed_as_examples() {
    let mut binary = response(200, "image/png", "");
    binary.body = vec![0xff, 0xd8, 0x00];
    let fixture = Fixture::with_http(
        FakeHttpSender::new()
            .reply(Ok(response(201, "application/json", "{\"token\":\"t\"}")))
            .reply(Ok(binary)),
    );
    let created = run(&fixture).await;
    let save = |run_id: RunId, name: &str| {
        fixture.app.execute(Command::SaveExample {
            collection_id: fixture.collection.clone(),
            request_id: fixture.login.clone(),
            run_id,
            name: name.into(),
        })
    };
    let list = examples(save(created, " Created ").unwrap());
    assert_eq!(list.len(), 1);
    assert_eq!(list[0].name, "Created");
    assert_eq!(list[0].response.status, 201);
    assert_eq!(list[0].response.body.text, "{\n  \"token\": \"t\"\n}");

    // Saving the request itself keeps them.
    let draft = fixture.draft(&fixture.login);
    fixture.save(draft).unwrap();
    let QueryResult::Examples { examples: listed } = fixture
        .app
        .query(Query::Examples {
            collection_id: fixture.collection.clone(),
            request_id: fixture.login.clone(),
        })
        .unwrap()
    else {
        panic!("expected examples");
    };
    assert_eq!(listed, list);

    let binary = run(&fixture).await;
    let key =
        |result: Result<CommandAck, crate::error::AppError>| result.unwrap_err().user_text().key;
    assert_eq!(key(save(binary, "Image")), keys::EXAMPLE_NOT_TEXT);
    assert_eq!(key(save(created, "  ")), keys::NAME_EMPTY);
    assert_eq!(key(save(RunId(999), "Gone")), keys::RESPONSE_GONE);

    let delete = |index: usize| {
        fixture.app.execute(Command::DeleteExample {
            collection_id: fixture.collection.clone(),
            request_id: fixture.login.clone(),
            index,
        })
    };
    assert_eq!(key(delete(3)), keys::UNKNOWN_ITEM);
    assert_eq!(examples(delete(0).unwrap()), []);
    let error = fixture
        .app
        .query(Query::Examples {
            collection_id: fixture.collection.clone(),
            request_id: fixture.auth.clone(),
        })
        .unwrap_err();
    assert_eq!(error.user_text().key, keys::UNKNOWN_ITEM);
}

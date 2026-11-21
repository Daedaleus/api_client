use std::time::Duration;

use super::*;
use crate::command::Command;
use crate::event::AppEvent;
use crate::ports::{StoreError, StoreErrorKind};
use crate::test_support::{Fixture, changed};
use crate::testing::{FakeHttpSender, response};
use crate::text::keys;
use crate::view::{BodyDraft, EnvironmentDraft, VariableDraft};

fn send(fixture: &Fixture, url: &str, environment: Option<&str>) -> Result<CommandAck, AppError> {
    fixture.app.execute(Command::SendRequest {
        method: "GET".into(),
        url: url.into(),
        headers: Vec::new(),
        body: BodyDraft::None,
        variables: Vec::new(),
        auth: crate::view::AuthDraft::Inherit,
        context: Some(SendContext {
            collection_id: fixture.collection.clone(),
            request_id: None,
            environment_id: environment.map(Into::into),
        }),
    })
}

fn dev(fixture: &Fixture) -> EnvironmentDraft {
    fixture.tree().environments[0].clone()
}

#[tokio::test]
async fn the_tree_lists_the_environments() {
    let fixture = Fixture::new();
    let environments = fixture.tree().environments;
    assert_eq!(environments.len(), 1);
    assert_eq!(environments[0].name, "Dev");
    assert_eq!(
        environments[0].variables,
        [VariableDraft {
            name: "baseUrl".into(),
            value: "https://dev.example.com".into(),
            enabled: true,
            secret: false,
        }]
    );
    assert!(!environments[0].read_only);
}

#[tokio::test]
async fn sending_resolves_variables_from_the_chosen_environment() {
    let fixture =
        Fixture::with_http(FakeHttpSender::new().reply(Ok(response(200, "text/plain", ""))));
    let mut events = fixture.app.subscribe();
    send(&fixture, "{{baseUrl}}/health", Some(&fixture.dev)).unwrap();
    // The run finishes once the request went out.
    let event = tokio::time::timeout(Duration::from_secs(5), events.recv())
        .await
        .expect("run finishes within 5 s")
        .expect("channel open");
    assert!(matches!(event, AppEvent::RunFinished { .. }));
    assert_eq!(
        fixture.http.sent()[0].url.as_str(),
        "https://dev.example.com/health"
    );
}

#[tokio::test]
async fn placeholders_without_environment_or_value_are_refused() {
    let fixture = Fixture::new();
    let error = send(&fixture, "{{baseUrl}}/health", None).unwrap_err();
    assert_eq!(error.user_text().key, keys::VARIABLE_UNKNOWN);
    let error = send(&fixture, "{{other}}/health", Some(&fixture.dev)).unwrap_err();
    assert_eq!(error.user_text().args["name"], "other");
    let error = send(&fixture, "x.example", Some(&fixture.health)).unwrap_err();
    assert_eq!(error, AppError::UnknownItem);
    assert_eq!(fixture.http.sent().len(), 0);
}

#[tokio::test]
async fn environments_can_be_created_saved_and_deleted() {
    let fixture = Fixture::new();
    let (tree, created) = changed(fixture.run(Command::CreateEnvironment {
        collection_id: fixture.collection.clone(),
        name: " Staging ".into(),
    }));
    let staging = created.unwrap();
    assert_eq!(tree.environments.len(), 2);
    assert!(
        tree.environments
            .iter()
            .any(|e| e.id == staging && e.name == "Staging")
    );

    let mut draft = dev(&fixture);
    draft.name = "Development".into();
    draft.variables.push(VariableDraft {
        name: "token".into(),
        value: "abc".into(),
        enabled: false,
        secret: false,
    });
    fixture.run(Command::SaveEnvironment {
        collection_id: fixture.collection.clone(),
        environment: draft.clone(),
    });
    let saved = &fixture.store.environments(&fixture.root)[0];
    assert_eq!(saved.name, "Development");
    assert_eq!(saved.variables.len(), 2);

    let (tree, _) = changed(fixture.run(Command::DeleteItem {
        collection_id: fixture.collection.clone(),
        item_id: staging,
    }));
    assert_eq!(tree.environments.len(), 1);
}

#[tokio::test]
async fn invalid_and_read_only_environments_are_refused() {
    let fixture = Fixture::new();
    let save = |environment: EnvironmentDraft| {
        fixture.app.execute(Command::SaveEnvironment {
            collection_id: fixture.collection.clone(),
            environment,
        })
    };
    let unnamed = EnvironmentDraft {
        name: " ".into(),
        ..dev(&fixture)
    };
    assert_eq!(save(unnamed).unwrap_err(), AppError::EmptyName);
    let unknown = EnvironmentDraft {
        id: fixture.health.clone(),
        ..dev(&fixture)
    };
    assert_eq!(save(unknown).unwrap_err(), AppError::UnknownItem);
    assert_eq!(
        fixture.fail(Command::CreateEnvironment {
            collection_id: fixture.collection.clone(),
            name: String::new(),
        }),
        AppError::EmptyName
    );

    fixture.store.mark_read_only(fixture.dev.parse().unwrap());
    let error = save(dev(&fixture)).unwrap_err();
    assert_eq!(error.user_text().key, keys::STORE_READ_ONLY);
    assert!(dev(&fixture).read_only);

    fixture.store.fail_next(StoreError {
        kind: StoreErrorKind::Io,
        path: "shop".into(),
        detail: "disk full".into(),
    });
    let error = fixture.fail(Command::CreateEnvironment {
        collection_id: fixture.collection.clone(),
        name: "x".into(),
    });
    assert_eq!(error.user_text().key, keys::STORE_IO);
}

use std::time::Duration;

use ac_domain::collection::ItemId;

use crate::command::{Command, CommandAck};
use crate::error::AppError;
use crate::ports::{SecretError, SecretValues, SecretVault};
use crate::test_support::Fixture;
use crate::testing::{FakeHttpSender, FakeVault, response};
use crate::text::keys;
use crate::view::{BodyDraft, EnvironmentDraft, SendContext, VariableDraft};

fn secret(name: &str, value: &str) -> VariableDraft {
    VariableDraft {
        name: name.into(),
        value: value.into(),
        enabled: true,
        secret: true,
    }
}

fn dev(fixture: &Fixture) -> EnvironmentDraft {
    fixture.tree().environments[0].clone()
}

fn save(fixture: &Fixture, environment: EnvironmentDraft) -> Result<CommandAck, AppError> {
    fixture.app.execute(Command::SaveEnvironment {
        collection_id: fixture.collection.clone(),
        environment,
    })
}

/// Saves Dev with the extra secret `token`.
fn with_token(fixture: &Fixture) {
    let mut draft = dev(fixture);
    draft.variables.retain(|variable| variable.name != "token");
    draft.variables.push(secret("token", "s3cr3t"));
    save(fixture, draft).unwrap();
}

fn dev_id(fixture: &Fixture) -> ItemId {
    fixture.dev.parse().unwrap()
}

/// The values stored for Dev, decrypted with the user's key.
fn stored(fixture: &Fixture) -> Option<SecretValues> {
    let data = fixture.store.secrets(&fixture.root, dev_id(fixture))?;
    Some(FakeVault::default().decrypt(&data).unwrap())
}

fn values(pairs: &[(&str, &str)]) -> SecretValues {
    pairs
        .iter()
        .map(|&(name, value)| (name.to_owned(), value.to_owned()))
        .collect()
}

fn send(fixture: &Fixture, url: &str) -> Result<CommandAck, AppError> {
    fixture.app.execute(Command::SendRequest {
        request: crate::view::OutgoingRequest {
            path_params: Vec::new(),
            certificate_check: crate::view::CertificateCheckView::Inherit,
            scripts: crate::view::ScriptsDraft::default(),
            method: "GET".into(),
            url: url.into(),
            headers: Vec::new(),
            body: BodyDraft::None,
            variables: Vec::new(),
            auth: crate::view::AuthDraft::Inherit,
            context: Some(SendContext {
                collection_id: fixture.collection.clone(),
                request_id: None,
                environment_id: Some(fixture.dev.clone()),
            }),
        },
    })
}

#[tokio::test]
async fn saved_secrets_are_encrypted_for_the_user_and_hidden_from_the_ui() {
    let fixture = Fixture::new();
    with_token(&fixture);
    assert_eq!(
        fixture.store.recipients_of(&fixture.root),
        [FakeVault::OWN_KEY]
    );
    assert_eq!(stored(&fixture), Some(values(&[("token", "s3cr3t")])));
    assert_eq!(dev(&fixture).variables[1], secret("token", ""));
    let in_store = &fixture.store.environments(&fixture.root)[0].variables[1];
    assert!(in_store.secret);
    assert_eq!(in_store.value, "");
}

#[tokio::test]
async fn sending_uses_the_decrypted_secrets() {
    let fixture =
        Fixture::with_http(FakeHttpSender::new().reply(Ok(response(200, "text/plain", ""))));
    with_token(&fixture);
    let mut events = fixture.app.subscribe();
    send(&fixture, "{{baseUrl}}/me?token={{ token }}").unwrap();
    tokio::time::timeout(Duration::from_secs(5), events.recv())
        .await
        .expect("run finishes")
        .expect("channel open");
    assert_eq!(
        fixture.http.sent()[0].url.as_str(),
        "https://dev.example.com/me?token=s3cr3t"
    );
}

#[tokio::test]
async fn empty_values_keep_secrets_and_unchanged_ones_are_not_rewritten() {
    let fixture = Fixture::new();
    with_token(&fixture);

    // Only the name changes: the secrets file is not touched at all.
    let marker = b"untouched".to_vec();
    fixture
        .store
        .set_secrets(&fixture.root, dev_id(&fixture), marker.clone());
    let mut draft = dev(&fixture);
    draft.name = "Development".into();
    save(&fixture, draft).unwrap();
    assert_eq!(
        fixture.store.secrets(&fixture.root, dev_id(&fixture)),
        Some(marker)
    );

    // A new secret next to a kept one: both are stored.
    with_token(&fixture);
    let mut draft = dev(&fixture);
    draft.variables.push(secret("password", "hunter2"));
    save(&fixture, draft).unwrap();
    assert_eq!(
        stored(&fixture),
        Some(values(&[("password", "hunter2"), ("token", "s3cr3t")]))
    );

    // A variable marked secret but never given a value is not stored.
    let mut draft = dev(&fixture);
    draft.variables.push(secret("unset", ""));
    save(&fixture, draft).unwrap();
    assert_eq!(stored(&fixture).unwrap().len(), 2);
}

#[tokio::test]
async fn without_secrets_the_secrets_file_goes() {
    let fixture = Fixture::new();
    with_token(&fixture);
    let mut draft = dev(&fixture);
    draft.variables.retain(|variable| !variable.secret);
    save(&fixture, draft).unwrap();
    assert_eq!(fixture.store.secrets(&fixture.root, dev_id(&fixture)), None);
}

#[tokio::test]
async fn existing_recipients_are_used_as_they_are() {
    let fixture = Fixture::new();
    fixture
        .store
        .set_recipients(&fixture.root, vec!["age1me".into(), "age1team".into()]);
    with_token(&fixture);
    let data = fixture
        .store
        .secrets(&fixture.root, dev_id(&fixture))
        .unwrap();
    assert!(String::from_utf8(data).unwrap().contains("age1team"));

    fixture
        .store
        .set_recipients(&fixture.root, vec!["alice@example.com".into()]);
    let mut draft = dev(&fixture);
    draft.variables[1].value = "new".into();
    let error = save(&fixture, draft).unwrap_err();
    assert_eq!(error.user_text().key, keys::SECRET_INVALID_RECIPIENT);
}

#[tokio::test]
async fn secrets_of_others_stay_locked_but_plain_variables_work() {
    let fixture = Fixture::with_http(
        FakeHttpSender::new()
            .reply(Ok(response(200, "text/plain", "")))
            .reply(Ok(response(200, "text/plain", ""))),
    );
    with_token(&fixture);
    let sealed = FakeVault::seal(&values(&[("token", "theirs")]), &["age1other"]);
    fixture
        .store
        .set_secrets(&fixture.root, dev_id(&fixture), sealed);

    send(&fixture, "{{baseUrl}}/health").unwrap();
    let error = send(&fixture, "{{baseUrl}}/me?t={{token}}").unwrap_err();
    assert_eq!(
        error,
        AppError::SecretUnavailable {
            name: "token".into(),
            reason: SecretError::NoKey,
        }
    );
    assert_eq!(error.user_text().key, keys::SECRET_UNAVAILABLE);
    assert_eq!(error.user_text().args["name"], "token");

    // Keeping a locked secret while changing secrets is refused …
    let mut draft = dev(&fixture);
    draft.variables.push(secret("password", "hunter2"));
    let error = save(&fixture, draft).unwrap_err();
    assert_eq!(error.user_text().key, keys::SECRET_NO_KEY);

    // … replacing it works and makes it the user's again.
    let mut draft = dev(&fixture);
    draft.variables[1].value = "mine".into();
    save(&fixture, draft).unwrap();
    assert_eq!(stored(&fixture), Some(values(&[("token", "mine")])));
}

#[tokio::test]
async fn missing_and_damaged_secrets_are_explained() {
    let fixture = Fixture::new();
    let mut draft = dev(&fixture);
    draft.variables.push(secret("token", ""));
    save(&fixture, draft).unwrap();
    let error = send(&fixture, "{{baseUrl}}/{{token}}").unwrap_err();
    assert_eq!(error.user_text().key, keys::SECRET_UNAVAILABLE);
    let error = send(&fixture, "{{baseUrl}}/{{other}}").unwrap_err();
    assert_eq!(error.user_text().key, keys::VARIABLE_UNKNOWN);

    fixture
        .store
        .set_secrets(&fixture.root, dev_id(&fixture), b"garbage".to_vec());
    let error = send(&fixture, "{{baseUrl}}/{{token}}").unwrap_err();
    assert_eq!(error.user_text().key, keys::SECRET_DAMAGED);
}

#[tokio::test]
async fn key_failures_are_reported() {
    let fixture = Fixture::new();
    fixture
        .vault
        .fail_next(SecretError::Key("disk full".into()));
    let mut draft = dev(&fixture);
    draft.variables.push(secret("token", "x"));
    let error = save(&fixture, draft).unwrap_err();
    assert_eq!(error.user_text().key, keys::SECRET_KEY);
    assert_eq!(error.user_text().args["detail"], "disk full");
    assert_eq!(fixture.store.secrets(&fixture.root, dev_id(&fixture)), None);

    let error = AppError::from(SecretError::NoRecipients);
    assert_eq!(error.user_text().key, keys::SECRET_NO_RECIPIENTS);
}

use ac_domain::collection::ItemId;

use crate::command::{Command, CommandAck, Query, QueryResult};
use crate::error::AppError;
use crate::ports::{SecretError, SecretValues, SecretVault};
use crate::test_support::Fixture;
use crate::testing::FakeVault;
use crate::text::keys;
use crate::view::{RecipientsView, VariableDraft};

/// Gives Dev the secret `token`; the recipients are then the user's key.
fn with_token(fixture: &Fixture) {
    let mut draft = fixture.tree().environments[0].clone();
    draft.variables.push(VariableDraft {
        name: "token".into(),
        value: "s3cr3t".into(),
        enabled: true,
        secret: true,
    });
    fixture.run(Command::SaveEnvironment {
        collection_id: fixture.collection.clone(),
        environment: draft,
    });
}

fn dev_id(fixture: &Fixture) -> ItemId {
    fixture.dev.parse().unwrap()
}

fn sealed(fixture: &Fixture) -> Vec<u8> {
    fixture
        .store
        .secrets(&fixture.root, dev_id(fixture))
        .unwrap()
}

fn set(fixture: &Fixture, recipients: &[&str]) -> Result<CommandAck, AppError> {
    fixture.app.execute(Command::SetRecipients {
        collection_id: fixture.collection.clone(),
        recipients: recipients.iter().map(|&r| r.to_owned()).collect(),
    })
}

fn recipients(fixture: &Fixture) -> RecipientsView {
    let result = fixture
        .app
        .query(Query::Recipients {
            collection_id: fixture.collection.clone(),
        })
        .unwrap();
    let QueryResult::Recipients { recipients } = result else {
        panic!("expected recipients, got {result:?}");
    };
    recipients
}

#[tokio::test]
async fn shows_the_own_key_and_the_recipients() {
    let fixture = Fixture::new();
    assert_eq!(
        recipients(&fixture),
        RecipientsView {
            own: FakeVault::OWN_KEY.into(),
            recipients: Vec::new(),
        }
    );
    with_token(&fixture);
    assert_eq!(recipients(&fixture).recipients, [FakeVault::OWN_KEY]);
}

#[tokio::test]
async fn new_recipients_get_every_secret_re_encrypted() {
    let fixture = Fixture::new();
    with_token(&fixture);
    set(&fixture, &["age1me", " age1team ", "", "age1team"]).unwrap();
    assert_eq!(
        fixture.store.recipients_of(&fixture.root),
        ["age1me", "age1team"]
    );
    let data = sealed(&fixture);
    assert!(
        String::from_utf8(data.clone())
            .unwrap()
            .contains("age1team")
    );
    let team = FakeVault::with_key("age1team").decrypt(&data).unwrap();
    assert_eq!(
        team,
        SecretValues::from([("token".to_owned(), "s3cr3t".to_owned())])
    );

    // Removing oneself is possible; afterwards only the team can decrypt.
    set(&fixture, &["age1team"]).unwrap();
    assert_eq!(
        FakeVault::default().decrypt(&sealed(&fixture)),
        Err(SecretError::NoKey)
    );
}

#[tokio::test]
async fn the_same_list_changes_nothing() {
    let fixture = Fixture::new();
    with_token(&fixture);
    let marker = b"untouched".to_vec();
    fixture
        .store
        .set_secrets(&fixture.root, dev_id(&fixture), marker.clone());
    set(&fixture, &[" age1me "]).unwrap();
    assert_eq!(sealed(&fixture), marker);
}

#[tokio::test]
async fn invalid_or_empty_lists_are_refused() {
    let fixture = Fixture::new();
    with_token(&fixture);
    let before = sealed(&fixture);

    let error = set(&fixture, &["age1team", "bob@example.com"]).unwrap_err();
    assert_eq!(error.user_text().key, keys::SECRET_INVALID_RECIPIENT);
    let error = set(&fixture, &["  "]).unwrap_err();
    assert_eq!(error.user_text().key, keys::SECRET_NO_RECIPIENTS);

    assert_eq!(fixture.store.recipients_of(&fixture.root), ["age1me"]);
    assert_eq!(sealed(&fixture), before);
}

#[tokio::test]
async fn secrets_the_user_cannot_decrypt_leave_everything_as_it_was() {
    let fixture = Fixture::new();
    with_token(&fixture);
    let theirs = FakeVault::seal(&SecretValues::new(), &["age1other"]);
    fixture
        .store
        .set_secrets(&fixture.root, dev_id(&fixture), theirs.clone());

    let error = set(&fixture, &["age1me", "age1team"]).unwrap_err();
    assert_eq!(
        error,
        AppError::NotReencrypted {
            environment: "Dev".into(),
            reason: SecretError::NoKey,
        }
    );
    assert_eq!(error.user_text().key, keys::SECRET_NOT_REENCRYPTED);
    assert_eq!(error.user_text().args["environment"], "Dev");
    assert_eq!(fixture.store.recipients_of(&fixture.root), ["age1me"]);
    assert_eq!(sealed(&fixture), theirs);

    fixture
        .store
        .set_secrets(&fixture.root, dev_id(&fixture), b"garbage".to_vec());
    let error = set(&fixture, &["age1team"]).unwrap_err();
    assert_eq!(error.user_text().key, keys::SECRET_DAMAGED);
}

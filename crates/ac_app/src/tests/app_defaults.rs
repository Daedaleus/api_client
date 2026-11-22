use std::time::Duration;

use crate::command::{Command, CommandAck};
use crate::error::AppError;
use crate::ports::StoreErrorKind;
use crate::test_support::Fixture;
use crate::testing::{FakeHttpSender, response};
use crate::view::{AuthDraft, BodyDraft, SendContext, TreeItemView, VariableDraft};

fn plain(name: &str, value: &str) -> VariableDraft {
    VariableDraft {
        name: name.into(),
        value: value.into(),
        enabled: true,
        secret: false,
    }
}

fn save(
    fixture: &Fixture,
    item_id: &str,
    variables: Vec<VariableDraft>,
) -> Result<CommandAck, AppError> {
    fixture.app.execute(Command::SaveDefaults {
        transport: crate::view::TransportDraft::default(),
        scripts: crate::view::ScriptsDraft::default(),
        certificate_check: crate::view::CertificateCheckView::Inherit,
        collection_id: fixture.collection.clone(),
        item_id: item_id.into(),
        variables,
        auth: AuthDraft::Inherit,
    })
}

/// Sends `url` as the request `request_id` with `own` variables and Dev.
async fn sent_url(
    fixture: &Fixture,
    request_id: Option<&str>,
    url: &str,
    own: Vec<VariableDraft>,
    environment: bool,
) -> String {
    let mut events = fixture.app.subscribe();
    fixture
        .app
        .execute(Command::SendRequest {
            request: crate::view::OutgoingRequest {
                transport: crate::view::TransportDraft::default(),
                path_params: Vec::new(),
                certificate_check: crate::view::CertificateCheckView::Inherit,
                scripts: crate::view::ScriptsDraft::default(),
                method: "GET".into(),
                url: url.into(),
                headers: Vec::new(),
                body: BodyDraft::None,
                variables: own,
                auth: AuthDraft::Inherit,
                context: Some(SendContext {
                    collection_id: fixture.collection.clone(),
                    request_id: request_id.map(Into::into),
                    environment_id: environment.then(|| fixture.dev.clone()),
                }),
            },
        })
        .unwrap();
    tokio::time::timeout(Duration::from_secs(5), events.recv())
        .await
        .expect("run finishes")
        .expect("channel open");
    fixture.http.sent().last().unwrap().url.to_string()
}

fn replies(count: usize) -> FakeHttpSender {
    (0..count).fold(FakeHttpSender::new(), |fake, _| {
        fake.reply(Ok(response(200, "text/plain", "")))
    })
}

#[tokio::test]
async fn the_request_beats_folders_beat_the_environment_beats_the_collection() {
    let fixture = Fixture::with_http(replies(4));
    save(
        &fixture,
        &fixture.collection.clone(),
        vec![
            plain("baseUrl", "https://collection.example.com"),
            plain("host", "collection"),
            plain("a", "1"),
        ],
    )
    .unwrap();
    save(
        &fixture,
        &fixture.auth.clone(),
        vec![plain("host", "folder")],
    )
    .unwrap();
    let url = "{{baseUrl}}/{{host}}/{{a}}";

    let login = fixture.login.clone();
    assert_eq!(
        sent_url(&fixture, Some(&login), url, Vec::new(), true).await,
        "https://dev.example.com/folder/1"
    );
    assert_eq!(
        sent_url(
            &fixture,
            Some(&login),
            url,
            vec![plain("host", "own")],
            true
        )
        .await,
        "https://dev.example.com/own/1"
    );
    let health = fixture.health.clone();
    assert_eq!(
        sent_url(&fixture, Some(&health), url, Vec::new(), true).await,
        "https://dev.example.com/collection/1"
    );
    assert_eq!(
        sent_url(&fixture, None, url, Vec::new(), false).await,
        "https://collection.example.com/collection/1"
    );
}

#[tokio::test]
async fn without_context_only_the_own_variables_count() {
    let fixture = Fixture::with_http(replies(1));
    let mut events = fixture.app.subscribe();
    fixture
        .app
        .execute(Command::SendRequest {
            request: crate::view::OutgoingRequest {
                transport: crate::view::TransportDraft::default(),
                path_params: Vec::new(),
                certificate_check: crate::view::CertificateCheckView::Inherit,
                scripts: crate::view::ScriptsDraft::default(),
                method: "GET".into(),
                url: "{{host}}/x".into(),
                headers: Vec::new(),
                body: BodyDraft::None,
                variables: vec![plain("host", "own.example.com")],
                auth: AuthDraft::Inherit,
                context: None,
            },
        })
        .unwrap();
    events.recv().await.unwrap();
    assert_eq!(
        fixture.http.sent()[0].url.as_str(),
        "http://own.example.com/x"
    );
}

#[tokio::test]
async fn variables_are_saved_for_the_collection_folders_and_requests() {
    let fixture = Fixture::new();
    let mut marked = plain("token", "abc");
    marked.secret = true;
    save(&fixture, &fixture.collection.clone(), vec![marked]).unwrap();
    save(&fixture, &fixture.auth.clone(), vec![plain("v", "2")]).unwrap();

    let tree = fixture.tree();
    // Only environments have secrets: elsewhere a secret row is plain.
    assert_eq!(tree.variables, [plain("token", "abc")]);
    let TreeItemView::Folder { variables, .. } = &tree.items[0] else {
        panic!("expected the Auth folder");
    };
    assert_eq!(*variables, [plain("v", "2")]);

    let mut draft = fixture.draft(&fixture.login);
    draft.variables = vec![plain("path", "login")];
    fixture.save(draft).unwrap();
    assert_eq!(
        fixture.draft(&fixture.login).variables,
        [plain("path", "login")]
    );
}

#[tokio::test]
async fn variables_go_only_to_the_collection_and_its_folders() {
    let fixture = Fixture::new();
    let health = fixture.health.clone();
    assert_eq!(
        save(&fixture, &health, Vec::new()),
        Err(AppError::NotAFolder)
    );
    let unknown = ac_domain::collection::ItemId::new().to_string();
    assert_eq!(
        save(&fixture, &unknown, Vec::new()),
        Err(AppError::UnknownItem)
    );

    fixture.store.mark_read_only(fixture.auth.parse().unwrap());
    let error = save(&fixture, &fixture.auth.clone(), Vec::new()).unwrap_err();
    assert!(matches!(
        error,
        AppError::Store(ref store) if store.kind == StoreErrorKind::ReadOnly
    ));
}

#[tokio::test]
async fn requests_inherit_auth_from_folders_and_the_collection() {
    let fixture = Fixture::with_http(replies(3));
    let bearer = AuthDraft::Bearer {
        token: "{{baseUrl}}".into(),
    };
    let basic = AuthDraft::Basic {
        username: "a".into(),
        password: "b".into(),
    };
    let save_auth = |item: &str, auth: AuthDraft| {
        fixture
            .app
            .execute(Command::SaveDefaults {
                transport: crate::view::TransportDraft::default(),
                scripts: crate::view::ScriptsDraft::default(),
                certificate_check: crate::view::CertificateCheckView::Inherit,
                collection_id: fixture.collection.clone(),
                item_id: item.into(),
                variables: Vec::new(),
                auth,
            })
            .unwrap();
    };
    save_auth(&fixture.collection, bearer.clone());
    save_auth(&fixture.auth, basic);
    assert_eq!(fixture.tree().auth, bearer);

    let send = |request: &str, auth: AuthDraft| {
        let mut events = fixture.app.subscribe();
        fixture
            .app
            .execute(Command::SendRequest {
                request: crate::view::OutgoingRequest {
                    transport: crate::view::TransportDraft::default(),
                    path_params: Vec::new(),
                    certificate_check: crate::view::CertificateCheckView::Inherit,
                    scripts: crate::view::ScriptsDraft::default(),
                    method: "GET".into(),
                    url: "x.example".into(),
                    headers: Vec::new(),
                    body: BodyDraft::None,
                    variables: Vec::new(),
                    auth,
                    context: Some(SendContext {
                        collection_id: fixture.collection.clone(),
                        request_id: Some(request.into()),
                        environment_id: Some(fixture.dev.clone()),
                    }),
                },
            })
            .unwrap();
        async move { events.recv().await.unwrap() }
    };
    send(&fixture.login.clone(), AuthDraft::Inherit).await;
    send(&fixture.health.clone(), AuthDraft::Inherit).await;
    send(&fixture.health.clone(), AuthDraft::None).await;
    let authorization = |index: usize| {
        fixture.http.sent()[index]
            .headers
            .iter()
            .find(|header| header.name == "authorization")
            .map(|header| header.value.clone())
    };
    assert_eq!(authorization(0).as_deref(), Some("Basic YTpi"));
    assert_eq!(
        authorization(1).as_deref(),
        Some("Bearer https://dev.example.com")
    );
    assert_eq!(authorization(2), None);
}

#[tokio::test]
async fn file_paths_are_relative_to_the_collection_folder() {
    let fixture = Fixture::with_http(replies(1));
    let mut events = fixture.app.subscribe();
    fixture
        .app
        .execute(Command::SendRequest {
            request: crate::view::OutgoingRequest {
                transport: crate::view::TransportDraft::default(),
                path_params: Vec::new(),
                certificate_check: crate::view::CertificateCheckView::Inherit,
                scripts: crate::view::ScriptsDraft::default(),
                method: "PUT".into(),
                url: "x.example".into(),
                headers: Vec::new(),
                body: BodyDraft::Binary {
                    file: "data/payload.bin".into(),
                },
                variables: Vec::new(),
                auth: AuthDraft::Inherit,
                context: Some(SendContext {
                    collection_id: fixture.collection.clone(),
                    request_id: Some(fixture.health.clone()),
                    environment_id: None,
                }),
            },
        })
        .unwrap();
    events.recv().await.unwrap();
    assert_eq!(
        fixture.http.sent()[0].body,
        Some(ac_domain::exchange::RequestBody::File(
            fixture.root.join("data/payload.bin")
        ))
    );
}

#[tokio::test]
async fn the_certificate_check_is_inherited_and_on_by_default() {
    use crate::view::CertificateCheckView::{Inherit, Off, On};
    let fixture = Fixture::with_http(replies(4));
    let send = |request: &str, check| {
        let mut events = fixture.app.subscribe();
        fixture
            .app
            .execute(Command::SendRequest {
                request: crate::view::OutgoingRequest {
                    transport: crate::view::TransportDraft::default(),
                    path_params: Vec::new(),
                    method: "GET".into(),
                    url: "https://x.example".into(),
                    headers: Vec::new(),
                    body: BodyDraft::None,
                    variables: Vec::new(),
                    auth: AuthDraft::Inherit,
                    certificate_check: check,
                    scripts: crate::view::ScriptsDraft::default(),
                    context: Some(SendContext {
                        collection_id: fixture.collection.clone(),
                        request_id: Some(request.into()),
                        environment_id: None,
                    }),
                },
            })
            .unwrap();
        async move { events.recv().await.unwrap() }
    };
    send(&fixture.login.clone(), Inherit).await;
    fixture
        .app
        .execute(Command::SaveDefaults {
            transport: crate::view::TransportDraft::default(),
            scripts: crate::view::ScriptsDraft::default(),
            collection_id: fixture.collection.clone(),
            item_id: fixture.auth.clone(),
            variables: Vec::new(),
            auth: AuthDraft::Inherit,
            certificate_check: Off,
        })
        .unwrap();
    let TreeItemView::Folder {
        certificate_check, ..
    } = &fixture.tree().items[0]
    else {
        panic!("expected the Auth folder");
    };
    assert_eq!(*certificate_check, Off);
    send(&fixture.login.clone(), Inherit).await;
    send(&fixture.login.clone(), On).await;
    send(&fixture.health.clone(), Inherit).await;
    let checks: Vec<bool> = fixture
        .http
        .sent()
        .iter()
        .map(|request| request.verify_tls)
        .collect();
    assert_eq!(checks, [true, false, true, true]);
}

#[tokio::test]
async fn redirects_and_time_limit_are_inherited() {
    let fixture = Fixture::with_http(replies(3));
    fixture
        .app
        .execute(Command::SaveDefaults {
            transport: crate::view::TransportDraft {
                redirects: crate::view::RedirectsView::Stop,
                timeout_seconds: Some(5),
            },
            scripts: crate::view::ScriptsDraft::default(),
            certificate_check: crate::view::CertificateCheckView::Inherit,
            collection_id: fixture.collection.clone(),
            item_id: fixture.auth.clone(),
            variables: Vec::new(),
            auth: AuthDraft::Inherit,
        })
        .unwrap();
    let tree = fixture.tree();
    assert!(matches!(
        &tree.items[0],
        crate::view::TreeItemView::Folder { transport, .. } if transport.timeout_seconds == Some(5)
    ));

    // In the folder: its settings; outside: the defaults.
    sent_url(
        &fixture,
        Some(&fixture.login.clone()),
        "https://x.example/a",
        Vec::new(),
        false,
    )
    .await;
    sent_url(
        &fixture,
        Some(&fixture.health.clone()),
        "https://x.example/b",
        Vec::new(),
        false,
    )
    .await;
    let sent = fixture.http.sent();
    assert!(!sent[0].transport.follow_redirects);
    assert_eq!(sent[0].transport.timeout, Duration::from_secs(5));
    assert!(sent[1].transport.follow_redirects);
    assert_eq!(sent[1].transport.timeout, Duration::from_secs(30));
}

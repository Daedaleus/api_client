use super::*;
use crate::command::{Command, Query};
use crate::test_support::{Fixture, changed, names};
use crate::text::keys;
use crate::view::{BodyDraft, TreeItemView};

#[tokio::test]
async fn opening_shows_the_tree() {
    let fixture = Fixture::new();
    let tree = fixture.tree();
    assert_eq!(tree.name, "Shop");
    assert_eq!(tree.root, "shop");
    assert_eq!(names(&tree.items), ["Auth/POST Login", "GET Health"]);

    // Opening again does not open it twice: one close closes it.
    fixture.run(Command::OpenCollection {
        path: "shop".into(),
    });
    let close = || Command::CloseCollection {
        collection_id: fixture.collection.clone(),
    };
    assert!(matches!(
        fixture.run(close()),
        CommandAck::CollectionClosed { .. }
    ));
    assert_eq!(fixture.fail(close()), AppError::UnknownCollection);
}

#[tokio::test]
async fn closed_and_unknown_collections_are_refused() {
    let fixture = Fixture::new();
    fixture.run(Command::CloseCollection {
        collection_id: fixture.collection.clone(),
    });
    let error = fixture
        .app
        .query(Query::Collection {
            collection_id: fixture.collection.clone(),
        })
        .unwrap_err();
    assert_eq!(error, AppError::UnknownCollection);
    let error = fixture.fail(Command::CloseCollection {
        collection_id: "not-an-id".into(),
    });
    assert_eq!(error.user_text().key, keys::UNKNOWN_COLLECTION);
}

#[tokio::test]
async fn missing_collections_report_the_store_error() {
    let fixture = Fixture::new();
    let error = fixture.fail(Command::OpenCollection {
        path: "elsewhere".into(),
    });
    assert_eq!(error.user_text().key, keys::NOT_A_COLLECTION);
    assert_eq!(error.user_text().args["path"], "elsewhere");
}

#[tokio::test]
async fn requests_load_into_the_editor() {
    let fixture = Fixture::new();
    let draft = fixture.draft(&fixture.login);
    assert_eq!(draft.name, "Login");
    assert_eq!(draft.method, "POST");
    assert_eq!(draft.url, "{{baseUrl}}/login");
    assert_eq!(
        draft.body,
        BodyDraft::Json {
            content: "{}".into()
        }
    );
    assert!(!draft.read_only);

    let query = |id: &str| {
        fixture.app.query(Query::Request {
            collection_id: fixture.collection.clone(),
            request_id: id.into(),
        })
    };
    assert_eq!(query(&fixture.auth).unwrap_err(), AppError::UnknownItem);
    assert_eq!(query("nope").unwrap_err(), AppError::UnknownItem);
    assert_eq!(
        query(&ItemId::new().to_string()).unwrap_err(),
        AppError::UnknownItem
    );
}

#[tokio::test]
async fn saving_writes_the_edited_request() {
    let fixture = Fixture::new();
    let mut draft = fixture.draft(&fixture.health);
    draft.url = "https://example.com/health".into();
    draft.method = "HEAD".into();
    draft.name = "  Ping  ".into();
    let (tree, created) = changed(fixture.save(draft).unwrap());
    assert_eq!(created, None);
    assert_eq!(names(&tree.items)[1], "HEAD Ping");

    let saved = fixture.draft(&fixture.health);
    assert_eq!(saved.url, "https://example.com/health");
    assert_eq!(saved.name, "Ping");
}

#[tokio::test]
async fn invalid_drafts_are_refused() {
    let fixture = Fixture::new();
    let draft = fixture.draft(&fixture.health);

    let empty = RequestDraft {
        name: " ".into(),
        ..draft.clone()
    };
    assert_eq!(fixture.save(empty).unwrap_err(), AppError::EmptyName);

    let method = RequestDraft {
        method: "GE T".into(),
        ..draft.clone()
    };
    assert_eq!(
        fixture.save(method).unwrap_err().user_text().key,
        keys::METHOD_INVALID
    );

    let folder = RequestDraft {
        id: fixture.auth.clone(),
        ..draft.clone()
    };
    assert_eq!(fixture.save(folder).unwrap_err(), AppError::UnknownItem);

    let unknown = RequestDraft {
        id: "nope".into(),
        ..draft
    };
    assert_eq!(fixture.save(unknown).unwrap_err(), AppError::UnknownItem);
}

#[tokio::test]
async fn read_only_requests_and_folders_are_not_written() {
    let fixture = Fixture::new();
    fixture
        .store
        .mark_read_only(fixture.health.parse().unwrap());
    fixture.store.mark_read_only(fixture.auth.parse().unwrap());

    let draft = fixture.draft(&fixture.health);
    assert!(draft.read_only);
    let error = fixture.save(draft).unwrap_err();
    assert_eq!(error.user_text().key, keys::STORE_READ_ONLY);

    let error = fixture.fail(Command::RenameItem {
        collection_id: fixture.collection.clone(),
        item_id: fixture.auth.clone(),
        name: "x".into(),
    });
    assert_eq!(error.user_text().key, keys::STORE_READ_ONLY);
    let tree = fixture.tree();
    assert!(matches!(
        tree.items[0],
        TreeItemView::Folder {
            read_only: true,
            ..
        }
    ));
}

#[tokio::test]
async fn new_requests_and_folders_go_into_the_collection_or_a_folder() {
    let fixture = Fixture::new();
    let create_request = |parent: &str, name: &str| Command::CreateRequest {
        collection_id: fixture.collection.clone(),
        parent_id: parent.into(),
        name: name.into(),
    };

    let (tree, created) = changed(fixture.run(create_request(&fixture.collection, "Status")));
    assert_eq!(
        names(&tree.items),
        ["Auth/POST Login", "GET Health", "GET Status"]
    );
    let created = created.unwrap();
    assert_eq!(fixture.draft(&created).name, "Status");

    let (tree, _) = changed(fixture.run(create_request(&fixture.auth, "Logout")));
    assert_eq!(names(&tree.items)[0], "Auth/POST Login,GET Logout");

    let (tree, folder) = changed(fixture.run(Command::CreateFolder {
        collection_id: fixture.collection.clone(),
        parent_id: fixture.auth.clone(),
        name: "Tokens".into(),
    }));
    assert!(folder.is_some());
    assert_eq!(names(&tree.items)[0], "Auth/POST Login,GET Logout,Tokens/");

    assert_eq!(
        fixture.fail(create_request(&fixture.health, "x")),
        AppError::NotAFolder
    );
    assert_eq!(
        fixture.fail(create_request(&ItemId::new().to_string(), "x")),
        AppError::UnknownItem
    );
    assert_eq!(
        fixture.fail(create_request(&fixture.auth, "  ")),
        AppError::EmptyName
    );
}

#[tokio::test]
async fn requests_and_folders_can_be_renamed() {
    let fixture = Fixture::new();
    let rename = |id: &str, name: &str| Command::RenameItem {
        collection_id: fixture.collection.clone(),
        item_id: id.into(),
        name: name.into(),
    };
    fixture.run(rename(&fixture.login, "Sign in"));
    let (tree, _) = changed(fixture.run(rename(&fixture.auth, "Authentication")));
    assert_eq!(names(&tree.items)[0], "Authentication/POST Sign in");
    // Renaming keeps everything else.
    assert_eq!(fixture.draft(&fixture.login).url, "{{baseUrl}}/login");

    assert_eq!(
        fixture.fail(rename(&ItemId::new().to_string(), "x")),
        AppError::UnknownItem
    );
    assert_eq!(fixture.fail(rename(&fixture.auth, "")), AppError::EmptyName);
}

#[tokio::test]
async fn deleting_removes_items() {
    let fixture = Fixture::new();
    let delete = |id: &str| Command::DeleteItem {
        collection_id: fixture.collection.clone(),
        item_id: id.into(),
    };
    let (tree, _) = changed(fixture.run(delete(&fixture.auth)));
    assert_eq!(names(&tree.items), ["GET Health"]);
    assert_eq!(fixture.store.collection(&fixture.root).items.len(), 1);

    assert_eq!(fixture.fail(delete(&fixture.auth)), AppError::UnknownItem);
    assert_eq!(
        fixture.fail(delete(&fixture.collection)),
        AppError::UnknownItem
    );
}

#[tokio::test]
async fn collections_can_be_created() {
    let fixture = Fixture::new();
    let create = |path: &str, name: &str| Command::CreateCollection {
        path: path.into(),
        name: name.into(),
    };
    let (tree, _) = changed(fixture.run(create("new", " New API ")));
    assert_eq!(tree.name, "New API");
    assert_eq!(tree.items, vec![]);

    let error = fixture.fail(create("new", "Again"));
    assert_eq!(error.user_text().key, keys::COLLECTION_EXISTS);
    assert_eq!(fixture.fail(create("other", "")), AppError::EmptyName);
}

#[tokio::test]
async fn store_problems_and_failures_reach_the_ui() {
    let fixture = Fixture::new();
    fixture.store.add_problem(StoreError {
        kind: StoreErrorKind::Damaged,
        path: "shop/broken.toml".into(),
        detail: "not valid TOML".into(),
    });
    let tree = fixture.tree();
    assert_eq!(tree.problems.len(), 1);
    assert_eq!(tree.problems[0].key, keys::STORE_DAMAGED);
    assert_eq!(tree.problems[0].args["detail"], "not valid TOML");

    fixture.store.fail_next(StoreError {
        kind: StoreErrorKind::Io,
        path: "shop".into(),
        detail: "disk full".into(),
    });
    let error = fixture.fail(Command::CreateFolder {
        collection_id: fixture.collection.clone(),
        parent_id: fixture.collection.clone(),
        name: "x".into(),
    });
    assert_eq!(error.user_text().key, keys::STORE_IO);
    assert!(error.to_string().contains("disk full"));
}

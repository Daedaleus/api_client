//! Test helpers shared by the app tests: a small collection in the
//! in-memory store, opened in an [`App`].

use std::path::PathBuf;
use std::sync::Arc;

use ac_domain::collection::{Body, Collection, Folder, Item, ItemId, Request};
use ac_domain::environment::{Environment, Variable};
use ac_domain::protocol::HttpMethod;
use tokio::runtime::Handle;

use crate::app::{App, Ports};
use crate::command::{Command, CommandAck, Query, QueryResult};
use crate::error::AppError;
use crate::testing::{FakeHttpSender, MemoryStore};
use crate::view::{CollectionView, RequestDraft, TreeItemView};

/// `Shop` with `Auth/Login` and `Health`, and the environment `Dev`
/// (`baseUrl`); opened in an app. Must be created inside a Tokio runtime.
pub(crate) struct Fixture {
    pub app: App<Arc<FakeHttpSender>, Arc<MemoryStore>>,
    pub http: Arc<FakeHttpSender>,
    pub store: Arc<MemoryStore>,
    pub root: PathBuf,
    pub collection: String,
    pub auth: String,
    pub login: String,
    pub health: String,
    pub dev: String,
}

impl Fixture {
    pub fn new() -> Self {
        Self::with_http(FakeHttpSender::new())
    }

    pub fn with_http(http: FakeHttpSender) -> Self {
        let mut login = Request::new("Login");
        login.method = HttpMethod::Post;
        login.url = "{{baseUrl}}/login".into();
        login.body = Body::Json("{}".into());
        let health = Request::new("Health");
        let auth = Folder {
            id: ItemId::new(),
            name: "Auth".into(),
            items: vec![Item::Request(login.clone())],
        };
        let collection = Collection {
            id: ItemId::new(),
            name: "Shop".into(),
            items: vec![Item::Folder(auth.clone()), Item::Request(health.clone())],
        };
        let mut dev = Environment::new("Dev");
        dev.variables.push(Variable {
            name: "baseUrl".into(),
            value: "https://dev.example.com".into(),
            enabled: true,
        });
        let root = PathBuf::from("shop");
        let store = Arc::new(
            MemoryStore::new()
                .with(&root, collection.clone())
                .with_environment(&root, dev.clone()),
        );
        let http = Arc::new(http);
        let app = App::new(
            Ports {
                http: Arc::clone(&http),
                store: Arc::clone(&store),
            },
            Handle::current(),
        );
        let fixture = Self {
            app,
            http,
            store,
            root,
            collection: collection.id.to_string(),
            auth: auth.id.to_string(),
            login: login.id.to_string(),
            health: health.id.to_string(),
            dev: dev.id.to_string(),
        };
        fixture.run(Command::OpenCollection {
            path: "shop".into(),
        });
        fixture
    }

    pub fn run(&self, command: Command) -> CommandAck {
        self.app.execute(command).unwrap()
    }

    pub fn fail(&self, command: Command) -> AppError {
        self.app.execute(command).unwrap_err()
    }

    pub fn draft(&self, request_id: &str) -> RequestDraft {
        let result = self
            .app
            .query(Query::Request {
                collection_id: self.collection.clone(),
                request_id: request_id.into(),
            })
            .unwrap();
        let QueryResult::Request { request } = result else {
            panic!("expected a request, got {result:?}");
        };
        request
    }

    pub fn tree(&self) -> CollectionView {
        let result = self
            .app
            .query(Query::Collection {
                collection_id: self.collection.clone(),
            })
            .unwrap();
        let QueryResult::Collection { collection } = result else {
            panic!("expected a collection, got {result:?}");
        };
        collection
    }

    pub fn save(&self, request: RequestDraft) -> Result<CommandAck, AppError> {
        self.app.execute(Command::SaveRequest {
            collection_id: self.collection.clone(),
            request,
        })
    }
}

/// Names in a tree: `GET Health`, folders as `Name/child,child`.
pub(crate) fn names(items: &[TreeItemView]) -> Vec<String> {
    items
        .iter()
        .map(|item| match item {
            TreeItemView::Folder { name, items, .. } => {
                format!("{name}/{}", names(items).join(","))
            }
            TreeItemView::Request { name, method, .. } => format!("{method} {name}"),
        })
        .collect()
}

/// The tree and created id of a `collectionChanged` acknowledgement.
pub(crate) fn changed(ack: CommandAck) -> (CollectionView, Option<String>) {
    let CommandAck::CollectionChanged {
        collection,
        created_id,
    } = ack
    else {
        panic!("expected a changed collection, got {ack:?}");
    };
    (collection, created_id)
}

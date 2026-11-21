use std::path::{Path, PathBuf};

use ac_domain::auth::{Auth, KeyPlacement};
use ac_domain::collection::{Body, HeaderEntry, MultipartField, Request};
use ac_domain::protocol::HttpMethod;

use super::*;

fn fixture() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/tests/fixtures/bruno-shop")
}

/// The fixture's files as the store would hand them over.
fn files(root: &Path) -> Vec<BrunoFile> {
    fn walk(root: &Path, dir: &Path, out: &mut Vec<BrunoFile>) {
        for entry in std::fs::read_dir(dir).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                walk(root, &path, out);
            } else {
                let relative = path.strip_prefix(root).unwrap();
                out.push(BrunoFile {
                    path: relative.to_string_lossy().replace('\\', "/"),
                    text: std::fs::read_to_string(&path).unwrap(),
                });
            }
        }
    }
    let mut out = Vec::new();
    walk(root, root, &mut out);
    out.sort_by(|a, b| a.path.cmp(&b.path));
    out
}

fn shop() -> Imported {
    let root = fixture();
    from_files(&root, &files(&root))
}

fn folder<'a>(items: &'a [Item], name: &str) -> &'a Folder {
    items
        .iter()
        .find_map(|item| match item {
            Item::Folder(folder) if folder.name == name => Some(folder),
            _ => None,
        })
        .unwrap()
}

fn request<'a>(items: &'a [Item], name: &str) -> &'a Request {
    items
        .iter()
        .find_map(|item| match item {
            Item::Request(request) if request.name == name => Some(request),
            _ => None,
        })
        .unwrap()
}

#[test]
fn the_tree_follows_brunos_order() {
    let collection = shop().collection;
    assert_eq!(collection.name, "Shop (Bruno)");
    let names: Vec<_> = collection.items.iter().map(Item::name).collect();
    assert_eq!(
        names,
        ["Products", "Authentication", "Health", "Products (GraphQL)"]
    );
    let products: Vec<_> = folder(&collection.items, "Products")
        .items
        .iter()
        .map(Item::name)
        .collect();
    assert_eq!(
        products,
        ["List products", "Get product", "Upload image", "Search"]
    );
}

#[test]
fn collection_and_folder_settings_and_environments() {
    let imported = shop();
    let collection = &imported.collection;
    assert_eq!(collection.variables, [Variable::plain("apiVersion", "v1")]);
    assert_eq!(
        collection.auth,
        Auth::Bearer {
            token: "{{token}}".into()
        }
    );
    assert_eq!(folder(&collection.items, "Authentication").auth, Auth::None);
    assert_eq!(
        folder(&collection.items, "Products").variables,
        [Variable::plain("page", "1")]
    );

    let [local] = imported.environments.as_slice() else {
        panic!("expected one environment");
    };
    assert_eq!(local.name, "Local");
    assert_eq!(
        local.variables,
        [
            Variable::plain("baseUrl", "http://localhost:8080"),
            Variable {
                enabled: false,
                ..Variable::plain("debug", "1")
            },
            Variable {
                name: "token".into(),
                value: String::new(),
                enabled: true,
                secret: true
            },
            Variable {
                name: "oldToken".into(),
                value: String::new(),
                enabled: false,
                secret: true
            },
        ]
    );
}

#[test]
fn requests_with_bodies_auth_and_path_parameters() {
    let collection = shop().collection;
    let login = request(&folder(&collection.items, "Authentication").items, "Login");
    assert_eq!(login.method, HttpMethod::Post);
    assert_eq!(login.url, "{{baseUrl}}/auth/login");
    assert_eq!(login.auth, Auth::Inherit);
    assert_eq!(
        login.headers,
        [
            HeaderEntry {
                name: "Accept".into(),
                value: "application/json".into(),
                enabled: true
            },
            HeaderEntry {
                name: "X-Debug".into(),
                value: "1".into(),
                enabled: false
            },
        ]
    );
    assert_eq!(
        login.body,
        Body::Json("{\n  \"user\": \"alice\",\n  \"password\": \"{{password}}\"\n}".into())
    );

    let products = &folder(&collection.items, "Products").items;
    let get = request(products, "Get product");
    assert_eq!(get.url, "{{baseUrl}}/products/{{id}}?verbose=true");
    assert_eq!(get.variables, [Variable::plain("id", "42")]);
    assert_eq!(
        get.auth,
        Auth::ApiKey {
            name: "X-API-Key".into(),
            value: "{{apiKey}}".into(),
            placement: KeyPlacement::Header
        }
    );
    let upload = request(products, "Upload image");
    assert_eq!(upload.method, HttpMethod::Put);
    assert_eq!(
        upload.auth,
        Auth::Basic {
            username: "admin".into(),
            password: "{{adminPassword}}".into()
        }
    );
    let image = fixture().join("files/teapot.png").display().to_string();
    assert_eq!(
        upload.body,
        Body::Multipart(vec![
            MultipartField {
                name: "title".into(),
                value: "Teapot".into(),
                enabled: true,
                file: false
            },
            MultipartField {
                name: "image".into(),
                value: image,
                enabled: true,
                file: true
            },
            MultipartField {
                name: "note".into(),
                value: "x".into(),
                enabled: false,
                file: false
            },
        ])
    );
    assert!(
        matches!(&request(products, "Search").body, Body::Form(fields) if fields[0].value == "tea")
    );
    let health = request(&collection.items, "Health");
    assert_eq!(health.method, HttpMethod::Get);
    assert_eq!(health.body, Body::None);
}

#[test]
fn what_has_no_counterpart_is_reported() {
    let imported = shop();
    let mut reported: Vec<_> = imported
        .skipped
        .iter()
        .map(|skipped| (skipped.item.as_str(), skipped.what.clone()))
        .collect();
    reported.sort_by(|a, b| a.0.cmp(b.0));
    assert_eq!(
        reported,
        [
            ("Authentication", SkippedKind::Headers),
            ("Login", SkippedKind::Scripts),
            ("Products (GraphQL)", SkippedKind::Body("graphql".into())),
            ("Products (GraphQL)", SkippedKind::Body("graphql".into())),
            ("Search", SkippedKind::Auth("oauth2".into())),
        ]
    );
}

#[test]
fn bare_and_odd_files() {
    let root = Path::new("/bruno");
    let imported = from_files(
        root,
        &[
            BrunoFile {
                path: "no-meta.bru".into(),
                text: "patch {\n  url: x\n  body: file\n}\nbody:file {\n  file: @file(/abs/data.bin) @contentType(application/octet-stream)\n}".into(),
            },
            BrunoFile {
                path: "deep/er/request.bru".into(),
                text: "meta {\n  name: Deep\n}\nget {\n  url: y\n  body: sparql\n}".into(),
            },
            BrunoFile {
                path: "no-method.bru".into(),
                text: "meta {\n  name: Nothing\n}\nunclosed {\n  a: 1".into(),
            },
            BrunoFile {
                path: "environments/nested/ignored.bru".into(),
                text: String::new(),
            },
            BrunoFile {
                path: "readme.md".into(),
                text: String::new(),
            },
        ],
    );
    let collection = imported.collection;
    assert_eq!(collection.name, "Bruno");
    let names: Vec<_> = collection.items.iter().map(Item::name).collect();
    assert_eq!(names, ["deep", "no-meta", "Nothing"]);
    let no_meta = request(&collection.items, "no-meta");
    assert_eq!(no_meta.method, HttpMethod::Patch);
    assert_eq!(no_meta.body, Body::Binary("/abs/data.bin".into()));
    let deep = folder(&folder(&collection.items, "deep").items, "er");
    assert_eq!(deep.items[0].name(), "Deep");
    assert_eq!(imported.environments.len(), 0);
    assert_eq!(
        imported.skipped,
        [skipped("Deep", SkippedKind::Body("sparql".into()))]
    );
}

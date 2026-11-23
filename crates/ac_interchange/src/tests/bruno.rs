use std::path::{Path, PathBuf};

use ac_domain::auth::{Auth, KeyPlacement};
use ac_domain::collection::{Body, HeaderEntry, MultipartField, Request};
use ac_domain::params::{PathParam, QueryParam};
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
    assert_eq!(get.url, "{{baseUrl}}/products/:id?verbose=true");
    assert_eq!(
        get.path_params,
        [PathParam {
            name: "id".into(),
            value: "42".into()
        }]
    );
    assert_eq!(
        get.disabled_query,
        [QueryParam {
            name: "debug".into(),
            value: "1".into()
        }]
    );
    assert_eq!(get.variables, []);
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
            ("Products (GraphQL)", SkippedKind::Body("graphql".into())),
            ("Products (GraphQL)", SkippedKind::Body("graphql".into())),
            ("Search", SkippedKind::Auth("oauth2".into())),
        ]
    );
}

#[test]
fn scripts_and_assertions_are_taken_over() {
    let imported = shop();
    let login = request(
        &folder(&imported.collection.items, "Authentication").items,
        "Login",
    );
    assert_eq!(login.scripts.pre_request, "");
    assert_eq!(
        login.scripts.post_response,
        "bru.setEnvVar(\"token\", res.body.token);"
    );

    let root = Path::new("/bruno");
    let imported = from_files(
        root,
        &[
            BrunoFile {
                path: "collection.bru".into(),
                text: "script:pre-request {
  req.setHeader('X-Team', 'a');
}
"
                .into(),
            },
            BrunoFile {
                path: "check.bru".into(),
                text: "get {
  url: x
}
script:pre-request {
  log(1);
}
script:post-response {
  log(2);
}
tests {
  test('t', () => {});
}
assert {
  res.status: eq 200
  res.status: between 1 2
}
"
                .into(),
            },
        ],
    );
    assert_eq!(
        imported.collection.scripts.pre_request,
        "req.setHeader('X-Team', 'a');"
    );
    let check = request(&imported.collection.items, "check");
    assert_eq!(check.scripts.pre_request, "log(1);");
    assert_eq!(
        check.scripts.post_response,
        "log(2);

test('t', () => {});

test(\"res.status: eq 200\", () => expect(res.getStatus()).to.eql(200));"
    );
    assert_eq!(
        imported.skipped,
        [Skipped {
            item: "check".into(),
            what: SkippedKind::Assertion("res.status: between 1 2".into()),
        }]
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

#[test]
fn digest_and_aws_signature_are_imported() {
    let imported = from_files(
        Path::new("/bruno"),
        &[
            BrunoFile {
                path: "digest.bru".into(),
                text: "meta {\n  name: Digest\n}\nget {\n  url: x\n  auth: digest\n}\nauth:digest {\n  username: bob\n  password: {{password}}\n}".into(),
            },
            BrunoFile {
                path: "aws.bru".into(),
                text: "meta {\n  name: Aws\n}\nget {\n  url: x\n  auth: awsv4\n}\nauth:awsv4 {\n  accessKeyId: AKID\n  secretAccessKey: {{awsSecret}}\n  sessionToken:\n  service: execute-api\n  region: eu-central-1\n  profileName:\n}".into(),
            },
        ],
    );
    let auth = |name: &str| request(&imported.collection.items, name).auth.clone();
    assert_eq!(
        auth("Digest"),
        Auth::Digest {
            username: "bob".into(),
            password: "{{password}}".into(),
        }
    );
    assert_eq!(
        auth("Aws"),
        Auth::AwsSigV4(ac_domain::auth::AwsSigV4 {
            access_key_id: "AKID".into(),
            secret_access_key: "{{awsSecret}}".into(),
            session_token: String::new(),
            region: "eu-central-1".into(),
            service: "execute-api".into(),
        })
    );
    assert_eq!(imported.skipped.len(), 0);
}

#[test]
fn oauth2_client_credentials_are_imported() {
    let imported = from_files(
        Path::new("/bruno"),
        &[BrunoFile {
            path: "token.bru".into(),
            text: "meta {\n  name: Token\n}\nget {\n  url: x\n  auth: oauth2\n}\nauth:oauth2 {\n  grant_type: client_credentials\n  access_token_url: {{auth}}/token\n  client_id: app\n  client_secret: {{secret}}\n  scope: read\n}".into(),
        }],
    );
    let Item::Request(request) = &imported.collection.items[0] else {
        panic!("expected a request");
    };
    assert_eq!(
        request.auth,
        Auth::OAuth2(ac_domain::auth::OAuth2 {
            token_url: "{{auth}}/token".into(),
            client_id: "app".into(),
            client_secret: "{{secret}}".into(),
            scope: "read".into(),
            ..Default::default()
        })
    );
    assert_eq!(imported.skipped.len(), 0);
}

#[test]
fn oauth2_authorization_code_is_imported() {
    let imported = from_files(
        Path::new("/bruno"),
        &[BrunoFile {
            path: "me.bru".into(),
            text: "meta {\n  name: Me\n}\nget {\n  url: x\n  auth: oauth2\n}\nauth:oauth2 {\n  grant_type: authorization_code\n  callback_url: http://127.0.0.1:9000/cb\n  authorization_url: https://auth.example.com/authorize\n  access_token_url: https://auth.example.com/token\n  client_id: app\n  client_secret:\n  scope: openid\n  pkce: true\n}".into(),
        }],
    );
    let Item::Request(request) = &imported.collection.items[0] else {
        panic!("expected a request");
    };
    assert_eq!(
        request.auth,
        Auth::OAuth2(ac_domain::auth::OAuth2 {
            grant: ac_domain::auth::OAuth2Grant::AuthorizationCode,
            token_url: "https://auth.example.com/token".into(),
            auth_url: "https://auth.example.com/authorize".into(),
            redirect_url: "http://127.0.0.1:9000/cb".into(),
            client_id: "app".into(),
            scope: "openid".into(),
            ..Default::default()
        })
    );
    assert_eq!(imported.skipped.len(), 0);
}

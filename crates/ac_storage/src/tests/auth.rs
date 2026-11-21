use toml_edit::DocumentMut;

use super::*;
use crate::test_support::{read as read_file, shop_api};

fn parse(text: &str) -> Result<Auth, StorageError> {
    let doc: DocumentMut = text.parse().unwrap();
    read(doc.get(KEY), Path::new("x.toml"), KEY)
}

fn round_trip(auth: &Auth) -> Auth {
    let mut doc = DocumentMut::new();
    write(doc.as_table_mut(), &Auth::Inherit, auth);
    parse(&doc.to_string()).unwrap()
}

#[test]
fn every_kind_round_trips() {
    for auth in [
        Auth::Inherit,
        Auth::None,
        Auth::Basic {
            username: "alice".into(),
            password: "{{password}}".into(),
        },
        Auth::Bearer {
            token: "{{token}}".into(),
        },
        Auth::ApiKey {
            name: "X-API-Key".into(),
            value: "k".into(),
            placement: KeyPlacement::Header,
        },
        Auth::ApiKey {
            name: "key".into(),
            value: "k".into(),
            placement: KeyPlacement::Query,
        },
        Auth::OAuth2(OAuth2 {
            token_url: "https://auth.example.com/token".into(),
            client_id: "app".into(),
            client_secret: "{{secret}}".into(),
            scope: "read write".into(),
        }),
        Auth::OAuth2(OAuth2 {
            token_url: "t".into(),
            ..OAuth2::default()
        }),
    ] {
        assert_eq!(round_trip(&auth), auth);
    }
}

#[test]
fn missing_values_get_defaults_and_wrong_ones_are_reported() {
    assert_eq!(parse("").unwrap(), Auth::Inherit);
    assert_eq!(
        parse("[auth]\ntype = \"api-key\"\nname = \"k\"").unwrap(),
        Auth::ApiKey {
            name: "k".into(),
            value: String::new(),
            placement: KeyPlacement::Header,
        }
    );
    for text in [
        "auth = 1",
        "[auth]\ntype = \"oauth\"",
        "[auth]\n",
        "[auth]\ntype = \"api-key\"\nin = \"cookie\"",
    ] {
        assert!(parse(text).is_err(), "{text}");
    }
}

#[test]
fn unchanged_auth_keeps_the_file_and_folders_and_collections_store_it() {
    let (_temp, root) = shop_api();
    let path = root.join("health.toml");
    let (mut request, _) = crate::request_file::read_request(&path).unwrap();
    assert_eq!(request.auth, Auth::Inherit);
    request.auth = Auth::Bearer {
        token: "{{token}}".into(),
    };
    crate::request_file::save_request(&path, &request).unwrap();
    let saved = read_file(&path);
    assert!(
        saved.contains("[request.auth]\ntype = \"bearer\""),
        "{saved}"
    );
    crate::request_file::save_request(&path, &request).unwrap();
    assert_eq!(read_file(&path), saved);

    let defaults = ac_domain::collection::RequestDefaults {
        certificate_check: ac_domain::tls::CertificateCheck::Inherit,
        variables: Vec::new(),
        auth: Auth::None,
    };
    crate::settings::save_collection_defaults(&root, &defaults).unwrap();
    let loaded = crate::load_collection(&root).unwrap();
    assert_eq!(loaded.collection.auth, Auth::None);
}

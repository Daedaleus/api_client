use ac_domain::collection::ItemId;

use super::*;
use crate::test_support::{read, shop_api};

#[test]
fn saving_unchanged_requests_keeps_the_files_byte_for_byte() {
    let (_temp, root) = shop_api();
    for file in [
        "products/list-products.toml",
        "auth/login.toml",
        "products/get-product.toml",
    ] {
        let path = root.join(file);
        let before = read(&path);
        let (request, _) = read_request(&path).unwrap();
        save_request(&path, &request).unwrap();
        assert_eq!(read(&path), before, "{file}");
    }
}

#[test]
fn changing_a_value_keeps_comments_order_and_unknown_keys() {
    let (_temp, root) = shop_api();
    let path = root.join("products/list-products.toml");
    let before = read(&path);
    let (mut request, _) = read_request(&path).unwrap();
    request.url = "{{baseUrl}}/products?limit=50".into();
    save_request(&path, &request).unwrap();

    let expected = before.replace("limit=20", "limit=50");
    assert_eq!(read(&path), expected);
}

#[test]
fn headers_and_body_are_rewritten_only_when_changed() {
    let (_temp, root) = shop_api();
    let path = root.join("products/get-product.toml");
    let (mut request, _) = read_request(&path).unwrap();
    request.headers = vec![HeaderEntry {
        name: "Accept".into(),
        value: "text/plain".into(),
        enabled: true,
    }];
    request.body = Body::Text("line one\nline two\n".into());
    save_request(&path, &request).unwrap();

    let text = read(&path);
    assert!(text.contains("[[request.headers]]"), "{text}");
    assert!(
        text.contains("content = '''\nline one\nline two\n'''"),
        "{text}"
    );
    let (reread, _) = read_request(&path).unwrap();
    assert_eq!(reread, request);

    request.body = Body::None;
    request.headers.clear();
    save_request(&path, &request).unwrap();
    let text = read(&path);
    assert!(
        !text.contains("body") && !text.contains("headers"),
        "{text}"
    );
    assert_eq!(read_request(&path).unwrap().0, request);
}

#[test]
fn awkward_body_text_round_trips() {
    let (_temp, root) = shop_api();
    let path = root.join("health.toml");
    let (mut request, _) = read_request(&path).unwrap();
    for content in [
        "has ''' inside\nand lines",
        "single line",
        "tab\there\n",
        "ends with ''",
    ] {
        request.body = Body::Json(content.into());
        save_request(&path, &request).unwrap();
        assert_eq!(
            read_request(&path).unwrap().0.body,
            request.body,
            "{content:?}"
        );
    }
}

#[test]
fn files_without_id_get_one_on_save() {
    let (_temp, root) = shop_api();
    let path = root.join("health.toml");
    let (request, _) = read_request(&path).unwrap();
    save_request(&path, &request).unwrap();
    assert!(read(&path).contains(&format!("id = \"{}\"", request.id)));
    assert_eq!(read_request(&path).unwrap().0.id, request.id);
}

#[test]
fn new_files_are_written_from_scratch() {
    let (_temp, root) = shop_api();
    let path = root.join("new.toml");
    let mut request = Request::new("New");
    request.method = HttpMethod::Custom("PROPFIND".into());
    request.url = "https://example.com".into();
    save_request(&path, &request).unwrap();

    let text = read(&path);
    assert!(
        text.starts_with("format = \"api-client/request\"\nversion = 1\n"),
        "{text}"
    );
    assert_eq!(read_request(&path).unwrap().0, request);
}

#[test]
fn newer_and_damaged_files_are_never_written() {
    let (_temp, root) = shop_api();
    for (file, check) in [
        ("products/newer.toml", "newer version"),
        ("products/broken.toml", "not valid TOML"),
        ("products/wrong.toml", "expected format"),
    ] {
        let path = root.join(file);
        let before = read(&path);
        let error = save_request(&path, &Request::new("x")).unwrap_err();
        assert!(error.to_string().contains(check), "{file}: {error}");
        assert_eq!(read(&path), before, "{file} was changed");
    }
}

#[test]
fn invalid_values_are_reported_with_their_key() {
    let (_temp, root) = shop_api();
    let path = root.join("bad.toml");
    let header = "format = \"api-client/request\"\nversion = 1\n";
    for (body, key) in [
        ("id = \"nope\"\n", "id"),
        ("name = 3\n", "name"),
        ("request = 1\n", "request"),
        ("[request]\nmethod = \"GE T\"\n", "request.method"),
        ("[request]\nheaders = 1\n", "request.headers"),
        ("[request]\nheaders = [1]\n", "request.headers"),
        ("[[request.headers]]\nvalue = \"x\"\n", "name"),
        (
            "[[request.headers]]\nname = \"a\"\nenabled = \"yes\"\n",
            "enabled",
        ),
        ("[request]\nbody = 1\n", "request.body"),
        ("[request.body]\ntype = \"xml\"\n", "request.body.type"),
    ] {
        std::fs::write(&path, format!("{header}{body}")).unwrap();
        let error = read_request(&path).unwrap_err();
        assert!(
            matches!(&error, StorageError::Invalid { field, .. } if *field == key),
            "{body:?}: {error}"
        );
    }
    std::fs::write(&path, "format = \"api-client/request\"\nversion = 0\n").unwrap();
    let error = read_request(&path).unwrap_err();
    assert!(error.to_string().contains("version"), "{error}");
}

#[test]
fn missing_files_are_io_errors() {
    let (_temp, root) = shop_api();
    let error = read_request(&root.join("nothing.toml")).unwrap_err();
    assert!(matches!(error, StorageError::Io { .. }), "{error}");
    let error =
        save_request(&root.join("no-folder").join("x.toml"), &Request::new("x")).unwrap_err();
    assert!(matches!(error, StorageError::Io { .. }), "{error}");
}

#[test]
fn ids_survive_a_round_trip() {
    let (_temp, root) = shop_api();
    let path = root.join("id.toml");
    let mut request = Request::new("Id");
    request.id = "0193a8f2-7c1e-7b40-9d3a-5f2c8e1b4a99"
        .parse::<ItemId>()
        .unwrap();
    save_request(&path, &request).unwrap();
    assert_eq!(read_request(&path).unwrap().0.id, request.id);
}

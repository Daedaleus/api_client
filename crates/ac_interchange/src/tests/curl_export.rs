use std::path::PathBuf;

use ac_domain::exchange::{Header, Part};
use url::Url;

use super::*;

fn request(method: HttpMethod, body: Option<RequestBody>) -> ResolvedRequest {
    ResolvedRequest {
        method,
        url: Url::parse("https://api.example.com/items?q=a b").unwrap(),
        headers: vec![Header::new("Authorization", "Bearer it's")],
        body,
        verify_tls: true,
    }
}

#[test]
fn a_get_needs_no_method_and_quotes_survive() {
    assert_eq!(
        to_curl(&request(HttpMethod::Get, None)),
        "curl \\\n  'https://api.example.com/items?q=a%20b' \\\n  --header 'Authorization: Bearer it'\\''s'"
    );
}

#[test]
fn bodies_and_unusual_methods_are_written() {
    let json = request(
        HttpMethod::Put,
        Some(RequestBody::Bytes(b"{\"a\":1}".to_vec())),
    );
    let text = to_curl(&json);
    assert!(text.starts_with("curl \\\n  --request 'PUT'"), "{text}");
    assert!(text.ends_with("--data-raw '{\"a\":1}'"), "{text}");

    let post = request(HttpMethod::Post, Some(RequestBody::Bytes(b"x".to_vec())));
    assert!(!to_curl(&post).contains("--request"));
    let get_with_body = request(HttpMethod::Get, Some(RequestBody::Bytes(b"x".to_vec())));
    assert!(to_curl(&get_with_body).contains("--request 'GET'"));

    let mut file = request(
        HttpMethod::Post,
        Some(RequestBody::File(PathBuf::from("/data/payload.bin"))),
    );
    file.verify_tls = false;
    let text = to_curl(&file);
    assert!(
        text.contains("--data-binary '@/data/payload.bin'"),
        "{text}"
    );
    assert!(text.ends_with("--insecure"), "{text}");

    let form = request(
        HttpMethod::Post,
        Some(RequestBody::Multipart(vec![
            Part {
                name: "title".into(),
                content: PartContent::Text("Tea".into()),
            },
            Part {
                name: "avatar".into(),
                content: PartContent::File(PathBuf::from("/files/a.png")),
            },
        ])),
    );
    let text = to_curl(&form);
    assert!(text.contains("--form 'title=Tea'"), "{text}");
    assert!(text.contains("--form 'avatar=@/files/a.png'"), "{text}");
}

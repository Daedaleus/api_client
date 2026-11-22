use std::time::Duration;

use ac_domain::exchange::{Header, RequestBody};
use ac_domain::protocol::{HttpMethod, StatusCode};

use super::*;

fn header(name: &str, value: &str) -> Header {
    Header {
        name: name.into(),
        value: value.into(),
    }
}

fn sent(url: &str, body: &str) -> ResolvedRequest {
    ResolvedRequest {
        method: HttpMethod::Post,
        url: url.parse().unwrap(),
        headers: vec![
            header("Authorization", "Bearer s3cret-token"),
            header("X-Api-Key", "k-123"),
            header("X-Trace", "has s3cret-token inside"),
        ],
        body: Some(RequestBody::Bytes(body.as_bytes().to_vec())),
        verify_tls: true,
    }
}

fn response(body: &[u8]) -> Response {
    Response {
        status: StatusCode::new(201).unwrap(),
        headers: vec![header("Set-Cookie", "id=1"), header("Cookie", "a=b")],
        body: body.to_vec(),
        elapsed: Duration::from_millis(42),
    }
}

#[test]
fn secrets_and_credentials_are_masked() {
    let masking = Masking::new(
        [
            "s3cret-token".to_owned(),
            String::new(),
            "s3cret".to_owned(),
        ],
        Some("x-api-key".to_owned()),
    );
    let entry = HistoryEntry::new(
        Some("Shop".into()),
        &sent(
            "https://api.example.com/items?x-api-key=k-123&q=s3cret-token",
            r#"{"password":"s3cret"}"#,
        ),
        &Ok(response(br#"{"token":"s3cret-token"}"#)),
        &masking,
    );
    assert_eq!(entry.collection.as_deref(), Some("Shop"));
    assert_eq!(entry.request.method, "POST");
    let url = &entry.request.url;
    assert!(
        url.starts_with("https://api.example.com/items?x-api-key=%E2%80%A2"),
        "{url}"
    );
    assert!(!url.contains("k-123") && !url.contains("s3cret"), "{url}");
    assert_eq!(
        entry.request.headers,
        [
            ("Authorization".to_owned(), MASK.to_owned()),
            ("X-Api-Key".to_owned(), MASK.to_owned()),
            ("X-Trace".to_owned(), format!("has {MASK} inside")),
        ]
    );
    assert_eq!(entry.request.body.as_deref(), Some(r#"{"password":"•••"}"#));
    let HistoryOutcome::Response {
        status,
        elapsed_ms,
        headers,
        body,
        size_bytes,
    } = &entry.outcome
    else {
        panic!("expected a response");
    };
    assert_eq!((*status, *elapsed_ms, *size_bytes), (201, 42, 24));
    assert_eq!(body.as_deref(), Some(r#"{"token":"•••"}"#));
    assert_eq!(headers[0].1, "id=1");
    assert_eq!(headers[1].1, MASK);
    assert_eq!(entry.error_text(), None);
}

#[test]
fn long_and_binary_bodies() {
    let long = "ä".repeat(MAX_BODY);
    let entry = HistoryEntry::new(
        None,
        &sent("https://example.com/", &long),
        &Ok(response(&[0xff, 0xfe, 0x00])),
        &Masking::default(),
    );
    let body = entry.request.body.unwrap();
    assert!(body.len() <= MAX_BODY && body.len() > MAX_BODY - 4);
    assert!(body.chars().all(|c| c == 'ä'));
    assert!(matches!(
        entry.outcome,
        HistoryOutcome::Response {
            body: None,
            size_bytes: 3,
            ..
        }
    ));
}

#[test]
fn failures_keep_their_text_and_unknown_keys_still_show() {
    let error = UserText::new(keys::REQUEST).with("detail", "refused by s3cret host");
    let entry = HistoryEntry::new(
        None,
        &sent("https://example.com/", ""),
        &Err(error),
        &Masking::new(["s3cret".to_owned()], None),
    );
    let text = entry.error_text().unwrap();
    assert_eq!(text.key, keys::REQUEST);
    assert_eq!(text.args["detail"], "refused by ••• host");

    let mut old = entry.clone();
    old.outcome = HistoryOutcome::Failed {
        key: "error.gone_by_now".into(),
        args: BTreeMap::new(),
    };
    let text = old.error_text().unwrap();
    assert_eq!(text.key, keys::REQUEST);
    assert_eq!(text.args["detail"], "error.gone_by_now");
    assert_ne!(HistoryId::new(), HistoryId::default());
}

#[test]
fn entries_round_trip_as_json() {
    let entry = HistoryEntry::new(
        None,
        &sent("https://example.com/", "x"),
        &Ok(response(b"ok")),
        &Masking::default(),
    );
    let text = serde_json::to_string(&entry).unwrap();
    assert!(text.contains("\"atMs\":0"), "{text}");
    assert_eq!(serde_json::from_str::<HistoryEntry>(&text).unwrap(), entry);
}

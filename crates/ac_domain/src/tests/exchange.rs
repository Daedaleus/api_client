use std::time::Duration;

use super::*;
use crate::protocol::header;

fn response(headers: Vec<Header>, body: &[u8]) -> Response {
    Response {
        status: StatusCode::new(200).unwrap(),
        headers,
        body: body.to_vec(),
        elapsed: Duration::from_millis(5),
    }
}

#[test]
fn size_is_the_body_length() {
    assert_eq!(response(vec![], b"hello").size(), 5);
    assert_eq!(response(vec![], b"").size(), 0);
}

#[test]
fn header_lookup_ignores_case_and_takes_the_first() {
    let response = response(
        vec![
            Header::new("Content-Type", "application/json"),
            Header::new("content-type", "text/plain"),
        ],
        b"",
    );
    assert_eq!(
        response.header(header::CONTENT_TYPE),
        Some("application/json")
    );
    assert_eq!(response.header("CONTENT-TYPE"), Some("application/json"));
    assert_eq!(response.header("x-missing"), None);
}

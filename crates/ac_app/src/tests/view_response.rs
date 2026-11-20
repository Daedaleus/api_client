use std::time::Duration;

use ac_domain::exchange::Header;
use ac_domain::protocol::StatusCode;

use super::*;
use crate::testing::response;

#[test]
fn response_view_has_status_timing_size_headers_and_body() {
    let view = ResponseView::from_response(&response(404, "application/json", r#"{"a":1}"#));
    assert_eq!(view.status, 404);
    assert_eq!(view.reason, Some("Not Found"));
    assert_eq!(view.status_class, StatusClassView::ClientError);
    assert_eq!(view.elapsed_ms, 7);
    assert_eq!(view.size_bytes, 7);
    assert_eq!(view.headers[0].name, "Content-Type");
    assert!(view.body.pretty);
}

#[test]
fn every_status_class_has_a_name() {
    let names: Vec<_> = [101, 200, 302, 400, 500, 600]
        .into_iter()
        .map(|code| {
            let class = StatusClassView::from(StatusCode::new(code).unwrap().class());
            serde_json::to_value(class).unwrap()
        })
        .collect();
    assert_eq!(
        names,
        [
            "informational",
            "success",
            "redirection",
            "clientError",
            "serverError",
            "nonstandard"
        ]
    );
}

#[test]
fn outcome_json_matches_the_ui_contract() {
    let completed = RunOutcomeView::Completed(ResponseView::from_response(&Response {
        status: StatusCode::new(200).unwrap(),
        headers: vec![Header::new("X-Id", "1")],
        body: b"ok".to_vec(),
        elapsed: Duration::from_millis(12),
    }));
    assert_eq!(
        serde_json::to_value(&completed).unwrap(),
        serde_json::json!({
            "kind": "completed",
            "status": 200,
            "reason": "OK",
            "statusClass": "success",
            "elapsedMs": 12,
            "sizeBytes": 2,
            "headers": [{ "name": "X-Id", "value": "1" }],
            "body": { "text": "ok", "pretty": false, "truncated": false, "binary": false }
        })
    );

    let failed = RunOutcomeView::Failed {
        error: UserText::new("error.timeout"),
    };
    assert_eq!(
        serde_json::to_value(&failed).unwrap(),
        serde_json::json!({ "kind": "failed", "error": { "key": "error.timeout", "args": {} } })
    );
}

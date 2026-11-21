use ac_app::UserText;

use super::*;

fn report() -> RunReport {
    RunReport {
        results: vec![
            RunResult {
                path: vec!["Auth".into(), "Login".into()],
                method: "POST".into(),
                outcome: RunResultOutcome::Response {
                    status: 201,
                    elapsed_ms: 1234,
                },
            },
            RunResult {
                path: vec!["Health <&>".into()],
                method: "GET".into(),
                outcome: RunResultOutcome::Failed {
                    error: UserText::new("error.timeout"),
                },
            },
            RunResult {
                path: vec!["Broken".into()],
                method: "GET".into(),
                outcome: RunResultOutcome::Response {
                    status: 500,
                    elapsed_ms: 5,
                },
            },
        ],
    }
}

#[test]
fn text_reports_list_each_request_and_sum_up() {
    let texts = Texts::new("en");
    let out = text(&report(), "Shop", Some("Dev"), true, &texts);
    let lines: Vec<_> = out.lines().collect();
    assert_eq!(lines[0], "Shop · environment Dev");
    assert_eq!(lines[1], "  ✓ 201 POST  Auth / Login  (1234 ms)");
    assert!(
        lines[2].starts_with("  ✗ --- GET   Health <&>  – "),
        "{}",
        lines[2]
    );
    assert_eq!(lines[3], "  ✗ 500 GET   Broken  (5 ms)");
    assert_eq!(lines[4], "3 requests: 1 passed, 2 failed");

    let lenient = text(&report(), "Shop", None, false, &texts);
    assert!(lenient.starts_with("Shop\n"));
    assert!(lenient.contains("  ✓ 500 GET"));
    assert!(lenient.ends_with("3 requests: 2 passed, 1 failed\n"));
}

#[test]
fn json_reports_mark_what_passed() {
    let value: serde_json::Value = serde_json::from_str(&json(&report(), true)).unwrap();
    let passed: Vec<_> = value["results"]
        .as_array()
        .unwrap()
        .iter()
        .map(|result| result["passed"].as_bool().unwrap())
        .collect();
    assert_eq!(passed, [true, false, false]);
    assert_eq!(value["results"][0]["outcome"]["status"], 201);
}

#[test]
fn junit_reports_are_escaped_xml() {
    let out = junit(&report(), "Shop & Co", true, &Texts::new("en"));
    assert!(out.starts_with("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n"));
    assert!(
        out.contains(
            "<testsuite name=\"Shop &amp; Co\" tests=\"3\" failures=\"2\" time=\"1.239\">"
        )
    );
    assert!(out.contains("<testcase classname=\"Auth\" name=\"POST Login\" time=\"1.234\"/>"));
    assert!(out.contains(
        "<testcase classname=\"Shop &amp; Co\" name=\"GET Health &lt;&amp;&gt;\" time=\"0.000\"><failure message=\""
    ));
    assert!(out.contains("<failure message=\"HTTP 500\"/>"));
    assert!(out.ends_with("</testsuite></testsuites>\n"));
    assert_eq!(escape("\"q\""), "&quot;q&quot;");
}

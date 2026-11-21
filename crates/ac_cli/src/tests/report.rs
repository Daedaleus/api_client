use ac_app::UserText;
use ac_app::view::TestResultView;

use super::*;

fn test(name: &str, message: Option<&str>) -> TestResultView {
    TestResultView {
        name: name.into(),
        passed: message.is_none(),
        message: message.map(Into::into),
    }
}

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
                tests: vec![test("status is 201", None)],
            },
            RunResult {
                path: vec!["Health <&>".into()],
                method: "GET".into(),
                outcome: RunResultOutcome::Failed {
                    error: UserText::new("error.timeout"),
                },
                tests: Vec::new(),
            },
            RunResult {
                path: vec!["Broken".into()],
                method: "GET".into(),
                outcome: RunResultOutcome::Response {
                    status: 500,
                    elapsed_ms: 5,
                },
                tests: Vec::new(),
            },
            RunResult {
                path: vec!["Items".into()],
                method: "GET".into(),
                outcome: RunResultOutcome::Response {
                    status: 200,
                    elapsed_ms: 7,
                },
                tests: vec![
                    test("has items", None),
                    test("first is <cheap>", Some("expected 3 to be less than 2")),
                ],
            },
        ],
    }
}

#[test]
fn text_reports_list_each_request_its_tests_and_sum_up() {
    let texts = Texts::new("en");
    let out = text(&report(), "Shop", Some("Dev"), true, &texts);
    let lines: Vec<_> = out.lines().collect();
    assert_eq!(lines[0], "Shop · environment Dev");
    assert_eq!(lines[1], "  ✓ 201 POST  Auth / Login  (1234 ms)");
    assert_eq!(lines[2], "      ✓ status is 201");
    assert!(
        lines[3].starts_with("  ✗ --- GET   Health <&>  – "),
        "{}",
        lines[3]
    );
    assert_eq!(lines[4], "  ✗ 500 GET   Broken  (5 ms)");
    // A request whose test failed fails, whatever its status.
    assert_eq!(lines[5], "  ✗ 200 GET   Items  (7 ms)");
    assert_eq!(lines[6], "      ✓ has items");
    assert_eq!(
        lines[7],
        "      ✗ first is <cheap>  – expected 3 to be less than 2"
    );
    assert_eq!(lines[8], "4 requests: 1 passed, 3 failed");
    assert_eq!(lines[9], "3 tests: 2 passed, 1 failed");

    let lenient = text(&report(), "Shop", None, false, &texts);
    assert!(lenient.starts_with("Shop\n"));
    assert!(lenient.contains("  ✓ 500 GET"));
    assert!(lenient.contains("4 requests: 2 passed, 2 failed\n"));

    let without_tests = RunReport {
        results: report().results[1..3].to_vec(),
    };
    assert!(!text(&without_tests, "Shop", None, true, &texts).contains("tests:"));
}

#[test]
fn json_reports_mark_what_passed_with_the_tests() {
    let value: serde_json::Value = serde_json::from_str(&json(&report(), true)).unwrap();
    let passed: Vec<_> = value["results"]
        .as_array()
        .unwrap()
        .iter()
        .map(|result| result["passed"].as_bool().unwrap())
        .collect();
    assert_eq!(passed, [true, false, false, false]);
    assert_eq!(value["results"][0]["outcome"]["status"], 201);
    assert_eq!(
        value["results"][3]["tests"][1],
        serde_json::json!({
            "name": "first is <cheap>",
            "passed": false,
            "message": "expected 3 to be less than 2"
        })
    );
}

#[test]
fn junit_reports_requests_and_tests_as_escaped_xml() {
    let out = junit(&report(), "Shop & Co", true, &Texts::new("en"));
    assert!(out.starts_with("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n"));
    assert!(
        out.contains(
            "<testsuite name=\"Shop &amp; Co\" tests=\"7\" failures=\"3\" time=\"1.246\">"
        ),
        "{out}"
    );
    assert!(out.contains("<testcase classname=\"Auth\" name=\"POST Login\" time=\"1.234\"/>"));
    assert!(
        out.contains("<testcase classname=\"Auth.Login\" name=\"status is 201\" time=\"0.000\"/>")
    );
    assert!(out.contains(
        "<testcase classname=\"Shop &amp; Co\" name=\"GET Health &lt;&amp;&gt;\" time=\"0.000\"><failure message=\""
    ));
    assert!(out.contains("<failure message=\"HTTP 500\"/>"));
    assert!(
        out.contains("<testcase classname=\"Shop &amp; Co\" name=\"GET Items\" time=\"0.007\"/>")
    );
    assert!(out.contains(
        "<testcase classname=\"Items\" name=\"first is &lt;cheap&gt;\" time=\"0.000\"><failure message=\"expected 3 to be less than 2\"/></testcase>"
    ));
    assert!(out.ends_with("</testsuite></testsuites>\n"));
    assert_eq!(escape("\"q\""), "&quot;q&quot;");
}

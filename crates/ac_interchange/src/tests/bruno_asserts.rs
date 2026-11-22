use super::*;
use crate::bruno::bru::blocks;

fn translated(lines: &str) -> (Vec<String>, Vec<Skipped>) {
    let text = format!("assert {{\n{lines}\n}}\n");
    let block = blocks(&text).remove(0);
    let mut report = Vec::new();
    (tests(&block, "Login", &mut report), report)
}

#[test]
fn assertions_become_tests() {
    let (tests, report) = translated(
        "  res.status: eq 200\n  res.body.items: length 2\n  res.body.token: isString\n  res.headers.content-type: contains json\n  res.responseTime: lt 500\n  ~res.body.off: eq 1\n  res.body.items[0].id: neq 'x'",
    );
    assert_eq!(report, []);
    assert_eq!(
        tests,
        [
            r#"test("res.status: eq 200", () => expect(res.getStatus()).to.eql(200));"#,
            r#"test("res.body.items: length 2", () => expect(res.getBody().items).to.have.lengthOf(2));"#,
            r#"test("res.body.token: isString", () => expect(res.getBody().token).to.be.a('string'));"#,
            r#"test("res.headers.content-type: contains json", () => expect(res.getHeader("content-type")).to.include("json"));"#,
            r#"test("res.responseTime: lt 500", () => expect(res.getResponseTime()).to.be.below(500));"#,
            r#"test("res.body.items[0].id: neq 'x'", () => expect(res.getBody().items[0].id).not.to.eql("x"));"#,
        ]
    );
}

#[test]
fn what_has_no_counterpart_is_reported() {
    let (tests, report) = translated(
        "  res.status: between 200 299\n  req.url: eq x\n  res.body.a(): eq 1\n  res.bodyx: eq 1",
    );
    assert_eq!(tests, Vec::<String>::new());
    let what: Vec<_> = report.into_iter().map(|skipped| skipped.what).collect();
    assert_eq!(
        what,
        [
            SkippedKind::Assertion("res.status: between 200 299".into()),
            SkippedKind::Assertion("req.url: eq x".into()),
            SkippedKind::Assertion("res.body.a(): eq 1".into()),
            SkippedKind::Assertion("res.bodyx: eq 1".into()),
        ]
    );
}

#[test]
fn values_are_literals_never_code() {
    assert_eq!(literal("1.5"), "1.5");
    assert_eq!(literal("true"), "true");
    assert_eq!(literal("OK"), "\"OK\"");
    assert_eq!(literal("'a'); evil('"), "\"a'); evil(\"");
    assert_eq!(literal("\"quoted\""), "\"quoted\"");
}

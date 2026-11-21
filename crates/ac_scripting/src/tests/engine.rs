use std::time::Duration;

use ac_app::ports::ScriptResponse;
use ac_domain::script::ScriptPhase;

use super::*;

fn pair(name: &str, value: &str) -> (String, String) {
    (name.into(), value.into())
}

fn job(phase: ScriptPhase, source: &str) -> ScriptJob {
    ScriptJob {
        phase,
        source: source.into(),
        request: ScriptRequest {
            method: "GET".into(),
            url: "{{baseUrl}}/items".into(),
            headers: vec![pair("Accept", "application/json"), pair("X-Old", "1")],
            body: Some("{}".into()),
        },
        response: (phase == ScriptPhase::PostResponse).then(|| ScriptResponse {
            status: 201,
            headers: vec![pair("Content-Type", "application/json")],
            body: r#"{"token":"t0k","items":[1,2]}"#.into(),
            time_ms: 12,
        }),
        variables: vec![
            pair("baseUrl", "https://api.example.com"),
            pair("user", "alice"),
        ],
    }
}

fn run_default(job: &ScriptJob) -> Result<ScriptOutcome, ScriptError> {
    run(job, Limits::default())
}

#[test]
fn a_pre_request_script_changes_the_request_and_sets_variables() {
    let source = r"
        interface Item { id: number }
        const user: string = env.get('user') ?? 'nobody';
        req.method = 'POST';
        req.url = req.url + '?user=' + user;
        req.setHeader('accept', 'text/plain');
        req.setHeader('X-New', String(env.has('baseUrl')));
        req.removeHeader('x-old');
        req.body = JSON.stringify({ id: 1 } satisfies Item);
        env.set('stamp', 42);
        log('user', user, { n: 1 }, [1, 2]);
        console.info('via console');
    ";
    let outcome = run_default(&job(ScriptPhase::PreRequest, source)).unwrap();
    assert_eq!(
        outcome.request,
        ScriptRequest {
            method: "POST".into(),
            url: "{{baseUrl}}/items?user=alice".into(),
            headers: vec![pair("accept", "text/plain"), pair("X-New", "true")],
            body: Some(r#"{"id":1}"#.into()),
        }
    );
    assert_eq!(outcome.variables, [pair("stamp", "42")]);
    assert_eq!(outcome.logs, [r#"user alice {"n":1} [1,2]"#, "via console"]);
}

#[test]
fn a_post_response_script_reads_the_response() {
    let source = r"
        const body = res.json<{ token: string; items: number[] }>();
        env.set('token', body.token);
        log(res.status, res.timeMs, res.getHeader('content-type'), res.headers()['Content-Type'], res.text().length);
        log(env.get('token'), req.getHeader('ACCEPT'), Object.keys(req.headers()).length);
    ";
    let outcome = run_default(&job(ScriptPhase::PostResponse, source)).unwrap();
    assert_eq!(outcome.variables, [pair("token", "t0k")]);
    assert_eq!(
        outcome.logs,
        [
            "201 12 application/json application/json 29",
            "t0k application/json 2"
        ]
    );
}

#[test]
fn scripts_can_await_and_see_no_outside_world() {
    let source = r"
        const value = await Promise.resolve(7);
        log(value, typeof fetch, typeof require, typeof setTimeout, typeof res);
    ";
    let outcome = run_default(&job(ScriptPhase::PreRequest, source)).unwrap();
    assert_eq!(outcome.logs, ["7 undefined undefined undefined undefined"]);
}

#[test]
fn errors_say_what_went_wrong() {
    let failed = |source: &str| run_default(&job(ScriptPhase::PreRequest, source)).unwrap_err();
    assert_eq!(
        failed("throw new TypeError('bad input')"),
        ScriptError::Failed("TypeError: bad input".into())
    );
    assert_eq!(
        failed("throw 'plain text'"),
        ScriptError::Failed("plain text".into())
    );
    assert_eq!(
        failed("await Promise.reject(new Error('later'))"),
        ScriptError::Failed("Error: later".into())
    );
    assert!(matches!(
        failed("req = null"),
        ScriptError::Failed(message) if message.contains("TypeError")
    ));
    assert_eq!(
        failed("log(1)\nlet = ;"),
        ScriptError::Syntax {
            line: 2,
            message: "Unexpected token".into()
        }
    );
    let post = run_default(&job(ScriptPhase::PostResponse, "JSON.parse('{')"));
    assert!(
        matches!(post, Err(ScriptError::Failed(message)) if message.starts_with("SyntaxError"))
    );
}

#[test]
fn runaway_scripts_are_stopped() {
    let limits = Limits {
        time: Duration::from_millis(200),
        memory: 16 * 1024 * 1024,
    };
    let start = std::time::Instant::now();
    assert_eq!(
        run(&job(ScriptPhase::PreRequest, "while (true) {}"), limits).unwrap_err(),
        ScriptError::TimedOut { seconds: 0 }
    );
    assert!(start.elapsed() < Duration::from_secs(2));
    let backtracking = job(
        ScriptPhase::PreRequest,
        "/^(a+)+$/.test('a'.repeat(40) + 'b')",
    );
    assert_eq!(
        run(&backtracking, limits).unwrap_err(),
        ScriptError::TimedOut { seconds: 0 }
    );
    let hog = job(
        ScriptPhase::PreRequest,
        "const a = []; while (true) a.push(new Array(100000).fill(1));",
    );
    assert_eq!(
        run(&hog, limits).unwrap_err(),
        ScriptError::OutOfMemory { megabytes: 16 }
    );
    assert_eq!(Limits::default().time, Duration::from_secs(5));
}

#[test]
fn tests_record_each_check_and_go_on_after_a_failure() {
    let source = r"
        test('status is 201', () => expect(res.status).toBe(201));
        test('status is 200', () => { expect(res.status).toBe(200); });
        test('token present', () => {
            const body = res.json();
            expect(body).toHaveProperty('token', 't0k');
            expect(body.items).toEqual([1, 2]);
            expect(body.items).toHaveLength(2);
            expect(body.items).toContain(2);
            expect(res.getHeader('content-type')).toMatch(/json/);
            expect(res.text()).toMatch('token');
        });
        test('negations', () => {
            expect(res.status).not.toBe(500);
            expect(res.json().missing).toBeUndefined();
            expect(null).toBeNull();
            expect(0).toBeFalsy();
            expect('x').toBeTruthy();
            expect(1).toBeDefined();
            expect(res.timeMs).toBeLessThan(1000);
            expect(res.timeMs).toBeLessThanOrEqual(12);
            expect(res.timeMs).toBeGreaterThan(1);
            expect(res.timeMs).toBeGreaterThanOrEqual(12);
            expect({ a: { b: 1 } }).not.toEqual({ a: { b: 2 } });
            expect([1]).not.toEqual({ 0: 1 });
            expect({ a: 1 }).not.toHaveProperty(['a', 'b']);
        });
        test('async', async () => {
            const value = await Promise.resolve(3);
            expect(value).toBe(4);
        });
        test('thrown error', () => { throw new TypeError('boom'); });
        test('thrown text', () => { throw 'plain'; });
        test('bad matchers', () => expect(undefined).toContain(1));
        test('objects in messages', () => expect({ a: 1 }).toEqual([1]));
        log('after the tests');
    ";
    let outcome = run_default(&job(ScriptPhase::PostResponse, source)).unwrap();
    let summary: Vec<(String, bool, Option<String>)> = outcome
        .tests
        .into_iter()
        .map(|test| (test.name, test.passed, test.message))
        .collect();
    let failed = |name: &str, message: &str| (name.to_owned(), false, Some(message.to_owned()));
    let passed = |name: &str| (name.to_owned(), true, None);
    assert_eq!(
        summary,
        [
            passed("status is 201"),
            failed("status is 200", "expected 201 to be 200"),
            passed("token present"),
            passed("negations"),
            failed("async", "expected 3 to be 4"),
            failed("thrown error", "TypeError: boom"),
            failed("thrown text", "plain"),
            failed("bad matchers", "expected undefined to contain 1"),
            failed("objects in messages", "expected {\"a\":1} to equal [1]"),
        ]
    );
    assert_eq!(outcome.logs, ["after the tests"]);
}

#[test]
fn expect_outside_a_test_fails_the_script() {
    let error = run_default(&job(
        ScriptPhase::PostResponse,
        "expect(res.status).toBe(200)",
    ))
    .unwrap_err();
    assert_eq!(
        error,
        ScriptError::Failed("AssertionError: expected 201 to be 200".into())
    );
}

use std::path::PathBuf;
use std::sync::LazyLock;
use std::time::Duration;

use super::*;

/// One service for all tests (it takes about a second to start).
static ASSIST: LazyLock<TsAssist> = LazyLock::new(|| {
    TsAssist::new(|| {
        let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../apps/desktop/node_modules/typescript/lib");
        TypeScriptFiles::from_dir(&dir).map_err(|error| error.to_string())
    })
});

/// A full check of `source` (waits until the service has started).
async fn full_check(assist: &TsAssist, source: &str) -> Result<ScriptCheck, AssistError> {
    loop {
        let check = assist.check(source.into()).await?;
        if !check.partial {
            return Ok(check);
        }
        std::thread::sleep(Duration::from_millis(20));
    }
}

async fn messages(source: &str) -> Vec<(String, usize, usize)> {
    full_check(&ASSIST, source)
        .await
        .unwrap()
        .diagnostics
        .into_iter()
        .map(|diagnostic| (diagnostic.message, diagnostic.from, diagnostic.to))
        .collect()
}

async fn labels(source: &str) -> Vec<String> {
    let offset = source.find('|').unwrap();
    ASSIST
        .complete(source.replace('|', ""), offset)
        .await
        .unwrap()
        .into_iter()
        .map(|completion| completion.label)
        .collect()
}

#[tokio::test]
async fn checks_find_unknown_names_members_and_wrong_types() {
    let found = messages("req.methd;").await;
    assert_eq!(found.len(), 1, "{found:?}");
    assert!(
        found[0].0.contains("Property 'methd' does not exist"),
        "{found:?}"
    );
    assert_eq!((found[0].1, found[0].2), (4, 9));

    let typed =
        messages("env.set('a', {});\nconst body = res.json<{ items: string[] }>();\nbody.itmes;")
            .await;
    let texts: Vec<&str> = typed.iter().map(|(message, ..)| message.as_str()).collect();
    assert!(
        texts
            .iter()
            .any(|m| m.contains("not assignable to parameter")),
        "{texts:?}"
    );
    assert!(
        texts
            .iter()
            .any(|m| m.contains("Property 'itmes' does not exist")),
        "{texts:?}"
    );

    let outside = messages("fetch('x'); require('fs'); setTimeout(() => {}, 1);").await;
    assert_eq!(outside.len(), 3, "{outside:?}");

    let syntax = messages("const x = ;").await;
    assert!(syntax[0].0.contains("Expression expected"), "{syntax:?}");
}

#[tokio::test]
async fn correct_scripts_pass_also_with_top_level_await_and_tests() {
    let script = "
        const value = await Promise.resolve(1);
        req.setHeader('X-Value', String(value));
        test('status', () => expect(res.status).not.toBe(500));
        const items: number[] = res.json<{ items: number[] }>().items;
        log(items.at(-1), env.get('token') ?? 'none');
    ";
    assert_eq!(messages(script).await, []);
}

#[tokio::test]
async fn completions_offer_the_api_and_what_types_say() {
    let members = labels("req.|").await;
    for wanted in ["setHeader", "getHeader", "method", "url", "body"] {
        assert!(
            members.iter().any(|label| label == wanted),
            "{wanted}: {members:?}"
        );
    }
    let derived = labels("res.json<{ items: string[]; total: number }>().|").await;
    assert_eq!(derived, ["items", "total"]);
    let matchers = labels("expect(1).not.|").await;
    assert!(
        matchers.iter().any(|label| label == "toHaveProperty"),
        "{matchers:?}"
    );

    let kinds = ASSIST.complete("env.".into(), 4).await.unwrap();
    let get = kinds
        .iter()
        .find(|completion| completion.label == "get")
        .unwrap();
    assert_eq!(get.kind, CompletionKind::Function);
    let globals = ASSIST.complete("e".into(), 1).await.unwrap();
    let env = globals
        .iter()
        .find(|completion| completion.label == "env")
        .unwrap();
    assert_eq!(env.kind, CompletionKind::Variable);
}

#[tokio::test]
async fn a_checker_that_cannot_load_says_so_every_time() {
    let broken = TsAssist::new(|| Err("no TypeScript here".into()));
    for _ in 0..2 {
        assert_eq!(
            full_check(&broken, "").await.unwrap_err(),
            AssistError("no TypeScript here".into())
        );
    }
    assert_eq!(
        broken.complete(String::new(), 0).await.unwrap_err(),
        AssistError("no TypeScript here".into())
    );
    let invalid = TsAssist::new(|| {
        Ok(TypeScriptFiles {
            compiler: "this is not JavaScript (".into(),
            libraries: Vec::new(),
        })
    });
    let error = full_check(&invalid, "").await.unwrap_err();
    assert!(error.0.starts_with("cannot start"), "{error}");
    assert_eq!(stopped().0, "the checker stopped");
}

#[tokio::test]
async fn syntax_errors_come_at_once_while_the_service_starts() {
    let (release, wait) = std::sync::mpsc::channel::<()>();
    let slow = TsAssist::new(move || {
        // Starts only when the test says so.
        let _ = wait.recv();
        Err("stopped on purpose".into())
    });
    assert!(!slow.is_started());
    let first = slow.check("const ä = 1;\nconst = ;".into()).await.unwrap();
    assert!(first.partial);
    assert_eq!(first.diagnostics.len(), 1, "{first:?}");
    assert!(first.diagnostics[0].from >= 13, "{first:?}");
    // Still starting: again at once.
    assert!(slow.check("1;".into()).await.unwrap().partial);
    release.send(()).unwrap();
    assert_eq!(
        full_check(&slow, "1;").await.unwrap_err(),
        AssistError("stopped on purpose".into())
    );
    assert!(slow.is_started());
}

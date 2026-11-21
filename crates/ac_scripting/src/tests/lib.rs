use ac_app::ports::ScriptRequest;
use ac_domain::script::ScriptPhase;

use super::*;

#[tokio::test]
async fn the_runner_runs_scripts_off_the_async_threads() {
    let runner = QuickJsRunner::new(Limits::default());
    let outcome = runner
        .run(ScriptJob {
            phase: ScriptPhase::PreRequest,
            source: "log(req.method)".into(),
            request: ScriptRequest {
                method: "GET".into(),
                ..ScriptRequest::default()
            },
            response: None,
            variables: Vec::new(),
        })
        .await
        .unwrap();
    assert_eq!(outcome.logs, ["GET"]);
}

/// Everything the declarations promise is defined by the prelude.
#[test]
fn declarations_and_prelude_match() {
    let prelude = include_str!("../prelude.js");
    for global in ["req", "res", "env", "log", "console"] {
        assert!(
            API_DECLARATIONS.contains(&format!("declare const {global}:"))
                || API_DECLARATIONS.contains(&format!("declare function {global}(")),
            "{global} not declared"
        );
        assert!(
            prelude.contains(&format!("define('{global}'")),
            "{global} not defined"
        );
    }
    for member in [
        "getHeader",
        "setHeader",
        "removeHeader",
        "headers",
        "text",
        "json",
        "timeMs",
        "status",
        "has",
        "get",
        "set",
    ] {
        assert!(API_DECLARATIONS.contains(member), "{member} not declared");
        assert!(prelude.contains(member), "{member} not implemented");
    }
}

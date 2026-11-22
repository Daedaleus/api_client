use super::*;
use crate::ports::{AssistError, CompletionKind, ScriptCompletion, ScriptDiagnostic, Severity};
use crate::test_support::Fixture;
use crate::testing::FakeScriptAssist;
use crate::text::keys;
use crate::view::{CompletionKindView, SeverityView};

#[tokio::test]
async fn checks_and_completions_pass_through_as_views() {
    let fixture = Fixture::with_assist(
        FakeScriptAssist::new()
            .diagnostics(Ok(vec![ScriptDiagnostic {
                from: 4,
                to: 9,
                severity: Severity::Error,
                message: "Property 'methd' does not exist".into(),
            }]))
            .completions(Ok(vec![ScriptCompletion {
                label: "setHeader".into(),
                kind: CompletionKind::Function,
            }])),
    );
    let AssistResult::Diagnostics { items } = fixture
        .app
        .assist(AssistRequest::Check {
            source: "req.methd".into(),
        })
        .await
        .unwrap()
    else {
        panic!("expected diagnostics");
    };
    assert_eq!(items[0].severity, SeverityView::Error);
    assert_eq!((items[0].from, items[0].to), (4, 9));

    let AssistResult::Completions { items } = fixture
        .app
        .assist(AssistRequest::Complete {
            source: "req.".into(),
            offset: 4,
        })
        .await
        .unwrap()
    else {
        panic!("expected completions");
    };
    assert_eq!(items[0].label, "setHeader");
    assert_eq!(items[0].kind, CompletionKindView::Function);
    assert_eq!(
        fixture.assist.asked(),
        [("req.methd".to_owned(), None), ("req.".to_owned(), Some(4))]
    );
}

#[tokio::test]
async fn a_checker_that_cannot_answer_says_why() {
    let fixture = Fixture::with_assist(
        FakeScriptAssist::new().diagnostics(Err(AssistError("did not start".into()))),
    );
    let error = fixture
        .app
        .assist(AssistRequest::Check {
            source: String::new(),
        })
        .await
        .unwrap_err();
    let text = error.user_text();
    assert_eq!(text.key, keys::SCRIPT_ASSIST);
    assert_eq!(text.args["detail"], "did not start");

    // Without a checker nothing is wrong and nothing suggested.
    let none = crate::ports::NoAssist;
    assert_eq!(none.check(String::new()).await.unwrap().len(), 0);
    assert_eq!(none.complete(String::new(), 0).await.unwrap().len(), 0);
}

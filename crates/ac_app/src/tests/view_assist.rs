use super::*;

#[test]
fn requests_and_results_match_the_editor_contract() {
    let request: AssistRequest = serde_json::from_value(serde_json::json!({
        "type": "complete", "source": "req.", "offset": 4
    }))
    .unwrap();
    assert_eq!(
        request,
        AssistRequest::Complete {
            source: "req.".into(),
            offset: 4
        }
    );
    let result = AssistResult::Diagnostics {
        items: vec![DiagnosticView::from_diagnostic(&ScriptDiagnostic {
            from: 1,
            to: 3,
            severity: Severity::Hint,
            message: "m".into(),
        })],
        partial: true,
    };
    assert_eq!(
        serde_json::to_value(result).unwrap(),
        serde_json::json!({
            "type": "diagnostics",
            "items": [{ "from": 1, "to": 3, "severity": "info", "message": "m" }],
            "partial": true
        })
    );
}

#[test]
fn every_kind_and_severity_has_an_editor_name() {
    let kinds = [
        (CompletionKind::Function, "function"),
        (CompletionKind::Property, "property"),
        (CompletionKind::Variable, "variable"),
        (CompletionKind::Type, "type"),
        (CompletionKind::Keyword, "keyword"),
        (CompletionKind::Other, "text"),
    ];
    for (kind, name) in kinds {
        let view = CompletionView::from_completion(&ScriptCompletion {
            label: "x".into(),
            kind,
        });
        assert_eq!(serde_json::to_value(view).unwrap()["kind"], name);
    }
    for (severity, name) in [(Severity::Error, "error"), (Severity::Warning, "warning")] {
        let view = DiagnosticView::from_diagnostic(&ScriptDiagnostic {
            from: 0,
            to: 0,
            severity,
            message: String::new(),
        });
        assert_eq!(serde_json::to_value(view).unwrap()["severity"], name);
    }
}

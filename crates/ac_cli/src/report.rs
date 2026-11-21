//! Reports of a run: text for people, JSON for programs, `JUnit` XML for CI.

use std::fmt::Write as _;

use ac_app::view::{RunReport, RunResult, RunResultOutcome};

use crate::texts::{Texts, keys};

const PASSED: &str = "✓";
const FAILED: &str = "✗";
/// Shown instead of a status when no response came.
const NO_STATUS: &str = "---";
/// Separator between folder and request names.
const PATH_SEPARATOR: &str = " / ";

/// One line per request and a summary.
pub(crate) fn text(
    report: &RunReport,
    collection: &str,
    environment: Option<&str>,
    error_status_fails: bool,
    texts: &Texts,
) -> String {
    let mut out = match environment {
        Some(environment) => texts.get(
            keys::HEADING_ENVIRONMENT,
            &[("collection", collection), ("environment", environment)],
        ),
        None => texts.get(keys::HEADING, &[("collection", collection)]),
    };
    out.push('\n');
    let width = report
        .results
        .iter()
        .map(|result| result.method.len())
        .max()
        .unwrap_or(0);
    for result in &report.results {
        let mark = if result.passed(error_status_fails) {
            PASSED
        } else {
            FAILED
        };
        let name = result.path.join(PATH_SEPARATOR);
        let method = &result.method;
        let _ = match &result.outcome {
            RunResultOutcome::Response { status, elapsed_ms } => writeln!(
                out,
                "  {mark} {status} {method:<width$}  {name}  ({elapsed_ms} ms)"
            ),
            RunResultOutcome::Failed { error } => writeln!(
                out,
                "  {mark} {NO_STATUS} {method:<width$}  {name}  – {}",
                texts.text(error)
            ),
        };
    }
    let total = report.results.len();
    let failed = report.failed(error_status_fails);
    out.push_str(&texts.get(
        keys::SUMMARY,
        &[
            ("total", &total.to_string()),
            ("passed", &(total - failed).to_string()),
            ("failed", &failed.to_string()),
        ],
    ));
    out.push('\n');
    out
}

/// The report as JSON, each result with `passed`.
pub(crate) fn json(report: &RunReport, error_status_fails: bool) -> String {
    let results: Vec<serde_json::Value> = report
        .results
        .iter()
        .map(|result| {
            let mut value = serde_json::to_value(result).unwrap_or_default();
            if let Some(object) = value.as_object_mut() {
                object.insert(
                    "passed".into(),
                    serde_json::Value::Bool(result.passed(error_status_fails)),
                );
            }
            value
        })
        .collect();
    let mut out = serde_json::to_string_pretty(&serde_json::json!({ "results": results }))
        .unwrap_or_default();
    out.push('\n');
    out
}

/// `JUnit` XML: one test suite, one test case per request.
pub(crate) fn junit(
    report: &RunReport,
    collection: &str,
    error_status_fails: bool,
    texts: &Texts,
) -> String {
    let total = report.results.len();
    let failed = report.failed(error_status_fails);
    let seconds: u64 = report.results.iter().map(elapsed_ms).sum();
    let mut out = String::from("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
    let _ = writeln!(
        out,
        "<testsuites><testsuite name=\"{}\" tests=\"{total}\" failures=\"{failed}\" time=\"{}\">",
        escape(collection),
        millis(seconds)
    );
    for result in &report.results {
        let (folders, name) = result
            .path
            .split_last()
            .map_or((&[][..], ""), |(name, folders)| (folders, name.as_str()));
        let classname = if folders.is_empty() {
            collection.to_owned()
        } else {
            folders.join(".")
        };
        let _ = write!(
            out,
            "  <testcase classname=\"{}\" name=\"{}\" time=\"{}\"",
            escape(&classname),
            escape(&format!("{} {name}", result.method)),
            millis(elapsed_ms(result))
        );
        if result.passed(error_status_fails) {
            out.push_str("/>\n");
            continue;
        }
        let message = match &result.outcome {
            RunResultOutcome::Response { status, .. } => format!("HTTP {status}"),
            RunResultOutcome::Failed { error } => texts.text(error),
        };
        let _ = writeln!(
            out,
            "><failure message=\"{}\"/></testcase>",
            escape(&message)
        );
    }
    out.push_str("</testsuite></testsuites>\n");
    out
}

fn elapsed_ms(result: &RunResult) -> u64 {
    match result.outcome {
        RunResultOutcome::Response { elapsed_ms, .. } => elapsed_ms,
        RunResultOutcome::Failed { .. } => 0,
    }
}

/// Milliseconds as `JUnit`'s seconds with three decimals.
fn millis(ms: u64) -> String {
    format!("{}.{:03}", ms / 1000, ms % 1000)
}

/// Text for an XML attribute.
fn escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

#[cfg(test)]
#[path = "tests/report.rs"]
mod tests;

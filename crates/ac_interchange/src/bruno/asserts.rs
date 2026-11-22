//! Bruno's `assert` block (`res.status: eq 200`) as tests in the
//! post-response script, written with the chai-style `expect` that the
//! script API offers for Bruno scripts. Assertions without a counterpart
//! are reported.

use super::bru::Block;
use crate::import::{Skipped, SkippedKind};

/// One `test(…)` line per enabled assertion that can be translated.
pub(super) fn tests(block: &Block, item: &str, report: &mut Vec<Skipped>) -> Vec<String> {
    block
        .entries()
        .into_iter()
        .filter(|entry| entry.enabled)
        .filter_map(|entry| {
            let line = translate(&entry.key, &entry.value);
            if line.is_none() {
                report.push(Skipped {
                    item: item.to_owned(),
                    what: SkippedKind::Assertion(format!("{}: {}", entry.key, entry.value)),
                });
            }
            line
        })
        .collect()
}

/// The test for `subject: operator value`, if there is one.
fn translate(subject: &str, check: &str) -> Option<String> {
    let actual = actual(subject)?;
    let (operator, value) = check.split_once(' ').unwrap_or((check, ""));
    let value = literal(value.trim());
    let assertion = match operator {
        "eq" => format!("to.eql({value})"),
        "neq" => format!("not.to.eql({value})"),
        "gt" => format!("to.be.above({value})"),
        "gte" => format!("to.be.at.least({value})"),
        "lt" => format!("to.be.below({value})"),
        "lte" => format!("to.be.at.most({value})"),
        "contains" => format!("to.include({value})"),
        "notContains" => format!("not.to.include({value})"),
        "length" => format!("to.have.lengthOf({value})"),
        "isEmpty" => "to.be.empty".to_owned(),
        "isNull" => "to.be.null".to_owned(),
        "isUndefined" => "to.be.undefined".to_owned(),
        "isDefined" => "not.to.be.undefined".to_owned(),
        "isTruthy" => "to.be.ok".to_owned(),
        "isFalsy" => "not.to.be.ok".to_owned(),
        "isNumber" => "to.be.a('number')".to_owned(),
        "isString" => "to.be.a('string')".to_owned(),
        "isBoolean" => "to.be.a('boolean')".to_owned(),
        "isArray" => "to.be.an('array')".to_owned(),
        "isJson" => "to.be.an('object')".to_owned(),
        _ => return None,
    };
    let name = serde_json::to_string(&format!("{subject}: {check}")).ok()?;
    Some(format!("test({name}, () => expect({actual}).{assertion});"))
}

/// The script expression for a Bruno subject like `res.body.items[0]`.
fn actual(subject: &str) -> Option<String> {
    let subject = subject.trim();
    if subject == "res.status" {
        return Some("res.getStatus()".to_owned());
    }
    if subject == "res.responseTime" {
        return Some("res.getResponseTime()".to_owned());
    }
    if let Some(name) = subject.strip_prefix("res.headers.") {
        return Some(format!(
            "res.getHeader({})",
            serde_json::to_string(name).ok()?
        ));
    }
    let rest = subject.strip_prefix("res.body")?;
    // Only plain property paths, nothing that could run other code.
    let plain = rest
        .chars()
        .all(|c| c.is_alphanumeric() || matches!(c, '.' | '_' | '$' | '[' | ']' | '\'' | '"'));
    (plain && (rest.is_empty() || rest.starts_with(['.', '['])))
        .then(|| format!("res.getBody(){rest}"))
}

/// A Bruno value as JavaScript: numbers, `true`, `false` and `null` as
/// written, anything else (quoted or not) as a string literal – never as
/// code.
fn literal(value: &str) -> String {
    if matches!(value, "true" | "false" | "null") || value.parse::<f64>().is_ok() {
        return value.to_owned();
    }
    let quoted = value.len() >= 2
        && ((value.starts_with('"') && value.ends_with('"'))
            || (value.starts_with('\'') && value.ends_with('\'')));
    let text = if quoted {
        &value[1..value.len() - 1]
    } else {
        value
    };
    serde_json::to_string(text).unwrap_or_default()
}

#[cfg(test)]
#[path = "../tests/bruno_asserts.rs"]
mod tests;

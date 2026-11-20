//! Body preview: text for display, JSON re-indented, large bodies truncated.
//!
//! JSON is re-indented token by token instead of parsed and printed again, so
//! numbers (`1.10`, big integers) and key order stay exactly as received.

use serde::Serialize;

/// Bodies larger than this are shown truncated.
pub const PREVIEW_LIMIT: usize = 1024 * 1024;

const INDENT: &str = "  ";

/// What the UI shows of a body.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct BodyView {
    /// Text to show (empty for binary bodies).
    pub text: String,
    /// The text is re-indented JSON.
    pub pretty: bool,
    /// Only the first [`PREVIEW_LIMIT`] bytes are shown.
    pub truncated: bool,
    /// The body is not UTF-8 text; nothing is shown.
    pub binary: bool,
}

/// Builds the preview for a body with the given `Content-Type`.
#[must_use]
pub fn preview(content_type: Option<&str>, body: &[u8]) -> BodyView {
    preview_with_limit(content_type, body, PREVIEW_LIMIT)
}

fn preview_with_limit(content_type: Option<&str>, body: &[u8], limit: usize) -> BodyView {
    let Ok(text) = std::str::from_utf8(body) else {
        return BodyView {
            text: String::new(),
            pretty: false,
            truncated: false,
            binary: true,
        };
    };
    let fits = text.len() <= limit;
    if fits && is_json(content_type) && is_valid_json(text) {
        return BodyView {
            text: reindent(text),
            pretty: true,
            truncated: false,
            binary: false,
        };
    }
    let mut end = text.len().min(limit);
    while !text.is_char_boundary(end) {
        end -= 1;
    }
    BodyView {
        text: text[..end].to_owned(),
        pretty: false,
        truncated: !fits,
        binary: false,
    }
}

/// `application/json` and `…+json` types, parameters ignored.
fn is_json(content_type: Option<&str>) -> bool {
    content_type.is_some_and(|value| {
        let mime = value.split(';').next().unwrap_or_default().trim();
        mime.eq_ignore_ascii_case("application/json")
            || mime.to_ascii_lowercase().ends_with("+json")
    })
}

fn is_valid_json(text: &str) -> bool {
    serde_json::from_str::<serde::de::IgnoredAny>(text).is_ok()
}

/// Re-indents valid JSON with two spaces; strings are copied unchanged.
fn reindent(json: &str) -> String {
    let mut out = String::with_capacity(json.len() * 2);
    let mut depth = 0usize;
    let mut in_string = false;
    let mut escaped = false;
    let mut chars = json.chars().peekable();
    while let Some(c) = chars.next() {
        if in_string {
            out.push(c);
            if escaped {
                escaped = false;
            } else if c == '\\' {
                escaped = true;
            } else if c == '"' {
                in_string = false;
            }
            continue;
        }
        match c {
            '"' => {
                in_string = true;
                out.push(c);
            }
            '{' | '[' => {
                out.push(c);
                while chars.next_if(|next| next.is_whitespace()).is_some() {}
                if let Some(close) = chars.next_if(|next| matches!(next, '}' | ']')) {
                    out.push(close);
                } else {
                    depth += 1;
                    newline(&mut out, depth);
                }
            }
            '}' | ']' => {
                depth = depth.saturating_sub(1);
                newline(&mut out, depth);
                out.push(c);
            }
            ',' => {
                out.push(c);
                newline(&mut out, depth);
            }
            ':' => out.push_str(": "),
            c if c.is_whitespace() => {}
            c => out.push(c),
        }
    }
    out
}

fn newline(out: &mut String, depth: usize) {
    out.push('\n');
    for _ in 0..depth {
        out.push_str(INDENT);
    }
}

#[cfg(test)]
#[path = "tests/body.rs"]
mod tests;

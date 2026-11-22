//! Syntax errors at once, from oxc's parser, while the language service is
//! still starting (ADR S2, decision 3).

use ac_app::ports::{ScriptDiagnostic, Severity};
use oxc_allocator::Allocator;
use oxc_parser::Parser;
use oxc_span::SourceType;

/// The syntax errors of the TypeScript `source`, with JavaScript offsets.
pub(crate) fn diagnostics(source: &str) -> Vec<ScriptDiagnostic> {
    let allocator = Allocator::default();
    let parsed = Parser::new(&allocator, source, SourceType::ts()).parse();
    parsed
        .diagnostics
        .iter()
        .map(|diagnostic| {
            let (from, to) = diagnostic.labels.first().map_or((0, 0), |label| {
                let from = byte_offset(label.offset());
                (from, from.saturating_add(byte_offset(label.len())))
            });
            ScriptDiagnostic {
                from: utf16_offset(source, from),
                to: utf16_offset(source, to),
                severity: Severity::Error,
                message: diagnostic.message.to_string(),
            }
        })
        .collect()
}

/// oxc's `u32` offsets (sources are limited to 4 GiB) as `usize`.
fn byte_offset(offset: u32) -> usize {
    usize::try_from(offset).unwrap_or(usize::MAX)
}

/// The JavaScript (UTF-16) offset of the byte `offset` in `source`; inside a
/// character it counts from that character's start.
fn utf16_offset(source: &str, offset: usize) -> usize {
    let mut offset = offset.min(source.len());
    while !source.is_char_boundary(offset) {
        offset -= 1;
    }
    source[..offset].encode_utf16().count()
}

#[cfg(test)]
#[path = "../tests/assist_syntax.rs"]
mod tests;

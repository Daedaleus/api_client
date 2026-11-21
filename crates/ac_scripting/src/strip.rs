//! TypeScript → JavaScript with oxc: types removed, enums lowered. No type
//! checking here – that is the checker's job (ADR S2).

use std::path::Path;

use ac_app::ports::ScriptError;
use oxc_allocator::Allocator;
use oxc_codegen::Codegen;
use oxc_parser::Parser;
use oxc_semantic::SemanticBuilder;
use oxc_span::SourceType;
use oxc_transformer::{TransformOptions, Transformer};

/// The JavaScript of `source`, or its first syntax error.
pub(crate) fn strip(source: &str) -> Result<String, ScriptError> {
    let allocator = Allocator::default();
    let parsed = Parser::new(&allocator, source, SourceType::ts()).parse();
    if let Some(diagnostic) = parsed.diagnostics.first() {
        let offset = diagnostic.labels.first().map(oxc_span::LabeledSpan::offset);
        return Err(syntax_error(source, offset, &diagnostic.message));
    }
    let mut program = parsed.program;
    // Enum members are evaluated so enums can be lowered.
    let scoping = SemanticBuilder::new()
        .with_enum_eval(true)
        .build(&program)
        .semantic
        .into_scoping();
    let transformed = Transformer::new(
        &allocator,
        Path::new("script.ts"),
        &TransformOptions::default(),
    )
    .build_with_scoping(scoping, &mut program);
    if let Some(diagnostic) = transformed.diagnostics.first() {
        let offset = diagnostic.labels.first().map(oxc_span::LabeledSpan::offset);
        return Err(syntax_error(source, offset, &diagnostic.message));
    }
    Ok(Codegen::new().build(&program).code)
}

/// The error at byte `offset` of `source` (start of the script if unknown).
fn syntax_error(source: &str, offset: Option<u32>, message: &str) -> ScriptError {
    let offset = offset
        .and_then(|offset| usize::try_from(offset).ok())
        .unwrap_or(0)
        .min(source.len());
    let line = source
        .get(..offset)
        .map_or(1, |before| before.matches('\n').count() + 1);
    ScriptError::Syntax {
        line,
        message: message.to_owned(),
    }
}

#[cfg(test)]
#[path = "tests/strip.rs"]
mod tests;

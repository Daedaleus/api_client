//! TypeScript → JavaScript with oxc: types removed, enums lowered. No type
//! checking here – that is the checker's job (ADR S2). The source map of
//! the generated code leads runtime errors back to the line the user wrote.

use std::path::{Path, PathBuf};

use ac_app::ports::ScriptError;
use oxc_allocator::Allocator;
use oxc_codegen::{Codegen, CodegenOptions};
use oxc_parser::Parser;
use oxc_semantic::SemanticBuilder;
use oxc_span::SourceType;
use oxc_transformer::{TransformOptions, Transformer};

/// JavaScript made from a script, with where its parts came from.
#[derive(Debug)]
pub(crate) struct Stripped {
    /// The JavaScript.
    pub code: String,
    /// Mappings `(generated line, generated column, source line)`, all
    /// 0-based, in the order of the generated code.
    mappings: Vec<(u32, u32, u32)>,
}

impl Stripped {
    /// The 1-based line of the script for a 0-based position in the
    /// generated code: the last mapping at or before it on that line.
    pub fn source_line(&self, line: u32, column: u32) -> Option<usize> {
        let on_line = self
            .mappings
            .iter()
            .filter(|(generated, ..)| *generated == line);
        let mapping = on_line
            .clone()
            .take_while(|(_, generated_column, _)| *generated_column <= column)
            .last()
            .or_else(|| on_line.clone().next())?;
        usize::try_from(mapping.2).ok().map(|line| line + 1)
    }
}

/// The JavaScript of `source`, or its first syntax error.
pub(crate) fn strip(source: &str) -> Result<Stripped, ScriptError> {
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
    let generated = Codegen::new()
        .with_options(CodegenOptions {
            source_map_path: Some(PathBuf::from("script.ts")),
            ..CodegenOptions::default()
        })
        .build(&program);
    let mappings = generated.map.map_or_else(Vec::new, |map| {
        map.get_tokens()
            .map(|token| {
                (
                    token.get_dst_line(),
                    token.get_dst_col(),
                    token.get_src_line(),
                )
            })
            .collect()
    });
    Ok(Stripped {
        code: generated.code,
        mappings,
    })
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

//! TypeScript → JavaScript with oxc (types removed, enums lowered), then run
//! in QuickJS.

use std::path::Path;
use std::time::Instant;

use oxc_allocator::Allocator;
use oxc_codegen::Codegen;
use oxc_parser::Parser;
use oxc_semantic::SemanticBuilder;
use oxc_span::SourceType;
use oxc_transformer::{TransformOptions, Transformer};
use rquickjs::{Context, Runtime};
use script_runtime_spike::{isolated, median, ms};

/// A test script as users would write it, with the TypeScript features
/// scripts use: interfaces, type annotations, generics, `as`, `!`, enums.
const SCRIPT: &str = r#"
interface Product { id: number; name: string; price?: number }
enum Status { Ok = 200, NotFound = 404 }
type Body = { items: Product[] };

function cheapest<T extends Product>(items: readonly T[]): T | undefined {
    return [...items].sort((a, b) => (a.price ?? 0) - (b.price ?? 0))[0];
}

const body = JSON.parse('{"items":[{"id":1,"name":"a","price":3},{"id":2,"name":"b","price":1}]}') as Body;
const first = cheapest(body.items)!;
const status: Status = Status.Ok;
first.id === 2 && status === 200 && Status[404] === 'NotFound'
"#;

/// The JavaScript of `source`, or the syntax errors.
fn strip(source: &str) -> Result<String, Vec<String>> {
    let allocator = Allocator::default();
    let parsed = Parser::new(&allocator, source, SourceType::ts()).parse();
    if !parsed.diagnostics.is_empty() {
        return Err(parsed.diagnostics.iter().map(ToString::to_string).collect());
    }
    let mut program = parsed.program;
    let scoping = SemanticBuilder::new()
        .build(&program)
        .semantic
        .into_scoping();
    let result = Transformer::new(
        &allocator,
        Path::new("script.ts"),
        &TransformOptions::default(),
    )
    .build_with_scoping(scoping, &mut program);
    if !result.diagnostics.is_empty() {
        return Err(result.diagnostics.iter().map(ToString::to_string).collect());
    }
    Ok(Codegen::new().build(&program).code)
}

fn main() {
    println!("# TypeScript stripping (oxc 0.159)\n");
    let start = Instant::now();
    let js = strip(SCRIPT).expect("valid TypeScript");
    println!("first run: {}", ms(start.elapsed()));
    println!("median of 21: {}", ms(median(21, || drop(strip(SCRIPT)))));
    println!("--- output ---\n{js}--------------");

    let runtime = Runtime::new().unwrap();
    let context = Context::full(&runtime).unwrap();
    context.with(|ctx| {
        let value: Result<bool, _> = ctx.eval(isolated(&js));
        println!("runs in QuickJS: {:?}", value.map_err(|e| e.to_string()));
    });

    match strip("const x: number = ;\nlet y = 1") {
        Ok(_) => println!("syntax error not found"),
        Err(errors) => println!("syntax error reported: {errors:?}"),
    }
}

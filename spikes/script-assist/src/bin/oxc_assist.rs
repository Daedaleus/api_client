//! Own checks and completion in Rust with oxc: syntax errors, unknown
//! globals and unknown members of the API objects, from `api.d.ts` – no type
//! checker.

use std::collections::{BTreeSet, HashMap};
use std::time::Instant;

use oxc_allocator::Allocator;
use oxc_ast::ast::{
    BindingPattern, Declaration, Expression, PropertyKey, StaticMemberExpression, TSSignature,
    TSType, TSTypeName,
};
use oxc_ast_visit::{Visit, walk};
use oxc_parser::Parser;
use oxc_semantic::SemanticBuilder;
use oxc_span::SourceType;

/// Global names of the JavaScript language itself (a hand-kept list – the
/// TypeScript lib files would be the real source).
const LANGUAGE_GLOBALS: &[&str] = &[
    "Object",
    "Array",
    "String",
    "Number",
    "Boolean",
    "Symbol",
    "BigInt",
    "Math",
    "JSON",
    "Date",
    "RegExp",
    "Error",
    "TypeError",
    "RangeError",
    "SyntaxError",
    "Promise",
    "Map",
    "Set",
    "WeakMap",
    "WeakSet",
    "WeakRef",
    "Proxy",
    "Reflect",
    "ArrayBuffer",
    "DataView",
    "Uint8Array",
    "Int32Array",
    "Float64Array",
    "parseInt",
    "parseFloat",
    "isNaN",
    "isFinite",
    "encodeURIComponent",
    "decodeURIComponent",
    "encodeURI",
    "decodeURI",
    "globalThis",
    "undefined",
    "NaN",
    "Infinity",
    "console",
];

/// The API described in `api.d.ts`: global names, and the members of the
/// globals whose type is an interface there.
#[derive(Debug, Default)]
struct Api {
    globals: BTreeSet<String>,
    members: HashMap<String, BTreeSet<String>>,
}

fn read_api(text: &str) -> Api {
    let allocator = Allocator::default();
    let program = Parser::new(&allocator, text, SourceType::d_ts())
        .parse()
        .program;
    let mut interfaces: HashMap<String, BTreeSet<String>> = HashMap::new();
    let mut typed_globals: Vec<(String, String)> = Vec::new();
    let mut api = Api::default();
    for statement in &program.body {
        let Some(declaration) = statement.as_declaration() else {
            continue;
        };
        match declaration {
            Declaration::TSInterfaceDeclaration(interface) => {
                let names = interface
                    .body
                    .body
                    .iter()
                    .filter_map(|signature| match signature {
                        TSSignature::TSPropertySignature(property) => key_name(&property.key),
                        TSSignature::TSMethodSignature(method) => key_name(&method.key),
                        _ => None,
                    })
                    .collect();
                interfaces.insert(interface.id.name.to_string(), names);
            }
            Declaration::VariableDeclaration(variables) => {
                for declarator in &variables.declarations {
                    if let BindingPattern::BindingIdentifier(id) = &declarator.id {
                        api.globals.insert(id.name.to_string());
                        if let Some(annotation) = &declarator.type_annotation
                            && let TSType::TSTypeReference(reference) = &annotation.type_annotation
                            && let TSTypeName::IdentifierReference(name) = &reference.type_name
                        {
                            typed_globals.push((id.name.to_string(), name.name.to_string()));
                        }
                    }
                }
            }
            Declaration::FunctionDeclaration(function) => {
                if let Some(id) = &function.id {
                    api.globals.insert(id.name.to_string());
                }
            }
            _ => {}
        }
    }
    for (global, interface) in typed_globals {
        if let Some(members) = interfaces.get(&interface) {
            api.members.insert(global, members.clone());
        }
    }
    api
}

fn key_name(key: &PropertyKey<'_>) -> Option<String> {
    match key {
        PropertyKey::StaticIdentifier(id) => Some(id.name.to_string()),
        _ => None,
    }
}

/// Collects `apiObject.unknownMember` uses.
struct MemberCheck<'a> {
    api: &'a Api,
    problems: Vec<String>,
}

impl<'a, 'ast> Visit<'ast> for MemberCheck<'a> {
    fn visit_static_member_expression(&mut self, it: &StaticMemberExpression<'ast>) {
        if let Expression::Identifier(object) = &it.object
            && let Some(members) = self.api.members.get(object.name.as_str())
            && !members.contains(it.property.name.as_str())
        {
            self.problems.push(format!(
                "Property '{}' does not exist on '{}'",
                it.property.name, object.name
            ));
        }
        walk::walk_static_member_expression(self, it);
    }
}

fn check(api: &Api, script: &str) -> Vec<String> {
    let allocator = Allocator::default();
    let parsed = Parser::new(&allocator, script, SourceType::ts()).parse();
    if !parsed.diagnostics.is_empty() {
        return parsed.diagnostics.iter().map(ToString::to_string).collect();
    }
    let semantic = SemanticBuilder::new().build(&parsed.program).semantic;
    let mut problems: Vec<String> = semantic
        .scoping()
        .root_unresolved_references()
        .keys()
        .map(|name| name.as_str())
        .filter(|name| !LANGUAGE_GLOBALS.contains(name) && !api.globals.contains(*name))
        .map(|name| format!("Cannot find name '{name}'"))
        .collect();
    let mut members = MemberCheck {
        api,
        problems: Vec::new(),
    };
    members.visit_program(&parsed.program);
    problems.extend(members.problems);
    problems
}

/// Completion at `|`: members after `apiObject.`, else globals by prefix.
fn complete(api: &Api, script: &str) -> Vec<String> {
    let before = &script[..script.find('|').unwrap()];
    if let Some(object) = before.strip_suffix('.') {
        let name: String = object
            .chars()
            .rev()
            .take_while(|c| c.is_alphanumeric())
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect();
        return api
            .members
            .get(&name)
            .map(|m| m.iter().cloned().collect())
            .unwrap_or_default();
    }
    let prefix: String = before
        .chars()
        .rev()
        .take_while(|c| c.is_alphanumeric())
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect();
    api.globals
        .iter()
        .map(String::as_str)
        .chain(LANGUAGE_GLOBALS.iter().copied())
        .filter(|name| name.starts_with(&prefix))
        .map(str::to_owned)
        .collect()
}

const CHECKS: &[(&str, &str, Option<&str>)] = &[
    ("syntax error", "const x = ;", Some("Unexpected token")),
    (
        "unknown global",
        "reqq.method;",
        Some("Cannot find name 'reqq'"),
    ),
    (
        "unknown member",
        "req.methd;",
        Some("Property 'methd' does not exist"),
    ),
    (
        "wrong argument type",
        "env.set('a', 1);",
        Some("not assignable"),
    ),
    (
        "typo on a derived value",
        "const body = res.json<{ items: string[] }>();\nbody.itmes;",
        Some("Property 'itmes' does not exist"),
    ),
    (
        "no fetch or Node.js",
        "fetch('x'); require('fs');",
        Some("Cannot find name 'fetch'"),
    ),
    (
        "a correct script",
        "req.setHeader('X', env.get('token') ?? '');\nexpect(res.status).toBe(200);\nlog(res.json<{ id: number }>().id);",
        None,
    ),
];

const COMPLETIONS: &[(&str, &str, &[&str])] = &[
    (
        "members of req",
        "req.|",
        &["setHeader", "method", "headers"],
    ),
    (
        "members of a derived value",
        "res.json<{ items: string[]; total: number }>().|",
        &["items", "total"],
    ),
    ("globals", "e|", &["env", "expect"]),
];

fn main() {
    println!("# Own checks with oxc 0.159 (no type checker)\n");
    let start = Instant::now();
    let api = read_api(include_str!("../../js/api.d.ts"));
    println!(
        "read api.d.ts: {:.2} ms",
        start.elapsed().as_secs_f64() * 1000.0
    );

    let mut passed = 0;
    let mut slowest = 0.0_f64;
    for (name, script, expected) in CHECKS {
        let start = Instant::now();
        let found = check(&api, script);
        slowest = slowest.max(start.elapsed().as_secs_f64() * 1000.0);
        let ok = match expected {
            None => found.is_empty(),
            Some(wanted) => found.iter().any(|message| message.contains(wanted)),
        };
        if ok {
            passed += 1;
        } else {
            println!("  FAIL {name}: {found:?}");
        }
    }
    println!(
        "checks: {passed}/{}, per edit at most {slowest:.2} ms",
        CHECKS.len()
    );

    passed = 0;
    for (name, script, expected) in COMPLETIONS {
        let names = complete(&api, script);
        if expected
            .iter()
            .all(|wanted| names.iter().any(|n| n == wanted))
        {
            passed += 1;
        } else {
            println!("  FAIL {name}: {names:?}");
        }
    }
    println!("completions: {passed}/{}", COMPLETIONS.len());
}

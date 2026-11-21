//! The TypeScript language service inside QuickJS, i.e. in Rust below the UI.
//! Usage: tsls-quickjs <path to node_modules/typescript>

use std::path::PathBuf;
use std::time::Instant;

use rquickjs::{Context, Function, Object, Runtime, Value};

fn main() {
    let typescript = PathBuf::from(std::env::args().nth(1).expect("path to typescript"));
    let library = typescript.join("lib");
    let here = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("js");
    let source = std::fs::read_to_string(library.join("typescript.js")).expect("typescript.js");
    let driver = std::fs::read_to_string(here.join("driver.js")).expect("driver.js");
    let api = std::fs::read_to_string(here.join("api.d.ts")).expect("api.d.ts");

    let runtime = Runtime::new().unwrap();
    // The compiler recurses deeply; QuickJS's default stack is 256 KiB.
    runtime.set_max_stack_size(8 * 1024 * 1024);
    let context = Context::full(&runtime).unwrap();
    context.with(|ctx| {
        let start = Instant::now();
        let result: rquickjs::Result<Value> = ctx.eval(source);
        if let Err(error) = result {
            let exception = ctx.catch();
            panic!("typescript.js: {error} {exception:?}");
        }
        let ts: Object = ctx.globals().get("ts").expect("global ts");
        let version: String = ts.get("version").unwrap();
        println!("# TypeScript {version} language service in QuickJS (rquickjs 0.14)\n");
        println!("load typescript.js: {} ms", start.elapsed().as_millis());

        let read = Function::new(ctx.clone(), move |name: String| -> Option<String> {
            if name == "/api.d.ts" {
                return Some(api.clone());
            }
            std::fs::read_to_string(library.join(name.trim_start_matches('/'))).ok()
        })
        .unwrap();
        let clock = Instant::now();
        let now = Function::new(ctx.clone(), move || clock.elapsed().as_millis() as f64).unwrap();
        // QuickJS has no Intl; completions sort with Intl.Collator. A plain
        // comparison is enough for API names.
        let _: Value = ctx
            .eval(
                "globalThis.Intl = { Collator: function () { return { compare: (a, b) => (a < b ? -1 : a > b ? 1 : 0) }; } };",
            )
            .unwrap();
        let _: Value = ctx.eval(driver).unwrap();
        let run: Function = ctx.globals().get("runAssist").unwrap();
        match run.call::<_, String>((ts, read, now)) {
            Ok(report) => println!("{report}"),
            Err(error) => {
                let exception = ctx.catch();
                println!("failed: {error} {exception:?}");
            }
        }
    });
    let usage = runtime.memory_usage();
    println!("heap used: {} MB", usage.memory_used_size / 1_000_000);
}

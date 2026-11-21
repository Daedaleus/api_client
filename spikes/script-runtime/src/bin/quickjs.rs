//! QuickJS (quickjs-ng) through `rquickjs`.

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use std::time::{Duration, Instant};

use rquickjs::{Context, Ctx, Function, Object, Promise, Runtime, Value};
use script_runtime_spike::cases::{
    BACKTRACKING, BENCHMARK, CASES, ENDLESS, HOST_API, Kind, MEMORY_HOG,
};
use script_runtime_spike::{isolated, median, ms};

fn engine() -> (Runtime, Context) {
    let runtime = Runtime::new().expect("runtime");
    let context = Context::full(&runtime).expect("context");
    (runtime, context)
}

/// Evaluates a case: `true` if it passed, else why not.
fn check(ctx: &Ctx<'_>, kind: Kind, source: &str) -> Result<(), String> {
    let value: Value = ctx
        .eval(isolated(source))
        .map_err(|error| describe(ctx, error))?;
    let result = match kind {
        Kind::Sync => value,
        Kind::Async => {
            let promise = value.into_promise().ok_or("not a promise")?;
            promise
                .finish::<Value>()
                .map_err(|error| describe(ctx, error))?
        }
    };
    match result.as_bool() {
        Some(true) => Ok(()),
        _ => Err(format!("value {result:?}")),
    }
}

fn describe(ctx: &Ctx<'_>, error: rquickjs::Error) -> String {
    if error.is_exception() {
        let exception = ctx.catch();
        let message = exception
            .as_exception()
            .and_then(rquickjs::Exception::message)
            .unwrap_or_else(|| format!("{exception:?}"));
        return format!("exception: {message}");
    }
    error.to_string()
}

/// `req`, `env` and `log` as the app would provide them.
fn install_host_api(
    ctx: &Ctx<'_>,
    env: Rc<RefCell<HashMap<String, String>>>,
    logs: Rc<RefCell<Vec<String>>>,
) {
    let globals = ctx.globals();
    let headers: Rc<RefCell<HashMap<String, String>>> = Rc::default();
    let req = Object::new(ctx.clone()).unwrap();
    req.set("method", "POST").unwrap();
    let set = Rc::clone(&headers);
    req.set(
        "setHeader",
        Function::new(ctx.clone(), move |name: String, value: String| {
            set.borrow_mut().insert(name, value);
        })
        .unwrap(),
    )
    .unwrap();
    let get = Rc::clone(&headers);
    req.set(
        "headers",
        Function::new(ctx.clone(), move || get.borrow().clone()).unwrap(),
    )
    .unwrap();
    globals.set("req", req).unwrap();

    let env_object = Object::new(ctx.clone()).unwrap();
    let read = Rc::clone(&env);
    env_object
        .set(
            "get",
            Function::new(ctx.clone(), move |name: String| {
                read.borrow().get(&name).cloned()
            })
            .unwrap(),
        )
        .unwrap();
    let write = Rc::clone(&env);
    env_object
        .set(
            "set",
            Function::new(ctx.clone(), move |name: String, value: String| {
                write.borrow_mut().insert(name, value);
            })
            .unwrap(),
        )
        .unwrap();
    globals.set("env", env_object).unwrap();
    globals
        .set(
            "log",
            Function::new(ctx.clone(), move |line: String| {
                logs.borrow_mut().push(line)
            })
            .unwrap(),
        )
        .unwrap();
}

fn main() {
    println!("# QuickJS (rquickjs 0.14)\n");

    // Cold start: runtime, context with all intrinsics, a tiny script.
    let cold = median(21, || {
        let (_runtime, context) = engine();
        context.with(|ctx| {
            let _: i32 = ctx.eval("1 + 1").unwrap();
        });
    });
    println!("cold start (runtime + context + `1 + 1`): {}", ms(cold));

    // Coverage.
    let (runtime, context) = engine();
    let mut passed = 0;
    context.with(|ctx| {
        for case in CASES {
            match check(&ctx, case.kind, case.source) {
                Ok(()) => passed += 1,
                Err(why) => println!("  FAIL {}: {why}", case.name),
            }
        }
    });
    println!("coverage: {passed}/{} cases", CASES.len());

    // Host API.
    let env = Rc::new(RefCell::new(HashMap::from([(
        "baseUrl".to_owned(),
        "https://api.example.com".to_owned(),
    )])));
    let logs: Rc<RefCell<Vec<String>>> = Rc::default();
    context.with(|ctx| {
        install_host_api(&ctx, Rc::clone(&env), Rc::clone(&logs));
        let ok = check(&ctx, Kind::Sync, HOST_API);
        println!(
            "host API (req/env/log): {ok:?}, env token = {:?}, log = {:?}",
            env.borrow().get("token"),
            logs.borrow()
        );
    });

    // Benchmark.
    let run = median(5, || {
        context.with(|ctx| check(&ctx, Kind::Sync, BENCHMARK).expect("benchmark"));
    });
    println!("benchmark (1.3 MB JSON, regex, sort): {}", ms(run));
    println!(
        "memory after benchmark: {} bytes",
        runtime.memory_usage().memory_used_size
    );
    drop(context);
    drop(runtime);

    // Stopping an endless loop after 200 ms (wall clock).
    let (runtime, context) = engine();
    let start = Instant::now();
    runtime.set_interrupt_handler(Some(Box::new(move || {
        start.elapsed() > Duration::from_millis(200)
    })));
    context.with(|ctx| {
        let result = check(&ctx, Kind::Sync, ENDLESS);
        println!(
            "endless loop stopped after {}: {result:?}",
            ms(start.elapsed())
        );
        // The context stays usable afterwards.
        let after: Result<i32, _> = ctx.eval("40 + 2");
        println!("  usable afterwards: {:?}", after.ok());
    });
    drop(context);
    drop(runtime);

    // Catastrophic backtracking in a regular expression, stopped after 200 ms.
    let (runtime, context) = engine();
    let start = Instant::now();
    runtime.set_interrupt_handler(Some(Box::new(move || {
        start.elapsed() > Duration::from_millis(200)
    })));
    context.with(|ctx| {
        let result = check(&ctx, Kind::Sync, BACKTRACKING);
        println!(
            "regex backtracking stopped after {}: {result:?}",
            ms(start.elapsed())
        );
    });
    drop(context);
    drop(runtime);

    // Memory limit of 32 MB.
    let (runtime, context) = engine();
    runtime.set_memory_limit(32 * 1024 * 1024);
    context.with(|ctx| {
        let start = Instant::now();
        let result = check(&ctx, Kind::Sync, MEMORY_HOG);
        println!(
            "memory hog with 32 MB limit, after {}: {result:?}",
            ms(start.elapsed())
        );
    });

    // A promise from Rust: host functions can be async (an HTTP call later).
    let (_runtime, context) = engine();
    context.with(|ctx| {
        let (promise, resolve, _reject) = Promise::new(&ctx).unwrap();
        ctx.globals().set("pending", promise).unwrap();
        let script: Promise = ctx
            .eval::<Value, _>("(async () => (await pending) === 'from rust')()")
            .unwrap()
            .into_promise()
            .unwrap();
        resolve.call::<_, ()>(("from rust",)).unwrap();
        println!(
            "promise resolved by the host: {:?}",
            script.finish::<bool>().map_err(|e| e.to_string())
        );
    });
}

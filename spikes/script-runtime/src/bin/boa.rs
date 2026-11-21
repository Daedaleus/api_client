//! Boa (pure Rust).

use std::cell::RefCell;
use std::collections::HashMap;
use std::time::Instant;

use boa_engine::builtins::promise::PromiseState;
use boa_engine::object::ObjectInitializer;
use boa_engine::object::builtins::JsPromise;
use boa_engine::property::Attribute;
use boa_engine::{
    Context, JsError, JsObject, JsResult, JsString, JsValue, NativeFunction, Source, js_string,
};
use script_runtime_spike::cases::{BACKTRACKING, BENCHMARK, CASES, ENDLESS, HOST_API, Kind};
use script_runtime_spike::{isolated, median, ms};

thread_local! {
    // Host state; Boa's closures must be `Copy` or capture traced GC values,
    // so the spike keeps it in thread-locals.
    static HEADERS: RefCell<HashMap<String, String>> = RefCell::default();
    static ENV: RefCell<HashMap<String, String>> = RefCell::default();
    static LOGS: RefCell<Vec<String>> = RefCell::default();
}

fn describe(error: &JsError, context: &mut Context) -> String {
    match error.try_native(context) {
        Ok(native) => format!("exception: {native}"),
        Err(_) => match error.clone().into_opaque(context) {
            Ok(value) => format!("exception: {}", value.display()),
            Err(_) => format!("exception: {error}"),
        },
    }
}

fn check(context: &mut Context, kind: Kind, source: &str) -> Result<(), String> {
    let value = context
        .eval(Source::from_bytes(&isolated(source)))
        .map_err(|error| describe(&error, context))?;
    let result = match kind {
        Kind::Sync => value,
        Kind::Async => {
            let promise = value
                .as_object()
                .and_then(|object| JsPromise::from_object(object.clone()).ok())
                .ok_or("not a promise")?;
            context
                .run_jobs()
                .map_err(|error| describe(&error, context))?;
            match promise.state() {
                PromiseState::Fulfilled(value) => value,
                PromiseState::Rejected(reason) => {
                    return Err(format!("rejected: {}", reason.display()));
                }
                PromiseState::Pending => return Err("still pending".into()),
            }
        }
    };
    match result.as_boolean() {
        Some(true) => Ok(()),
        _ => Err(format!("value {}", result.display())),
    }
}

fn text(args: &[JsValue], index: usize, context: &mut Context) -> JsResult<String> {
    Ok(args
        .get(index)
        .cloned()
        .unwrap_or_default()
        .to_string(context)?
        .to_std_string_escaped())
}

fn install_host_api(context: &mut Context) {
    let set_header = NativeFunction::from_copy_closure(|_, args, context| {
        let (name, value) = (text(args, 0, context)?, text(args, 1, context)?);
        HEADERS.with(|headers| headers.borrow_mut().insert(name, value));
        Ok(JsValue::undefined())
    });
    let headers = NativeFunction::from_copy_closure(|_, _, context| {
        let object = JsObject::with_object_proto(context.intrinsics());
        for (name, value) in HEADERS.with(|headers| headers.borrow().clone()) {
            object.set(
                JsString::from(name.as_str()),
                JsString::from(value.as_str()),
                false,
                context,
            )?;
        }
        Ok(object.into())
    });
    let req = ObjectInitializer::new(context)
        .property(js_string!("method"), js_string!("POST"), Attribute::all())
        .function(set_header, js_string!("setHeader"), 2)
        .function(headers, js_string!("headers"), 0)
        .build();
    context
        .register_global_property(js_string!("req"), req, Attribute::all())
        .unwrap();

    let get = NativeFunction::from_copy_closure(|_, args, context| {
        let name = text(args, 0, context)?;
        Ok(ENV
            .with(|env| env.borrow().get(&name).cloned())
            .map_or(JsValue::undefined(), |value| {
                JsString::from(value.as_str()).into()
            }))
    });
    let set = NativeFunction::from_copy_closure(|_, args, context| {
        let (name, value) = (text(args, 0, context)?, text(args, 1, context)?);
        ENV.with(|env| env.borrow_mut().insert(name, value));
        Ok(JsValue::undefined())
    });
    let env = ObjectInitializer::new(context)
        .function(get, js_string!("get"), 1)
        .function(set, js_string!("set"), 2)
        .build();
    context
        .register_global_property(js_string!("env"), env, Attribute::all())
        .unwrap();
    context
        .register_global_callable(
            js_string!("log"),
            1,
            NativeFunction::from_copy_closure(|_, args, context| {
                let line = text(args, 0, context)?;
                LOGS.with(|logs| logs.borrow_mut().push(line));
                Ok(JsValue::undefined())
            }),
        )
        .unwrap();
}

fn main() {
    println!(
        "# Boa 0.22{}\n",
        if cfg!(feature = "boa-intl") {
            " (intl_bundled)"
        } else {
            ""
        }
    );

    let cold = median(21, || {
        let mut context = Context::default();
        context.eval(Source::from_bytes("1 + 1")).unwrap();
    });
    println!("cold start (context + `1 + 1`): {}", ms(cold));

    let mut context = Context::default();
    let mut passed = 0;
    for case in CASES {
        match check(&mut context, case.kind, case.source) {
            Ok(()) => passed += 1,
            Err(why) => println!("  FAIL {}: {why}", case.name),
        }
    }
    println!("coverage: {passed}/{} cases", CASES.len());

    ENV.with(|env| {
        env.borrow_mut()
            .insert("baseUrl".into(), "https://api.example.com".into())
    });
    install_host_api(&mut context);
    let ok = check(&mut context, Kind::Sync, HOST_API);
    println!(
        "host API (req/env/log): {ok:?}, env token = {:?}, log = {:?}",
        ENV.with(|env| env.borrow().get("token").cloned()),
        LOGS.with(|logs| logs.borrow().clone())
    );

    let run = median(5, || {
        check(&mut context, Kind::Sync, BENCHMARK).expect("benchmark")
    });
    println!("benchmark (1.3 MB JSON, regex, sort): {}", ms(run));

    // No wall-clock interrupt: only a limit on loop iterations.
    let mut context = Context::default();
    context
        .runtime_limits_mut()
        .set_loop_iteration_limit(50_000_000);
    let start = Instant::now();
    let result = check(&mut context, Kind::Sync, ENDLESS);
    println!(
        "endless loop with an iteration limit of 50 M, after {}: {result:?}",
        ms(start.elapsed())
    );
    let after = context
        .eval(Source::from_bytes("40 + 2"))
        .map(|v| v.display().to_string());
    println!("  usable afterwards: {:?}", after.ok());
    // Catastrophic backtracking: the iteration limit does not apply inside
    // the regex engine. Run it on a thread and give up waiting after 5 s.
    let (sender, receiver) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let mut context = Context::default();
        context
            .runtime_limits_mut()
            .set_loop_iteration_limit(50_000_000);
        let start = Instant::now();
        let result = check(&mut context, Kind::Sync, BACKTRACKING);
        let _ = sender.send((start.elapsed(), result));
    });
    match receiver.recv_timeout(std::time::Duration::from_secs(5)) {
        Ok((elapsed, result)) => {
            println!("regex backtracking ended after {}: {result:?}", ms(elapsed))
        }
        Err(_) => println!("regex backtracking: still running after 5 s, cannot be stopped"),
    }
    println!("memory limit: none in Boa (a memory hog would exhaust the process) – not run");

    // A promise resolved by the host.
    let mut context = Context::default();
    let (promise, resolvers) = JsPromise::new_pending(&mut context);
    context
        .register_global_property(js_string!("pending"), promise, Attribute::all())
        .unwrap();
    let script = context
        .eval(Source::from_bytes(
            "(async () => (await pending) === 'from rust')()",
        ))
        .unwrap();
    resolvers
        .resolve
        .call(
            &JsValue::undefined(),
            &[js_string!("from rust").into()],
            &mut context,
        )
        .unwrap();
    context.run_jobs().unwrap();
    let state = JsPromise::from_object(script.as_object().unwrap().clone())
        .unwrap()
        .state();
    println!("promise resolved by the host: {state:?}");
}

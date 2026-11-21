use std::path::Path;

use ac_domain::collection::Request;
use ac_domain::environment::{Environment, Variable};
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

use super::*;

/// A collection on disk with `Health` and `Fails`, and the environment
/// `Local` pointing at `server`.
fn collection(dir: &Path, server: &str) -> PathBuf {
    let root = dir.join("api");
    ac_storage::create::create_collection(&root, "Shop").unwrap();
    let mut health = Request::new("Health");
    health.url = "{{baseUrl}}/health".into();
    ac_storage::create::create_request(&root, &health).unwrap();
    let mut fails = Request::new("Fails");
    fails.url = "{{baseUrl}}/fails".into();
    ac_storage::create::create_request(&root, &fails).unwrap();
    let mut local = Environment::new("Local");
    local.variables.push(Variable::plain("baseUrl", server));
    ac_storage::environment_file::create_environment(&root, &local).unwrap();
    root
}

fn keys(dir: &Path) -> KeyConfig {
    KeyConfig {
        identity_file: dir.join("identity.txt"),
        ci_identity: None,
        ssh_dir: None,
    }
}

fn args(collection: &Path) -> RunArgs {
    RunArgs {
        collection: collection.to_owned(),
        env: Some("Local".into()),
        only: None,
        format: Format::Text,
        allow_error_status: false,
    }
}

/// Runs and returns the outcome, stdout and stderr.
fn run_with(args: &RunArgs, dir: &Path) -> (Outcome, String, String) {
    let mut out = Vec::new();
    let mut err = Vec::new();
    let outcome = run(args, &Texts::new("en"), keys(dir), &mut out, &mut err);
    (
        outcome,
        String::from_utf8(out).unwrap(),
        String::from_utf8(err).unwrap(),
    )
}

async fn server() -> MockServer {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/health"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/fails"))
        .respond_with(ResponseTemplate::new(503))
        .mount(&server)
        .await;
    server
}

#[tokio::test(flavor = "multi_thread")]
async fn a_run_reports_every_request_and_fails_on_error_status() {
    let server = server().await;
    let temp = tempfile::tempdir().unwrap();
    let root = collection(temp.path(), &server.uri());
    let args = args(&root);
    let (outcome, out, err) = tokio::task::spawn_blocking({
        let dir = temp.path().to_owned();
        move || run_with(&args, &dir)
    })
    .await
    .unwrap();
    assert_eq!(outcome, Outcome::Failed, "{out}{err}");
    assert!(out.starts_with("Shop · environment Local\n"), "{out}");
    assert!(out.contains("✓ 200 GET  Health"), "{out}");
    assert!(out.contains("✗ 503 GET  Fails"), "{out}");
    assert!(out.ends_with("2 requests: 1 passed, 1 failed\n"), "{out}");
    assert_eq!(err, "");
}

#[tokio::test(flavor = "multi_thread")]
async fn options_choose_requests_formats_and_what_fails() {
    let server = server().await;
    let temp = tempfile::tempdir().unwrap();
    let root = collection(temp.path(), &server.uri());
    let dir = temp.path().to_owned();
    let results = tokio::task::spawn_blocking(move || {
        let mut only = args(&root);
        only.only = Some("Health".into());
        only.format = Format::Junit;
        let lenient = RunArgs {
            allow_error_status: true,
            format: Format::Json,
            ..args(&root)
        };
        (run_with(&only, &dir), run_with(&lenient, &dir))
    })
    .await
    .unwrap();
    let ((only_outcome, junit, _), (lenient_outcome, json, _)) = results;
    assert_eq!(only_outcome, Outcome::Passed);
    assert!(junit.contains("tests=\"1\" failures=\"0\""), "{junit}");
    assert_eq!(lenient_outcome, Outcome::Passed);
    assert!(json.contains("\"passed\": true"), "{json}");
}

#[test]
fn runs_that_cannot_start_are_errors() {
    let temp = tempfile::tempdir().unwrap();
    let (outcome, out, err) = run_with(&args(&temp.path().join("missing")), temp.path());
    assert_eq!(outcome, Outcome::Error);
    assert_eq!(out, "");
    assert!(err.starts_with("Error: "), "{err}");

    let root = collection(temp.path(), "http://127.0.0.1:9");
    let mut unknown = args(&root);
    unknown.env = Some("Prod".into());
    let (outcome, _, err) = run_with(&unknown, temp.path());
    assert_eq!(outcome, Outcome::Error);
    assert_eq!(err, "Error: No environment named “Prod”.\n");
}

#[test]
fn the_program_parses_its_command_line() {
    assert_eq!(
        main(["api-client-cli", "--version"].map(Into::into)),
        ExitCode::SUCCESS
    );
    assert_eq!(
        main(["api-client-cli", "run"].map(Into::into)),
        ExitCode::from(2)
    );
    let temp = tempfile::tempdir().unwrap();
    let missing = temp.path().join("missing").display().to_string();
    assert_eq!(
        main(["api-client-cli".to_owned(), "run".into(), missing].map(Into::into)),
        ExitCode::from(2)
    );
    assert_eq!(ExitCode::from(Outcome::Passed), ExitCode::SUCCESS);
    assert_eq!(ExitCode::from(Outcome::Failed), ExitCode::from(1));
    assert!(config_dir().ends_with(APP_ID));
}

//! Checks or updates the generated TypeScript bindings
//! (`apps/desktop/src/lib/bindings.ts`).
//!
//! - `bindings --check`: fails when the committed file differs from what the
//!   Rust types generate (`mise run lint`, CI).
//! - `bindings --write`: regenerates the file (`mise run bindings`).
//!
//! A program instead of a unit test, because test binaries that link Tauri
//! cannot start on Windows (see the crate docs).

use std::path::PathBuf;
use std::process::ExitCode;

fn main() -> ExitCode {
    let mode = std::env::args().nth(1).unwrap_or_default();
    let committed = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(ac_desktop::BINDINGS_PATH);
    let fresh_path = std::env::temp_dir().join(format!("ac-bindings-{}.ts", std::process::id()));

    if let Err(error) = ac_desktop::export_bindings(&fresh_path) {
        eprintln!("cannot generate bindings: {error}");
        return ExitCode::FAILURE;
    }
    let fresh = std::fs::read_to_string(&fresh_path).unwrap_or_default();
    let _ = std::fs::remove_file(&fresh_path);

    match mode.as_str() {
        "--write" => match std::fs::write(&committed, &fresh) {
            Ok(()) => {
                println!("updated {}", committed.display());
                ExitCode::SUCCESS
            }
            Err(error) => {
                eprintln!("cannot write {}: {error}", committed.display());
                ExitCode::FAILURE
            }
        },
        "--check" => {
            let current = std::fs::read_to_string(&committed).unwrap_or_default();
            // Line endings differ between checkouts (CRLF on Windows).
            if current.replace("\r\n", "\n") == fresh.replace("\r\n", "\n") {
                println!("bindings are up to date");
                ExitCode::SUCCESS
            } else {
                eprintln!(
                    "apps/desktop/src/lib/bindings.ts is out of date with the Rust types; \
                     run `mise run bindings` and commit the result"
                );
                ExitCode::FAILURE
            }
        }
        _ => {
            eprintln!("usage: bindings --check | --write");
            ExitCode::from(2)
        }
    }
}

//! Desktop app: composition root and thin Tauri adapter.
//!
//! Wires the adapters into [`ac_app::App`], exposes two Tauri commands
//! (`execute`, `query`) and forwards every [`AppEvent`] to the window as
//! [`AppEventMessage`]. No logic lives here – it belongs in `ac_app`.
//!
//! The TypeScript side of this interface (`apps/desktop/src/lib/bindings.ts`)
//! is generated from these types by tauri-specta; `mise run lint` fails when
//! the committed file is out of date, `mise run bindings` regenerates it
//! (program `src/bin/bindings.rs`).
//!
//! Keep this crate free of unit tests: on Windows, test binaries that link
//! Tauri lack the application manifest and cannot start
//! (`STATUS_ENTRYPOINT_NOT_FOUND`). Logic belongs in `ac_app` anyway.

use std::path::Path;

use ac_app::{App, AppEvent, Command, CommandAck, Ports, Query, QueryResult, UserText};
use ac_http::{DEFAULT_TIMEOUT, LoopbackLogin, ReqwestSender};
use ac_scripting::QuickJsRunner;
use ac_secrets::{AgeVault, KeyConfig};
use ac_storage::FileStore;
use serde::Serialize;
use specta_typescript::Typescript;
use tauri::{AppHandle, Manager, State};
use tauri_specta::{Event, collect_commands, collect_events};
use tokio::sync::broadcast::{Receiver, error::RecvError};

type DesktopApp = App<ReqwestSender, FileStore, AgeVault, LoopbackLogin, QuickJsRunner>;

/// The user's own key for secrets, in the app's config folder (ADR S4).
const IDENTITY_FILE: &str = "identity.txt";

/// Header of the generated bindings file.
const BINDINGS_HEADER: &str = "// Generated from the Rust types by tauri-specta - do not edit.\n\
     // Regenerate with `mise run bindings`; `mise run lint` fails when this file is stale.";

/// The committed bindings file, relative to this crate.
pub const BINDINGS_PATH: &str = "../src/lib/bindings.ts";

/// An [`AppEvent`] on its way to the window.
#[derive(Debug, Clone, Serialize, specta::Type, tauri_specta::Event)]
pub struct AppEventMessage(pub AppEvent);

/// Carries out a command from the UI.
#[tauri::command]
#[specta::specta]
// Tauri commands receive their arguments by value.
#[allow(clippy::needless_pass_by_value)]
fn execute(app: State<'_, DesktopApp>, command: Command) -> Result<CommandAck, UserText> {
    app.execute(command).map_err(|error| error.user_text())
}

/// Answers a query from the UI.
#[tauri::command]
#[specta::specta]
#[allow(clippy::needless_pass_by_value)]
fn query(app: State<'_, DesktopApp>, query: Query) -> Result<QueryResult, UserText> {
    app.query(query).map_err(|error| error.user_text())
}

/// Commands and events of the app, the single source for both the Tauri
/// handlers and the TypeScript bindings.
fn specta_builder() -> tauri_specta::Builder<tauri::Wry> {
    tauri_specta::Builder::<tauri::Wry>::new()
        .commands(collect_commands![execute, query])
        .events(collect_events![AppEventMessage])
        // Run ids, milliseconds and byte counts stay far below 2^53.
        .dangerously_cast_bigints_to_number()
}

/// Writes the TypeScript bindings to `path`.
///
/// # Errors
/// If the types cannot be expressed in TypeScript or the file cannot be
/// written.
pub fn export_bindings(path: &Path) -> Result<(), specta_typescript::Error> {
    specta_builder().export(Typescript::default().header(BINDINGS_HEADER), path)
}

/// Runs the desktop app until the window is closed.
///
/// # Panics
/// If the window system or the web view cannot be started.
pub fn run() {
    let builder = specta_builder();
    tauri::Builder::default()
        // Native folder picker for opening and creating collections.
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(builder.invoke_handler())
        .setup(move |tauri_app| {
            builder.mount_events(tauri_app);
            // Commands run on the main thread; the app spawns its work on
            // Tauri's Tokio runtime instead.
            let runtime = tauri::async_runtime::handle().inner().clone();
            let identity = tauri_app.path().app_config_dir()?.join(IDENTITY_FILE);
            let app = App::new(
                Ports {
                    http: ReqwestSender::new(DEFAULT_TIMEOUT)?,
                    store: FileStore::new(),
                    vault: AgeVault::new(KeyConfig::from_environment(identity)),
                    login: LoopbackLogin::new(),
                    scripts: QuickJsRunner::default(),
                },
                runtime,
            );
            tauri::async_runtime::spawn(forward_events(
                tauri_app.handle().clone(),
                app.subscribe(),
            ));
            tauri_app.manage(app);
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("failed to start the desktop app");
}

/// Forwards application events to the window until the app shuts down.
async fn forward_events(handle: AppHandle, mut events: Receiver<AppEvent>) {
    loop {
        match events.recv().await {
            Ok(event) => {
                // The window may already be closing; nothing to do then.
                let _ = AppEventMessage(event).emit(&handle);
            }
            Err(RecvError::Lagged(_)) => {}
            Err(RecvError::Closed) => break,
        }
    }
}

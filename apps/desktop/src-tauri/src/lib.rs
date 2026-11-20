//! Desktop app: composition root and thin Tauri adapter.
//!
//! Wires the adapters into [`ac_app::App`], exposes one Tauri command
//! (`execute`) and forwards every [`AppEvent`] to the window as the Tauri
//! event [`EVENT_NAME`]. No logic lives here – it belongs in `ac_app`.

use ac_app::{App, AppEvent, Command, CommandAck, Ports, UserText};
use ac_http::{DEFAULT_TIMEOUT, ReqwestSender};
use tauri::{AppHandle, Emitter, Manager, State};
use tokio::sync::broadcast::{Receiver, error::RecvError};

/// Name of the Tauri event that carries [`AppEvent`]s to the UI.
pub const EVENT_NAME: &str = "app-event";

type DesktopApp = App<ReqwestSender>;

/// Carries out a command from the UI.
#[tauri::command]
// Tauri commands receive their arguments by value.
#[allow(clippy::needless_pass_by_value)]
fn execute(app: State<'_, DesktopApp>, command: Command) -> Result<CommandAck, UserText> {
    app.execute(command).map_err(|error| error.user_text())
}

/// Runs the desktop app until the window is closed.
///
/// # Panics
/// If the window system or the web view cannot be started.
pub fn run() {
    tauri::Builder::default()
        .setup(|tauri_app| {
            let app = App::new(Ports {
                http: ReqwestSender::new(DEFAULT_TIMEOUT)?,
            });
            tauri::async_runtime::spawn(forward_events(
                tauri_app.handle().clone(),
                app.subscribe(),
            ));
            tauri_app.manage(app);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![execute])
        .run(tauri::generate_context!())
        .expect("failed to start the desktop app");
}

/// Forwards application events to the window until the app shuts down.
async fn forward_events(handle: AppHandle, mut events: Receiver<AppEvent>) {
    loop {
        match events.recv().await {
            Ok(event) => {
                // The window may already be closing; nothing to do then.
                let _ = handle.emit(EVENT_NAME, event);
            }
            Err(RecvError::Lagged(_)) => {}
            Err(RecvError::Closed) => break,
        }
    }
}

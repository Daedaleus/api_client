//! Starting the user's browser with a URL.

use std::process::{Command, Stdio};

use url::Url;

/// The environment variable that names a browser command to use instead of
/// the system's default browser (a common convention, e.g. on Linux).
pub const BROWSER_VARIABLE: &str = "BROWSER";

/// Opens a URL in a browser.
pub trait BrowserOpener: Send + Sync + 'static {
    /// Opens `url`; the error says why not.
    ///
    /// # Errors
    /// If no browser can be started.
    fn open(&self, url: &Url) -> Result<(), String>;
}

/// The user's browser: the command in `BROWSER` if set (split like a shell
/// would, the URL added as last argument), else the system's default.
#[derive(Debug, Clone, Copy, Default)]
pub struct SystemBrowser;

impl BrowserOpener for SystemBrowser {
    fn open(&self, url: &Url) -> Result<(), String> {
        let browser = std::env::var(BROWSER_VARIABLE).ok();
        let mut command = command(url, browser.as_deref())?;
        let mut child = command
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|error| error.to_string())?;
        // Reap the process when it ends; its result does not matter (a
        // launcher may exit before the browser has the page).
        std::thread::spawn(move || child.wait());
        Ok(())
    }
}

/// The command that opens `url`. The URL is always passed as an argument
/// of its own, never through a shell: it contains `&`.
fn command(url: &Url, browser: Option<&str>) -> Result<Command, String> {
    if let Some(browser) = browser.filter(|browser| !browser.trim().is_empty()) {
        let words = shell_words::split(browser).map_err(|error| error.to_string())?;
        let (program, arguments) = words.split_first().ok_or("BROWSER is empty")?;
        let mut command = Command::new(program);
        command.args(arguments).arg(url.as_str());
        return Ok(command);
    }
    let (program, arguments): (&str, &[&str]) = if cfg!(windows) {
        ("rundll32", &["url.dll,FileProtocolHandler"])
    } else if cfg!(target_os = "macos") {
        ("open", &[])
    } else {
        ("xdg-open", &[])
    };
    let mut command = Command::new(program);
    command.args(arguments).arg(url.as_str());
    Ok(command)
}

#[cfg(test)]
#[path = "../tests/browser.rs"]
mod tests;

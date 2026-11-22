//! The [HTTPie](https://httpie.io) command `http` (POSIX shell).

use std::fmt::Write as _;

use ac_domain::exchange::{PartContent, RequestBody, ResolvedRequest};

use super::text_of;

/// Continues the command on the next line.
const NEXT_LINE: &str = " \\\n  ";

/// `text` in single quotes for a POSIX shell.
fn quote(text: &str) -> String {
    format!("'{}'", text.replace('\'', "'\\''"))
}

/// The `http` command that sends the request.
pub(super) fn command(request: &ResolvedRequest) -> String {
    let mut command = String::from("http");
    if request.transport.follow_redirects {
        command.push_str(" --follow");
    }
    if !request.verify_tls {
        command.push_str(" --verify=no");
    }
    if matches!(request.body, Some(RequestBody::Multipart(_))) {
        command.push_str(" --multipart");
    }
    let _ = write!(
        command,
        " --timeout={}",
        request.transport.timeout.as_secs().max(1)
    );
    let _ = write!(
        command,
        "{NEXT_LINE}{} {}",
        request.method.as_str(),
        quote(request.url.as_str())
    );
    for header in &request.headers {
        // `Name:value` items are headers.
        let _ = write!(
            command,
            "{NEXT_LINE}{}",
            quote(&format!("{}:{}", header.name, header.value))
        );
    }
    match &request.body {
        None => {}
        Some(RequestBody::Bytes(bytes)) => {
            let _ = write!(command, "{NEXT_LINE}--raw {}", quote(&text_of(bytes)));
        }
        Some(RequestBody::File(path)) => {
            let _ = write!(
                command,
                "{NEXT_LINE}< {}",
                quote(&path.display().to_string())
            );
        }
        Some(RequestBody::Multipart(parts)) => {
            for part in parts {
                let item = match &part.content {
                    PartContent::Text(text) => format!("{}={text}", part.name),
                    PartContent::File(path) => format!("{}@{}", part.name, path.display()),
                };
                let _ = write!(command, "{NEXT_LINE}{}", quote(&item));
            }
        }
    }
    command
}

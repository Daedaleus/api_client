//! JavaScript: `fetch` (built into browsers and Node.js 18+) and axios.

use std::fmt::Write as _;
use std::path::Path;

use ac_domain::exchange::{PartContent, RequestBody, ResolvedRequest};

use super::{literal, text_of};

/// The body as a JavaScript expression, and whether it needs `node:fs`.
fn body(request: &ResolvedRequest) -> Option<(String, bool)> {
    let read = |path: &Path| format!("fs.readFileSync({})", literal(&path.display().to_string()));
    match request.body.as_ref()? {
        RequestBody::Bytes(bytes) => Some((literal(&text_of(bytes)), false)),
        RequestBody::File(path) => Some((read(path), true)),
        RequestBody::Multipart(parts) => {
            let mut form = String::from("(() => {\n    const form = new FormData();\n");
            let mut uses_files = false;
            for part in parts {
                let value = match &part.content {
                    PartContent::Text(text) => literal(text),
                    PartContent::File(path) => {
                        uses_files = true;
                        let name = path
                            .file_name()
                            .map_or_default(|name| name.to_string_lossy().into_owned());
                        format!("new Blob([{}]), {}", read(path), literal(&name))
                    }
                };
                let _ = writeln!(form, "    form.append({}, {value});", literal(&part.name));
            }
            form.push_str("    return form;\n  })()");
            Some((form, uses_files))
        }
    }
}

/// Headers as an object literal, indented by `indent`.
fn headers(request: &ResolvedRequest, indent: &str) -> String {
    let mut object = String::from("{\n");
    for header in &request.headers {
        let _ = writeln!(
            object,
            "{indent}  {}: {},",
            literal(&header.name),
            literal(&header.value)
        );
    }
    let _ = write!(object, "{indent}}}");
    object
}

/// `await fetch(…)`.
pub(super) fn fetch(request: &ResolvedRequest) -> String {
    let body = body(request);
    let mut code = String::new();
    if body.as_ref().is_some_and(|(_, fs)| *fs) {
        code.push_str("import fs from 'node:fs';\n\n");
    }
    if !request.verify_tls {
        code.push_str(
            "// The certificate check is off for this request; fetch has no option for that\n\
             // (Node.js: start with NODE_TLS_REJECT_UNAUTHORIZED=0, for test servers only).\n",
        );
    }
    let _ = writeln!(
        code,
        "const response = await fetch({}, {{",
        literal(request.url.as_str())
    );
    let _ = writeln!(code, "  method: {},", literal(request.method.as_str()));
    if !request.headers.is_empty() {
        let _ = writeln!(code, "  headers: {},", headers(request, "  "));
    }
    if let Some((body, _)) = body {
        let _ = writeln!(code, "  body: {body},");
    }
    if !request.transport.follow_redirects {
        code.push_str("  redirect: 'manual',\n");
    }
    code.push_str("});\nconsole.log(response.status, await response.text());\n");
    code
}

/// `await axios.request(…)`.
pub(super) fn axios(request: &ResolvedRequest) -> String {
    let body = body(request);
    let mut code = String::from("import axios from 'axios';\n");
    if body.as_ref().is_some_and(|(_, fs)| *fs) {
        code.push_str("import fs from 'node:fs';\n");
    }
    if !request.verify_tls {
        code.push_str("import https from 'node:https';\n");
    }
    code.push_str("\nconst response = await axios.request({\n");
    let _ = writeln!(code, "  method: {},", literal(request.method.as_str()));
    let _ = writeln!(code, "  url: {},", literal(request.url.as_str()));
    if !request.headers.is_empty() {
        let _ = writeln!(code, "  headers: {},", headers(request, "  "));
    }
    if let Some((body, _)) = body {
        let _ = writeln!(code, "  data: {body},");
    }
    if !request.transport.follow_redirects {
        code.push_str("  maxRedirects: 0,\n");
    }
    if !request.verify_tls {
        code.push_str("  // Certificate check off (test servers only).\n");
        code.push_str("  httpsAgent: new https.Agent({ rejectUnauthorized: false }),\n");
    }
    code.push_str(
        "  validateStatus: () => true,\n});\nconsole.log(response.status, response.data);\n",
    );
    code
}

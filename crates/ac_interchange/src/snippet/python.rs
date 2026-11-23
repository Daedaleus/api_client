//! Python with [requests](https://requests.readthedocs.io).

use std::fmt::Write as _;
use std::path::Path;

use ac_domain::exchange::{PartContent, RequestBody, ResolvedRequest};

use super::{literal, text_of};
use crate::curl::DigestCredentials;

/// `requests.request(…)`.
pub(super) fn requests(request: &ResolvedRequest, digest: Option<&DigestCredentials>) -> String {
    let open = |path: &Path| format!("open({}, \"rb\")", literal(&path.display().to_string()));
    let mut code = String::from("import requests\n");
    if digest.is_some() {
        code.push_str("from requests.auth import HTTPDigestAuth\n");
    }
    code.push('\n');
    code.push_str("response = requests.request(\n");
    let _ = writeln!(code, "    {},", literal(request.method.as_str()));
    let _ = writeln!(code, "    {},", literal(request.url.as_str()));
    if !request.headers.is_empty() {
        code.push_str("    headers={\n");
        for header in &request.headers {
            let _ = writeln!(
                code,
                "        {}: {},",
                literal(&header.name),
                literal(&header.value)
            );
        }
        code.push_str("    },\n");
    }
    match &request.body {
        None => {}
        Some(RequestBody::Bytes(bytes)) => {
            let _ = writeln!(
                code,
                "    data={}.encode(\"utf-8\"),",
                literal(&text_of(bytes))
            );
        }
        Some(RequestBody::File(path)) => {
            let _ = writeln!(code, "    data={},", open(path));
        }
        Some(RequestBody::Multipart(parts)) => {
            // requests builds the multipart body (and its Content-Type) from these.
            let (texts, files): (Vec<_>, Vec<_>) = parts
                .iter()
                .partition(|part| matches!(part.content, PartContent::Text(_)));
            if !texts.is_empty() {
                code.push_str("    data={\n");
                for part in texts {
                    if let PartContent::Text(text) = &part.content {
                        let _ =
                            writeln!(code, "        {}: {},", literal(&part.name), literal(text));
                    }
                }
                code.push_str("    },\n");
            }
            if !files.is_empty() {
                code.push_str("    files={\n");
                for part in files {
                    if let PartContent::File(path) = &part.content {
                        let _ = writeln!(code, "        {}: {},", literal(&part.name), open(path));
                    }
                }
                code.push_str("    },\n");
            }
        }
    }
    if let Some(digest) = digest {
        let _ = writeln!(
            code,
            "    auth=HTTPDigestAuth({}, {}),",
            literal(&digest.username),
            literal(&digest.password)
        );
    }
    if !request.transport.follow_redirects {
        code.push_str("    allow_redirects=False,\n");
    }
    if !request.verify_tls {
        code.push_str("    verify=False,  # certificate check off (test servers only)\n");
    }
    let _ = writeln!(
        code,
        "    timeout={},",
        request.transport.timeout.as_secs().max(1)
    );
    code.push_str(")\nprint(response.status_code, response.text)\n");
    code
}

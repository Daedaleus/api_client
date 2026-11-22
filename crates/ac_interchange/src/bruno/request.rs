//! One Bruno request file, and the settings of folders and the collection.

use std::path::Path;

use ac_domain::auth::{Auth, KeyPlacement, OAuth2, OAuth2Grant};
use ac_domain::collection::{Body, FormField, HeaderEntry, MultipartField, Request};
use ac_domain::environment::Variable;
use ac_domain::protocol::HttpMethod;
use ac_domain::protocol::grant_type::{AUTHORIZATION_CODE, CLIENT_CREDENTIALS};
use ac_domain::script::Scripts;

use super::bru::{Block, find};
use super::{seq_of, skipped};
use crate::import::{Skipped, SkippedKind, loopback_redirect};

/// Method blocks of a request file.
const METHODS: [&str; 9] = [
    "get", "post", "put", "patch", "delete", "options", "head", "connect", "trace",
];

/// The script blocks; `tests` and the translated `assert` block run after
/// the response, after `script:post-response`.
const PRE_REQUEST: &str = "script:pre-request";
const POST_RESPONSE: &str = "script:post-response";
const TESTS: &str = "tests";
const ASSERT: &str = "assert";

/// What a folder or the collection passes on.
pub(super) struct Defaults {
    pub variables: Vec<Variable>,
    pub auth: Auth,
    pub scripts: Scripts,
}

/// Variables and auth of `collection.bru` or `folder.bru`.
pub(super) fn defaults(blocks: &[Block], item: &str, report: &mut Vec<Skipped>) -> Defaults {
    let scripts = scripts(blocks, item, report);
    if let Some(headers) = find(blocks, "headers")
        && !headers.entries().is_empty()
    {
        report.push(skipped(item, SkippedKind::Headers));
    }
    let mode = find(blocks, "auth").and_then(|block| block.get("mode"));
    Defaults {
        variables: variables(blocks),
        auth: auth(blocks, mode.as_deref(), item, report),
        scripts,
    }
}

/// The request in a request file at `path`, with its `seq`.
pub(super) fn request(
    path: &str,
    blocks: &[Block],
    root: &Path,
    report: &mut Vec<Skipped>,
) -> (u32, Request) {
    let meta = find(blocks, "meta");
    let stem = path
        .rsplit('/')
        .next()
        .unwrap_or(path)
        .trim_end_matches(".bru");
    let name = meta
        .and_then(|meta| meta.get("name"))
        .unwrap_or_else(|| stem.to_owned());
    let mut request = Request::new(name);
    request.scripts = scripts(blocks, &request.name, report);
    if meta.and_then(|meta| meta.get("type")).as_deref() == Some("graphql") {
        report.push(skipped(&request.name, SkippedKind::Body("graphql".into())));
    }
    let Some(method_block) = METHODS.iter().find_map(|method| find(blocks, method)) else {
        return (seq_of(meta), request);
    };
    request.method = method_block
        .name
        .to_uppercase()
        .parse()
        .unwrap_or(HttpMethod::Get);
    let (url, path_variables) = path_parameters(&method_block.get("url").unwrap_or_default());
    request.url = url;
    request.variables = path_variables;
    if let Some(params) = find(blocks, "params:path") {
        for entry in params.entries() {
            if let Some(variable) = request.variables.iter_mut().find(|v| v.name == entry.key) {
                variable.value = entry.value;
            }
        }
    }
    request.variables.extend(variables(blocks));
    request.headers = find(blocks, "headers").map_or_default(|headers| {
        headers
            .entries()
            .into_iter()
            .map(|entry| HeaderEntry {
                name: entry.key,
                value: entry.value,
                enabled: entry.enabled,
            })
            .collect()
    });
    let body_mode = method_block.get("body").unwrap_or_default();
    request.body = body(blocks, &body_mode, root, &request.name, report);
    let auth_mode = method_block.get("auth");
    request.auth = auth(blocks, auth_mode.as_deref(), &request.name, report);
    (seq_of(meta), request)
}

/// Bruno's `:param` path parameters as `{{param}}` with a variable each.
fn path_parameters(url: &str) -> (String, Vec<Variable>) {
    let (path, query) = url
        .split_once('?')
        .map_or((url, None), |(p, q)| (p, Some(q)));
    let mut variables = Vec::new();
    let path: Vec<String> = path
        .split('/')
        .map(|segment| match segment.strip_prefix(':') {
            Some(name) if !name.is_empty() => {
                variables.push(Variable::plain(name, ""));
                format!("{{{{{name}}}}}")
            }
            _ => segment.to_owned(),
        })
        .collect();
    let mut url = path.join("/");
    if let Some(query) = query {
        url.push('?');
        url.push_str(query);
    }
    (url, variables)
}

fn variables(blocks: &[Block]) -> Vec<Variable> {
    find(blocks, "vars:pre-request").map_or_default(|vars| {
        vars.entries()
            .into_iter()
            .map(|entry| Variable {
                enabled: entry.enabled,
                ..Variable::plain(entry.key, entry.value)
            })
            .collect()
    })
}

fn body(blocks: &[Block], mode: &str, root: &Path, item: &str, report: &mut Vec<Skipped>) -> Body {
    let text = |name: &str| find(blocks, name).map_or_default(Block::text);
    match mode {
        "" | "none" => Body::None,
        "json" => Body::Json(text("body:json")),
        "text" => Body::Text(text("body:text")),
        "xml" => Body::Text(text("body:xml")),
        "formUrlEncoded" => {
            Body::Form(find(blocks, "body:form-urlencoded").map_or_default(|form| {
                form.entries()
                    .into_iter()
                    .map(|entry| FormField {
                        name: entry.key,
                        value: entry.value,
                        enabled: entry.enabled,
                    })
                    .collect()
            }))
        }
        "multipartForm" => {
            Body::Multipart(find(blocks, "body:multipart-form").map_or_default(|form| {
                form.entries()
                    .into_iter()
                    .map(|entry| {
                        let file = file_reference(&entry.value);
                        MultipartField {
                            name: entry.key,
                            file: file.is_some(),
                            value: file.map_or(entry.value, |file| absolute(root, &file)),
                            enabled: entry.enabled,
                        }
                    })
                    .collect()
            }))
        }
        "file" => {
            let file = find(blocks, "body:file")
                .and_then(|block| {
                    block
                        .entries()
                        .into_iter()
                        .find(|entry| entry.enabled)
                        .and_then(|entry| file_reference(&entry.value))
                })
                .map_or_default(|file| absolute(root, &file));
            Body::Binary(file)
        }
        other => {
            report.push(skipped(item, SkippedKind::Body(other.to_owned())));
            Body::None
        }
    }
}

/// The path in `@file(path)` (several paths: the first).
fn file_reference(value: &str) -> Option<String> {
    let inner = value.strip_prefix("@file(")?;
    let end = inner.find(')')?;
    inner[..end].split('|').next().map(str::to_owned)
}

/// `path` as an absolute path (relative ones were relative to `root`).
fn absolute(root: &Path, path: &str) -> String {
    let candidate = Path::new(path);
    if candidate.is_absolute() {
        path.to_owned()
    } else {
        root.join(candidate).display().to_string()
    }
}

fn auth(blocks: &[Block], mode: Option<&str>, item: &str, report: &mut Vec<Skipped>) -> Auth {
    let field = |block: &str, key: &str| {
        find(blocks, block)
            .and_then(|block| block.get(key))
            .unwrap_or_default()
    };
    match mode {
        None | Some("inherit") => Auth::Inherit,
        Some("none") => Auth::None,
        Some("basic") => Auth::Basic {
            username: field("auth:basic", "username"),
            password: field("auth:basic", "password"),
        },
        Some("bearer") => Auth::Bearer {
            token: field("auth:bearer", "token"),
        },
        Some("oauth2") if field("auth:oauth2", "grant_type") == CLIENT_CREDENTIALS => {
            Auth::OAuth2(OAuth2 {
                token_url: field("auth:oauth2", "access_token_url"),
                client_id: field("auth:oauth2", "client_id"),
                client_secret: field("auth:oauth2", "client_secret"),
                scope: field("auth:oauth2", "scope"),
                ..Default::default()
            })
        }
        Some("oauth2") if field("auth:oauth2", "grant_type") == AUTHORIZATION_CODE => {
            Auth::OAuth2(OAuth2 {
                grant: OAuth2Grant::AuthorizationCode,
                token_url: field("auth:oauth2", "access_token_url"),
                auth_url: field("auth:oauth2", "authorization_url"),
                redirect_url: loopback_redirect(&field("auth:oauth2", "callback_url")),
                client_id: field("auth:oauth2", "client_id"),
                client_secret: field("auth:oauth2", "client_secret"),
                scope: field("auth:oauth2", "scope"),
            })
        }
        Some("apikey") => Auth::ApiKey {
            name: field("auth:apikey", "key"),
            value: field("auth:apikey", "value"),
            placement: if field("auth:apikey", "placement") == "queryparams" {
                KeyPlacement::Query
            } else {
                KeyPlacement::Header
            },
        },
        Some(other) => {
            report.push(skipped(item, SkippedKind::Auth(other.to_owned())));
            Auth::Inherit
        }
    }
}

/// The scripts as written – they run with the Bruno names of the script
/// API (`bru`, `req.getUrl()`, `res.getBody()`, chai-style `expect`) – plus
/// the `assert` block as tests.
fn scripts(blocks: &[Block], item: &str, report: &mut Vec<Skipped>) -> Scripts {
    let code = |name: &str| {
        find(blocks, name)
            .map(|block| block.lines.join("\n").trim().to_owned())
            .filter(|code| !code.is_empty())
    };
    let asserts = find(blocks, ASSERT)
        .map(|block| super::asserts::tests(block, item, report).join("\n"))
        .filter(|code| !code.is_empty());
    let post: Vec<String> = [code(POST_RESPONSE), code(TESTS), asserts]
        .into_iter()
        .flatten()
        .collect();
    Scripts {
        pre_request: code(PRE_REQUEST).unwrap_or_default(),
        post_response: post.join("\n\n"),
    }
}

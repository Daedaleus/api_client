//! One Insomnia request (and the auth and scripts of request groups).

use ac_domain::auth::{Auth, AwsSigV4, KeyPlacement, OAuth2, OAuth2Grant};
use ac_domain::collection::{Body, FormField, HeaderEntry, MultipartField, Request};
use ac_domain::params::QueryParam;
use ac_domain::protocol::grant_type::{AUTHORIZATION_CODE, CLIENT_CREDENTIALS};
use ac_domain::script::Scripts;
use ac_domain::transport::{Cookies, Redirects};
use serde_json::Value;

use super::convert_template;
use crate::import::{Skipped, SkippedKind, loopback_redirect, text_of};

/// Body types (`body.mimeType`) with their own shape.
mod mime {
    pub const FORM: &str = "application/x-www-form-urlencoded";
    pub const MULTIPART: &str = "multipart/form-data";
    pub const FILE: &str = "application/octet-stream";
    pub const GRAPHQL: &str = "application/graphql";
}

/// `settingFollowRedirects` values.
const REDIRECTS_ON: &str = "on";
const REDIRECTS_OFF: &str = "off";
/// Settings that switch cookies off when `false`.
const SEND_COOKIES: &str = "settingSendCookies";
const STORE_COOKIES: &str = "settingStoreCookies";

/// Collects the template tags met while converting one item's texts.
struct Templates<'a> {
    item: &'a str,
    tags: Vec<String>,
}

impl Templates<'_> {
    fn text(&mut self, value: Option<&Value>) -> String {
        let (text, tags) = convert_template(&text_of(value).unwrap_or_default());
        for tag in tags {
            if !self.tags.contains(&tag) {
                self.tags.push(tag);
            }
        }
        text
    }

    fn report(self, skipped: &mut Vec<Skipped>) {
        skipped.extend(self.tags.into_iter().map(|tag| Skipped {
            item: self.item.to_owned(),
            what: SkippedKind::TemplateTag(tag),
        }));
    }
}

pub(super) fn request_of(resource: &Value, skipped: &mut Vec<Skipped>) -> Request {
    let name = text_of(resource.get("name")).unwrap_or_default();
    let mut request = Request::new(name.clone());
    let mut templates = Templates {
        item: &name,
        tags: Vec::new(),
    };
    if let Some(method) = text_of(resource.get("method")).and_then(|m| m.parse().ok()) {
        request.method = method;
    }
    let mut url = templates.text(resource.get("url"));
    // Query parameters are kept apart from the URL in Insomnia.
    for row in rows(resource.get("parameters")) {
        let (name, value) = (
            templates.text(row.get("name")),
            templates.text(row.get("value")),
        );
        if disabled(row) {
            request.disabled_query.push(QueryParam { name, value });
        } else {
            url.push(if url.contains('?') { '&' } else { '?' });
            url.push_str(&name);
            url.push('=');
            url.push_str(&value);
        }
    }
    request.url = url;
    request.headers = rows(resource.get("headers"))
        .map(|row| HeaderEntry {
            name: templates.text(row.get("name")),
            value: templates.text(row.get("value")),
            enabled: !disabled(row),
        })
        .collect();
    request.body = body_of(resource.get("body"), &mut templates, skipped);
    request.transport.redirects = match text_of(resource.get("settingFollowRedirects")).as_deref() {
        Some(REDIRECTS_ON) => Redirects::Follow,
        Some(REDIRECTS_OFF) => Redirects::Stop,
        _ => Redirects::Inherit,
    };
    // Insomnia sends and keeps cookies unless switched off.
    if [SEND_COOKIES, STORE_COOKIES]
        .iter()
        .any(|setting| resource.get(*setting).and_then(Value::as_bool) == Some(false))
    {
        request.transport.cookies = Cookies::Off;
    }
    request.auth = auth_with(resource.get("authentication"), &mut templates, skipped);
    request.scripts = scripts(resource);
    templates.report(skipped);
    request
}

fn body_of(body: Option<&Value>, templates: &mut Templates, skipped: &mut Vec<Skipped>) -> Body {
    let Some(body) = body.filter(|body| body.as_object().is_some_and(|b| !b.is_empty())) else {
        return Body::None;
    };
    let mime = text_of(body.get("mimeType")).unwrap_or_default();
    match mime.as_str() {
        mime::FORM => Body::Form(
            rows(body.get("params"))
                .map(|row| FormField {
                    name: templates.text(row.get("name")),
                    value: templates.text(row.get("value")),
                    enabled: !disabled(row),
                })
                .collect(),
        ),
        mime::MULTIPART => Body::Multipart(
            rows(body.get("params"))
                .map(|row| {
                    let file = row.get("type").and_then(Value::as_str) == Some("file");
                    MultipartField {
                        name: templates.text(row.get("name")),
                        value: if file {
                            text_of(row.get("fileName")).unwrap_or_default()
                        } else {
                            templates.text(row.get("value"))
                        },
                        enabled: !disabled(row),
                        file,
                    }
                })
                .collect(),
        ),
        mime::FILE => Body::Binary(text_of(body.get("fileName")).unwrap_or_default()),
        mime::GRAPHQL => {
            skipped.push(Skipped {
                item: templates.item.to_owned(),
                what: SkippedKind::Body("graphql".to_owned()),
            });
            Body::None
        }
        _ => {
            let text = templates.text(body.get("text"));
            if text.is_empty() {
                Body::None
            } else if mime.contains("json") {
                Body::Json(text)
            } else {
                Body::Text(text)
            }
        }
    }
}

/// The auth of a request group.
pub(super) fn auth(auth: Option<&Value>, item: &str, skipped: &mut Vec<Skipped>) -> Auth {
    let mut templates = Templates {
        item,
        tags: Vec::new(),
    };
    let converted = auth_with(auth, &mut templates, skipped);
    templates.report(skipped);
    converted
}

fn auth_with(auth: Option<&Value>, templates: &mut Templates, skipped: &mut Vec<Skipped>) -> Auth {
    let Some(auth) = auth.filter(|auth| auth.get("type").is_some()) else {
        return Auth::Inherit;
    };
    if auth.get("disabled").and_then(Value::as_bool) == Some(true) {
        return Auth::None;
    }
    let kind = text_of(auth.get("type")).unwrap_or_default();
    let mut field = |name: &str| templates.text(auth.get(name));
    match kind.as_str() {
        "none" => Auth::None,
        "basic" => Auth::Basic {
            username: field("username"),
            password: field("password"),
        },
        "digest" => Auth::Digest {
            username: field("username"),
            password: field("password"),
        },
        "iam" => Auth::AwsSigV4(AwsSigV4 {
            access_key_id: field("accessKeyId"),
            secret_access_key: field("secretAccessKey"),
            session_token: field("sessionToken"),
            region: field("region"),
            service: field("service"),
        }),
        "bearer" => Auth::Bearer {
            token: field("token"),
        },
        "apikey" => Auth::ApiKey {
            name: field("key"),
            value: field("value"),
            placement: if field("addTo") == "queryParams" {
                KeyPlacement::Query
            } else {
                KeyPlacement::Header
            },
        },
        "oauth2" if field("grantType") == CLIENT_CREDENTIALS => Auth::OAuth2(OAuth2 {
            token_url: field("accessTokenUrl"),
            client_id: field("clientId"),
            client_secret: field("clientSecret"),
            scope: field("scope"),
            ..Default::default()
        }),
        "oauth2" if field("grantType") == AUTHORIZATION_CODE => Auth::OAuth2(OAuth2 {
            grant: OAuth2Grant::AuthorizationCode,
            token_url: field("accessTokenUrl"),
            auth_url: field("authorizationUrl"),
            redirect_url: loopback_redirect(&field("redirectUrl")),
            client_id: field("clientId"),
            client_secret: field("clientSecret"),
            scope: field("scope"),
        }),
        other => {
            skipped.push(Skipped {
                item: templates.item.to_owned(),
                what: SkippedKind::Auth(other.to_owned()),
            });
            Auth::Inherit
        }
    }
}

/// Scripts as written (`insomnia` is the same object as Postman's `pm`).
pub(super) fn scripts(resource: &Value) -> Scripts {
    let code =
        |name: &str| text_of(resource.get(name)).map_or_default(|code| code.trim().to_owned());
    Scripts {
        pre_request: code("preRequestScript"),
        post_response: code("afterResponseScript"),
    }
}

fn rows(rows: Option<&Value>) -> impl Iterator<Item = &Value> {
    rows.and_then(Value::as_array).into_iter().flatten()
}

fn disabled(row: &Value) -> bool {
    row.get("disabled").and_then(Value::as_bool) == Some(true)
}

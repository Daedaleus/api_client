//! Authentication tables in request, folder and collection files:
//!
//! ```toml
//! [request.auth]            # [auth] in folder.toml and collection.toml
//! type = "bearer"           # none | basic | bearer | api-key | oauth2
//! token = "{{token}}"
//! ```
//!
//! No table means "inherit". `basic` has `username` and `password`,
//! `bearer` has `token`, `api-key` has `name`, `value` and
//! `in` (`header` or `query`), `oauth2` has `grant`
//! (`client-credentials`, the default, or `authorization-code`),
//! `token_url`, `client_id`, `client_secret`, optionally `scope` and, for
//! the authorization code, `auth_url` and optionally `redirect_url`.

use std::path::Path;

use ac_domain::auth::{Auth, KeyPlacement, OAuth2, OAuth2Grant};
use toml_edit::{Item, Table, TableLike};

use crate::document;
use crate::error::StorageError;

/// Key of the table.
pub(crate) const KEY: &str = "auth";

const TYPE: &str = "type";
const NONE: &str = "none";
const BASIC: &str = "basic";
const BEARER: &str = "bearer";
const API_KEY: &str = "api-key";
const OAUTH2: &str = "oauth2";
const GRANT: &str = "grant";
const CLIENT_CREDENTIALS: &str = "client-credentials";
const AUTHORIZATION_CODE: &str = "authorization-code";
const IN: &str = "in";
const HEADER: &str = "header";
const QUERY: &str = "query";

/// Reads the authentication in `item` (missing = inherit); `field` names the
/// table in error messages.
pub(crate) fn read(
    item: Option<&Item>,
    path: &Path,
    field: &'static str,
) -> Result<Auth, StorageError> {
    let Some(item) = item else {
        return Ok(Auth::Inherit);
    };
    let table = item
        .as_table_like()
        .ok_or_else(|| StorageError::invalid(path, field, "must be a table"))?;
    let text = |key| -> Result<String, StorageError> {
        Ok(document::opt_str(table, key, path)?.unwrap_or_default())
    };
    match document::opt_str(table, TYPE, path)?.as_deref() {
        Some(NONE) => Ok(Auth::None),
        Some(BASIC) => Ok(Auth::Basic {
            username: text("username")?,
            password: text("password")?,
        }),
        Some(BEARER) => Ok(Auth::Bearer {
            token: text("token")?,
        }),
        Some(OAUTH2) => Ok(Auth::OAuth2(OAuth2 {
            grant: match document::opt_str(table, GRANT, path)?.as_deref() {
                None | Some(CLIENT_CREDENTIALS) => OAuth2Grant::ClientCredentials,
                Some(AUTHORIZATION_CODE) => OAuth2Grant::AuthorizationCode,
                Some(other) => {
                    return Err(StorageError::invalid(
                        path,
                        field,
                        format!(
                            "`grant` must be \"{CLIENT_CREDENTIALS}\" or \"{AUTHORIZATION_CODE}\", found {other:?}"
                        ),
                    ));
                }
            },
            token_url: text("token_url")?,
            auth_url: text("auth_url")?,
            redirect_url: text("redirect_url")?,
            client_id: text("client_id")?,
            client_secret: text("client_secret")?,
            scope: text("scope")?,
        })),
        Some(API_KEY) => Ok(Auth::ApiKey {
            name: text("name")?,
            value: text("value")?,
            placement: match document::opt_str(table, IN, path)?.as_deref() {
                None | Some(HEADER) => KeyPlacement::Header,
                Some(QUERY) => KeyPlacement::Query,
                Some(other) => {
                    return Err(StorageError::invalid(
                        path,
                        field,
                        format!("`in` must be \"header\" or \"query\", found {other:?}"),
                    ));
                }
            },
        }),
        other => Err(StorageError::invalid(
            path,
            field,
            format!("type must be none, basic, bearer, api-key or oauth2, found {other:?}"),
        )),
    }
}

/// Replaces the authentication table under [`KEY`] in `table` if it changed.
pub(crate) fn write(table: &mut dyn TableLike, before: &Auth, after: &Auth) {
    if before == after {
        return;
    }
    table.remove(KEY);
    if let Some(new) = to_table(after) {
        table.insert(KEY, Item::Table(new));
    }
}

fn to_table(auth: &Auth) -> Option<Table> {
    let mut table = Table::new();
    let mut set = |key: &str, value: &str| {
        table.insert(key, toml_edit::value(value));
    };
    match auth {
        Auth::Inherit => return None,
        Auth::None => set(TYPE, NONE),
        Auth::Basic { username, password } => {
            set(TYPE, BASIC);
            set("username", username);
            set("password", password);
        }
        Auth::Bearer { token } => {
            set(TYPE, BEARER);
            set("token", token);
        }
        Auth::OAuth2(oauth) => {
            set(TYPE, OAUTH2);
            match oauth.grant {
                OAuth2Grant::ClientCredentials => set(GRANT, CLIENT_CREDENTIALS),
                OAuth2Grant::AuthorizationCode => {
                    set(GRANT, AUTHORIZATION_CODE);
                    set("auth_url", &oauth.auth_url);
                    if !oauth.redirect_url.is_empty() {
                        set("redirect_url", &oauth.redirect_url);
                    }
                }
            }
            set("token_url", &oauth.token_url);
            set("client_id", &oauth.client_id);
            set("client_secret", &oauth.client_secret);
            if !oauth.scope.is_empty() {
                set("scope", &oauth.scope);
            }
        }
        Auth::ApiKey {
            name,
            value,
            placement,
        } => {
            set(TYPE, API_KEY);
            set("name", name);
            set("value", value);
            set(
                IN,
                match placement {
                    KeyPlacement::Header => HEADER,
                    KeyPlacement::Query => QUERY,
                },
            );
        }
    }
    Some(table)
}

#[cfg(test)]
#[path = "tests/auth.rs"]
mod tests;

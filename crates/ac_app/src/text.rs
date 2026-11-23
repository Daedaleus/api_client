//! Display texts as translation keys with arguments.
//!
//! The application layer never produces translated text: it hands out a key
//! and named arguments, the UI looks the key up in `locales/app.json`. So a
//! language switch also changes messages that are already on screen.

use std::collections::BTreeMap;

use serde::Serialize;

/// A display text: translation key plus named arguments for its placeholders.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct UserText {
    /// Key in `locales/app.json`, e.g. `error.connect`.
    pub key: &'static str,
    /// Values for the `{name}` placeholders of the text.
    pub args: BTreeMap<&'static str, String>,
}

impl UserText {
    /// A text without arguments.
    #[must_use]
    pub fn new(key: &'static str) -> Self {
        Self {
            key,
            args: BTreeMap::new(),
        }
    }

    /// Adds the value for placeholder `{name}`.
    #[must_use]
    pub fn with(mut self, name: &'static str, value: impl Into<String>) -> Self {
        self.args.insert(name, value.into());
        self
    }
}

/// Every translation key the application layer uses.
pub mod keys {
    /// The URL field is empty.
    pub const URL_EMPTY: &str = "error.url_empty";
    /// The URL cannot be parsed. Argument: `reason`.
    pub const URL_INVALID: &str = "error.url_invalid";
    /// The URL scheme is not http or https. Argument: `scheme`.
    pub const URL_SCHEME: &str = "error.url_scheme";
    /// The method is not a valid token. Argument: `method`.
    pub const METHOD_INVALID: &str = "error.method_invalid";
    /// A header cannot be sent. Argument: `name`.
    pub const HEADER_INVALID: &str = "error.header_invalid";
    /// A `{{name}}` has no value. Argument: `name`.
    pub const VARIABLE_UNKNOWN: &str = "error.variable_unknown";
    /// A secret variable is used, but none of the user's keys can decrypt
    /// it. Argument: `name`.
    pub const SECRET_UNAVAILABLE: &str = "error.secret_unavailable";
    /// Recipients not changed: an environment's secrets cannot be decrypted.
    /// Argument: `environment`.
    pub const SECRET_NOT_REENCRYPTED: &str = "error.secret_not_reencrypted";
    /// None of the user's keys can decrypt the secrets.
    pub const SECRET_NO_KEY: &str = "error.secret_no_key";
    /// Nobody to encrypt the secrets for.
    pub const SECRET_NO_RECIPIENTS: &str = "error.secret_no_recipients";
    /// A line of `.age-recipients` is no public key. Argument: `recipient`.
    pub const SECRET_INVALID_RECIPIENT: &str = "error.secret_invalid_recipient";
    /// The encrypted secrets are damaged. Argument: `detail`.
    pub const SECRET_DAMAGED: &str = "error.secret_damaged";
    /// The user's own key cannot be read or created. Argument: `detail`.
    pub const SECRET_KEY: &str = "error.secret_key";
    /// No answer in time.
    pub const TIMEOUT: &str = "error.timeout";
    /// No connection. Argument: `detail`.
    pub const CONNECT: &str = "error.connect";
    /// A file of the body cannot be read. Arguments: `path`, `detail`.
    pub const FILE_UNREADABLE: &str = "error.file_unreadable";
    /// A relative file path in a request that belongs to no collection.
    /// Argument: `path`.
    pub const FILE_RELATIVE: &str = "error.file_relative";
    /// A pasted text is no usable cURL command. Argument: `detail`.
    pub const CURL_INVALID: &str = "error.curl_invalid";
    /// A file cannot be imported. Argument: `detail`.
    pub const IMPORT_INVALID: &str = "error.import_invalid";
    /// An import left out a declarative assertion. Arguments: `item`, `assertion`.
    pub const IMPORT_SKIPPED_ASSERTION: &str = "import.skipped_assertion";
    /// An import left out an auth type. Arguments: `item`, `kind`.
    pub const IMPORT_SKIPPED_AUTH: &str = "import.skipped_auth";
    /// An import left out headers of a folder or the collection. Argument:
    /// `item`.
    pub const IMPORT_SKIPPED_HEADERS: &str = "import.skipped_headers";
    /// An import left out a body. Arguments: `item`, `mode`.
    pub const IMPORT_SKIPPED_BODY: &str = "import.skipped_body";
    /// An export left out a secret value. Arguments: `item`, `name`.
    pub const EXPORT_SECRET_VALUE: &str = "export.secret_value";
    /// An export has no place for a time limit. Argument: `item`.
    pub const EXPORT_TIMEOUT: &str = "export.timeout";
    /// An export has no place for the redirect setting. Argument: `item`.
    pub const EXPORT_REDIRECTS: &str = "export.redirects";
    /// An export has no place for the certificate check. Argument: `item`.
    pub const EXPORT_CERTIFICATE_CHECK: &str = "export.certificate_check";
    /// An export has no place for the cookie setting. Argument: `item`.
    pub const EXPORT_COOKIES: &str = "export.cookies";
    /// Recorded entries that are no API calls (HAR). Arguments: `item`,
    /// `kind`, `count`.
    pub const IMPORT_SKIPPED_RESOURCES: &str = "import.skipped_resources";
    /// An import left out a request kind (gRPC …). Arguments: `item`, `kind`.
    pub const IMPORT_SKIPPED_REQUEST_KIND: &str = "import.skipped_request_kind";
    /// A template tag stayed as text (Insomnia). Arguments: `item`, `tag`.
    pub const IMPORT_TEMPLATE_TAG: &str = "import.template_tag";
    /// A relative `OpenAPI` server. Placeholders: `item`, `url`.
    pub const IMPORT_RELATIVE_SERVER: &str = "import.relative_server";
    /// An import from a URL got nothing usable. Placeholder: `detail`.
    pub const IMPORT_DOWNLOAD: &str = "error.import_download";
    /// Any other transport failure. Argument: `detail`.
    pub const REQUEST: &str = "error.request";
    /// No OAuth 2 access token. Placeholder: `detail`.
    pub const OAUTH_TOKEN: &str = "error.oauth_token";
    /// Nobody signed in in the browser in time.
    pub const OAUTH_SIGN_IN_TIMEOUT: &str = "error.oauth_sign_in_timeout";
    /// Redirect URL not on the loopback address. Placeholder: `url`.
    pub const OAUTH_REDIRECT: &str = "error.oauth_redirect";
    /// The pre-request script failed. Placeholder: `detail`.
    pub const SCRIPT_PRE_REQUEST: &str = "error.script_pre_request";
    /// The post-response script failed. Placeholder: `detail`.
    pub const SCRIPT_POST_RESPONSE: &str = "error.script_post_response";
    /// The pre-request script of a folder or the collection failed.
    /// Placeholders: `owner`, `detail`.
    pub const SCRIPT_PRE_REQUEST_OF: &str = "error.script_pre_request_of";
    /// The post-response script of a folder or the collection failed.
    /// Placeholders: `owner`, `detail`.
    pub const SCRIPT_POST_RESPONSE_OF: &str = "error.script_post_response_of";
    /// Checking or completing a script failed. Placeholder: `detail`.
    pub const SCRIPT_ASSIST: &str = "error.script_assist";
    /// The request history cannot be read or written. Placeholder: `detail`.
    pub const HISTORY: &str = "error.history";
    /// The app's settings cannot be read, written or applied. Placeholder:
    /// `detail`.
    pub const SETTINGS: &str = "error.settings";

    /// A file or folder cannot be read or written. Arguments: `path`, `detail`.
    pub const STORE_IO: &str = "error.store_io";
    /// A file is damaged. Arguments: `path`, `detail`.
    pub const STORE_DAMAGED: &str = "error.store_damaged";
    /// A file comes from a newer version and is not written. Argument: `path`.
    pub const STORE_READ_ONLY: &str = "error.store_read_only";
    /// The folder is no collection. Argument: `path`.
    pub const NOT_A_COLLECTION: &str = "error.not_a_collection";
    /// A collection already exists there. Argument: `path`.
    pub const COLLECTION_EXISTS: &str = "error.collection_exists";
    /// No environment of this name. Argument: `name`.
    pub const UNKNOWN_ENVIRONMENT: &str = "error.unknown_environment";
    /// Nothing matches what should be run. Argument: `path`.
    pub const NOTHING_TO_RUN: &str = "error.nothing_to_run";
    /// The collection is not open (any more).
    pub const UNKNOWN_COLLECTION: &str = "error.unknown_collection";
    /// "Save as" for a saved collection.
    pub const ALREADY_SAVED: &str = "error.already_saved";
    /// The item does not exist (any more), e.g. changed outside the app.
    pub const UNKNOWN_ITEM: &str = "error.unknown_item";
    /// New items can only be created in the collection or a folder.
    pub const NOT_A_FOLDER: &str = "error.not_a_folder";
    /// A folder cannot go into itself or one of its folders.
    pub const MOVE_INTO_ITSELF: &str = "error.move_into_itself";
    /// Names must not be empty.
    pub const NAME_EMPTY: &str = "error.name_empty";
    /// A cookie name is empty or has spaces, `;` or `=`.
    pub const COOKIE_NAME: &str = "error.cookie_name";
    /// A cookie value has `;` or control characters.
    pub const COOKIE_VALUE: &str = "error.cookie_value";
    /// A cookie domain is empty or has spaces or `/`.
    pub const COOKIE_DOMAIN: &str = "error.cookie_domain";
    /// A cookie path does not start with `/`.
    pub const COOKIE_PATH: &str = "error.cookie_path";

    /// All keys with the names of their placeholders, checked against the
    /// translation file by a test.
    pub const ALL: [(&str, &[&str]); 61] = [
        (URL_EMPTY, &[]),
        (URL_INVALID, &["reason"]),
        (URL_SCHEME, &["scheme"]),
        (METHOD_INVALID, &["method"]),
        (HEADER_INVALID, &["name"]),
        (VARIABLE_UNKNOWN, &["name"]),
        (SECRET_UNAVAILABLE, &["name"]),
        (SECRET_NOT_REENCRYPTED, &["environment"]),
        (SECRET_NO_KEY, &[]),
        (SECRET_NO_RECIPIENTS, &[]),
        (SECRET_INVALID_RECIPIENT, &["recipient"]),
        (SECRET_DAMAGED, &["detail"]),
        (SECRET_KEY, &["detail"]),
        (TIMEOUT, &[]),
        (CONNECT, &["detail"]),
        (FILE_UNREADABLE, &["path", "detail"]),
        (FILE_RELATIVE, &["path"]),
        (CURL_INVALID, &["detail"]),
        (IMPORT_INVALID, &["detail"]),
        (IMPORT_SKIPPED_ASSERTION, &["item", "assertion"]),
        (IMPORT_SKIPPED_AUTH, &["item", "kind"]),
        (IMPORT_SKIPPED_BODY, &["item", "mode"]),
        (IMPORT_RELATIVE_SERVER, &["item", "url"]),
        (EXPORT_SECRET_VALUE, &["item", "name"]),
        (EXPORT_TIMEOUT, &["item"]),
        (EXPORT_REDIRECTS, &["item"]),
        (EXPORT_CERTIFICATE_CHECK, &["item"]),
        (EXPORT_COOKIES, &["item"]),
        (IMPORT_SKIPPED_RESOURCES, &["item", "kind", "count"]),
        (IMPORT_SKIPPED_REQUEST_KIND, &["item", "kind"]),
        (IMPORT_TEMPLATE_TAG, &["item", "tag"]),
        (IMPORT_DOWNLOAD, &["detail"]),
        (IMPORT_SKIPPED_HEADERS, &["item"]),
        (REQUEST, &["detail"]),
        (OAUTH_TOKEN, &["detail"]),
        (OAUTH_SIGN_IN_TIMEOUT, &[]),
        (OAUTH_REDIRECT, &["url"]),
        (SCRIPT_PRE_REQUEST, &["detail"]),
        (SCRIPT_POST_RESPONSE, &["detail"]),
        (SCRIPT_PRE_REQUEST_OF, &["owner", "detail"]),
        (SCRIPT_POST_RESPONSE_OF, &["owner", "detail"]),
        (SCRIPT_ASSIST, &["detail"]),
        (HISTORY, &["detail"]),
        (SETTINGS, &["detail"]),
        (STORE_IO, &["path", "detail"]),
        (STORE_DAMAGED, &["path", "detail"]),
        (STORE_READ_ONLY, &["path"]),
        (NOT_A_COLLECTION, &["path"]),
        (COLLECTION_EXISTS, &["path"]),
        (UNKNOWN_ENVIRONMENT, &["name"]),
        (NOTHING_TO_RUN, &["path"]),
        (UNKNOWN_COLLECTION, &[]),
        (ALREADY_SAVED, &[]),
        (UNKNOWN_ITEM, &[]),
        (NOT_A_FOLDER, &[]),
        (MOVE_INTO_ITSELF, &[]),
        (NAME_EMPTY, &[]),
        (COOKIE_NAME, &[]),
        (COOKIE_VALUE, &[]),
        (COOKIE_DOMAIN, &[]),
        (COOKIE_PATH, &[]),
    ];
}

#[cfg(test)]
#[path = "tests/text.rs"]
mod tests;

//! Languages of the code snippets the editor copies, and formats a
//! collection can be exported to.

use ac_interchange::snippet::SnippetLanguage;
use serde::{Deserialize, Serialize};

/// A language for "copy as code" (serialized in camelCase).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub enum SnippetLanguageView {
    /// cURL.
    Curl,
    /// JavaScript `fetch`.
    Fetch,
    /// JavaScript axios.
    Axios,
    /// Python `requests`.
    Python,
    /// Go `net/http`.
    Go,
    /// `HTTPie`.
    Httpie,
}

impl SnippetLanguageView {
    /// The language of the generator.
    #[must_use]
    pub fn to_language(self) -> SnippetLanguage {
        match self {
            Self::Curl => SnippetLanguage::Curl,
            Self::Fetch => SnippetLanguage::JavaScriptFetch,
            Self::Axios => SnippetLanguage::JavaScriptAxios,
            Self::Python => SnippetLanguage::PythonRequests,
            Self::Go => SnippetLanguage::GoNetHttp,
            Self::Httpie => SnippetLanguage::Httpie,
        }
    }
}

/// A format a collection can be exported to (serialized in camelCase).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub enum ExportFormatView {
    /// A Postman collection (v2.1).
    Postman,
}

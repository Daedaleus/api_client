//! A request as the editor sends it – to send it, or to show it as a cURL
//! command.

use serde::{Deserialize, Serialize};

use crate::view::{
    AuthDraft, BodyDraft, CertificateCheckView, HeaderDraft, ParamDraft, ScriptsDraft, SendContext,
    TransportDraft, VariableDraft,
};

/// A request as entered in the editor, with where its variables come from.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct OutgoingRequest {
    /// Method as entered, e.g. `GET`.
    pub method: String,
    /// URL as entered; `http://` is assumed when no scheme is given.
    pub url: String,
    /// Header rows; disabled rows and rows without a name are not sent.
    #[serde(default)]
    pub headers: Vec<HeaderDraft>,
    /// Body; JSON and text get a `Content-Type` unless a header sets one.
    #[serde(default)]
    pub body: BodyDraft,
    /// The request's own variables, as in the editor (strongest).
    #[serde(default)]
    pub variables: Vec<VariableDraft>,
    /// The request's own authentication; `inherit` takes that of its
    /// folders and collection (see `context`).
    #[serde(default)]
    pub auth: AuthDraft,
    /// The request's own certificate check; `inherit` takes that of its
    /// folders and collection.
    #[serde(default)]
    pub certificate_check: CertificateCheckView,
    /// Scripts before sending and after the response.
    #[serde(default)]
    pub scripts: ScriptsDraft,
    /// The request's own redirects and time limit; `inherit` takes those of
    /// its folders and collection.
    #[serde(default)]
    pub transport: TransportDraft,
    /// Values of the path variables (`:name`) in the URL.
    #[serde(default)]
    pub path_params: Vec<ParamDraft>,
    /// Where further `{{variables}}` come from: collection, folders and
    /// environment. Without it only the request's own variables count.
    #[serde(default)]
    pub context: Option<SendContext>,
}

impl OutgoingRequest {
    /// A `GET` of `url` without anything else (tests, simple callers).
    #[must_use]
    pub fn get(url: &str) -> Self {
        Self {
            method: "GET".into(),
            url: url.into(),
            headers: Vec::new(),
            body: BodyDraft::None,
            variables: Vec::new(),
            auth: AuthDraft::Inherit,
            certificate_check: CertificateCheckView::Inherit,
            scripts: ScriptsDraft::default(),
            transport: TransportDraft::default(),
            path_params: Vec::new(),
            context: None,
        }
    }
}

//! View models: plain data for the UI, serialized as camelCase JSON.
//!
//! - [`auth`]: authentication of requests, folders and the collection.
//! - [`response`]: results of request runs.
//! - [`body`]: the body preview of a response.
//! - [`collection`]: the request tree and saved requests for editing.
//! - [`environment`]: environments and their variables.
//! - [`request_body`]: the body of a saved request.
//! - [`outgoing`]: a request as the editor sends it.
//! - [`recipients`]: who secrets are encrypted for.
//! - [`run_report`]: results of running requests one after another.
//! - [`variable`]: variable rows and where variables come from when sending.

pub mod assist;
pub mod auth;
pub mod body;
pub mod collection;
pub mod cookie;
pub mod environment;
pub mod example;
pub mod history;
pub mod outgoing;
pub mod params;
pub mod recipients;
pub mod request_body;
pub mod response;
pub mod run_report;
pub mod scripts;
pub mod settings;
pub mod snippet;
pub mod transport;
pub mod variable;

pub use assist::{
    AssistRequest, AssistResult, CompletionKindView, CompletionView, DiagnosticView, SeverityView,
};
pub use auth::{AuthDraft, CertificateCheckView, KeyPlacementView, OAuth2GrantView};
pub use body::BodyView;
pub use collection::{CollectionView, HeaderDraft, RequestDraft, TreeItemView};
pub use cookie::{CookieKeyView, CookieView};
pub use environment::EnvironmentDraft;
pub use example::ExampleView;
pub use history::HistoryItemView;
pub use outgoing::OutgoingRequest;
pub use params::ParamDraft;
pub use recipients::RecipientsView;
pub use request_body::{BodyDraft, FormFieldDraft, MultipartFieldDraft};
pub use response::{HeaderView, ResponseView, RunOutcomeView, StatusClassView, TestResultView};
pub use run_report::{RunReport, RunResult, RunResultOutcome};
pub use scripts::ScriptsDraft;
pub use settings::{ClientCertificateView, ProxyView, SettingsView};
pub use snippet::{ExportFormatView, SnippetLanguageView};
pub use transport::{CookiesView, RedirectsView, TransportDraft};
pub use variable::{SendContext, VariableDraft};

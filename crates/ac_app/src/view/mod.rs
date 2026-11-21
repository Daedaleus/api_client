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
//! - [`variable`]: variable rows and where variables come from when sending.

pub mod auth;
pub mod body;
pub mod collection;
pub mod environment;
pub mod outgoing;
pub mod recipients;
pub mod request_body;
pub mod response;
pub mod variable;

pub use auth::{AuthDraft, CertificateCheckView, KeyPlacementView};
pub use body::BodyView;
pub use collection::{CollectionView, HeaderDraft, RequestDraft, TreeItemView};
pub use environment::EnvironmentDraft;
pub use outgoing::OutgoingRequest;
pub use recipients::RecipientsView;
pub use request_body::{BodyDraft, FormFieldDraft, MultipartFieldDraft};
pub use response::{HeaderView, ResponseView, RunOutcomeView, StatusClassView};
pub use variable::{SendContext, VariableDraft};

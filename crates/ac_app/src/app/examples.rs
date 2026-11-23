//! Response examples of saved requests: kept from a run's response, listed
//! and removed. They live next to the request file (see the store).

use ac_domain::collection::Item;
use ac_domain::example::ResponseExample;

use super::App;
use super::collections::{item_id_from, writable};
use crate::command::{CommandAck, QueryResult, RunId};
use crate::error::AppError;
use crate::ports::{
    BrowserLogin, CollectionStore, HistoryStore, HttpSender, ScriptAssist, ScriptRunner,
    SecretVault, SettingsStore,
};
use crate::view::ExampleView;

impl<
    H: HttpSender,
    S: CollectionStore,
    V: SecretVault,
    B: BrowserLogin,
    R: ScriptRunner,
    A: ScriptAssist,
    Y: HistoryStore,
    Z: SettingsStore,
> App<H, S, V, B, R, A, Y, Z>
{
    pub(super) fn query_examples(
        &self,
        collection_id: &str,
        request_id: &str,
    ) -> Result<QueryResult, AppError> {
        Ok(QueryResult::Examples {
            examples: ExampleView::list(&self.examples_of(collection_id, request_id)?),
        })
    }

    /// Keeps the response of `run_id` as the example `name` (its body must
    /// be text).
    pub(super) fn save_example(
        &self,
        collection_id: &str,
        request_id: &str,
        run_id: RunId,
        name: &str,
    ) -> Result<CommandAck, AppError> {
        let name = name.trim();
        if name.is_empty() {
            return Err(AppError::EmptyName);
        }
        let response = self.kept_response(run_id)?;
        let body = String::from_utf8(response.body).map_err(|_| AppError::ExampleNotText)?;
        let mut examples = self.examples_of(collection_id, request_id)?;
        examples.push(ResponseExample {
            name: name.to_owned(),
            status: response.status,
            headers: response.headers,
            body,
        });
        self.write_examples(collection_id, request_id, &examples)
    }

    pub(super) fn delete_example(
        &self,
        collection_id: &str,
        request_id: &str,
        index: usize,
    ) -> Result<CommandAck, AppError> {
        let mut examples = self.examples_of(collection_id, request_id)?;
        if index >= examples.len() {
            return Err(AppError::UnknownItem);
        }
        examples.remove(index);
        self.write_examples(collection_id, request_id, &examples)
    }

    fn examples_of(
        &self,
        collection_id: &str,
        request_id: &str,
    ) -> Result<Vec<ResponseExample>, AppError> {
        let opened = self.current(collection_id)?;
        let id = item_id_from(request_id)?;
        match opened.collection.find(id) {
            Some(Item::Request(request)) => Ok(request.examples.clone()),
            _ => Err(AppError::UnknownItem),
        }
    }

    fn write_examples(
        &self,
        collection_id: &str,
        request_id: &str,
        examples: &[ResponseExample],
    ) -> Result<CommandAck, AppError> {
        let opened = self.current(collection_id)?;
        let file = writable(&opened, item_id_from(request_id)?)?;
        self.inner.store.save_examples(file, examples)?;
        Ok(CommandAck::ExamplesChanged {
            examples: ExampleView::list(examples),
        })
    }
}

#[cfg(test)]
#[path = "../tests/app_examples.rs"]
mod tests;

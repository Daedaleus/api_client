//! Who a collection's secrets are encrypted for (ADR S4): listing them with
//! the user's own key, and changing them, which re-encrypts the secrets of
//! every environment.

use std::path::PathBuf;

use super::App;
use super::collections::location;
use crate::command::{CommandAck, QueryResult};
use crate::error::AppError;
use crate::ports::{
    BrowserLogin, CollectionStore, HttpSender, ScriptRunner, SecretError, SecretValues, SecretVault,
};
use crate::ports::{HistoryStore, ScriptAssist};
use crate::view::RecipientsView;

impl<
    H: HttpSender,
    S: CollectionStore,
    V: SecretVault,
    B: BrowserLogin,
    R: ScriptRunner,
    A: ScriptAssist,
    Y: HistoryStore,
> App<H, S, V, B, R, A, Y>
{
    pub(super) fn query_recipients(&self, collection_id: &str) -> Result<QueryResult, AppError> {
        let opened = self.current(collection_id)?;
        Ok(QueryResult::Recipients {
            recipients: RecipientsView {
                own: self.inner.vault.own_recipient()?,
                recipients: self.inner.store.recipients(&opened.root)?,
            },
        })
    }

    /// Sets the recipients and re-encrypts every environment's secrets for
    /// them. Everything is decrypted and encrypted before anything is
    /// written, so a secret the user cannot decrypt leaves all as it was.
    pub(super) fn set_recipients(
        &self,
        collection_id: &str,
        recipients: &[String],
    ) -> Result<CommandAck, AppError> {
        let opened = self.current(collection_id)?;
        let recipients = normalized(recipients);
        if recipients.is_empty() {
            return Err(SecretError::NoRecipients.into());
        }
        for recipient in &recipients {
            self.inner.vault.check_recipient(recipient)?;
        }
        if self.inner.store.recipients(&opened.root)? == recipients {
            return self.reload(&opened.root, None);
        }
        let mut sealed: Vec<(PathBuf, SecretValues)> = Vec::new();
        for environment in &opened.environments {
            let file = &location(&opened, environment.id)?.path;
            let Some(data) = self.inner.store.read_secrets(file)? else {
                continue;
            };
            let values =
                self.inner
                    .vault
                    .decrypt(&data)
                    .map_err(|reason| AppError::NotReencrypted {
                        environment: environment.name.clone(),
                        reason,
                    })?;
            sealed.push((file.clone(), values));
        }
        let encrypted = sealed
            .iter()
            .map(|(file, values)| Ok((file, self.inner.vault.encrypt(values, &recipients)?)))
            .collect::<Result<Vec<_>, AppError>>()?;
        self.inner
            .store
            .write_recipients(&opened.root, &recipients)?;
        for (file, data) in encrypted {
            self.inner.store.write_secrets(file, Some(&data))?;
        }
        self.reload(&opened.root, None)
    }
}

/// Trimmed, without empty lines and repeats, in the order given.
fn normalized(recipients: &[String]) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for recipient in recipients.iter().map(|recipient| recipient.trim()) {
        if !recipient.is_empty() && !out.iter().any(|known| known == recipient) {
            out.push(recipient.to_owned());
        }
    }
    out
}

#[cfg(test)]
#[path = "../tests/app_recipients.rs"]
mod tests;

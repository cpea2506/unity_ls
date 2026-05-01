use crate::document_storage::DocumentStorage;
use gen_lsp_types::{
    DidChangeTextDocumentParams, DidCloseTextDocumentParams, DidOpenTextDocumentParams,
    LspNotificationMethod, TextDocumentContentChangeEvent,
};
use lsp_server::Notification;
use std::error::Error;

pub trait NotificationHandle {
    fn handle(&mut self) -> Result<(), Box<dyn Error>>;
}

pub struct UnityNotification<'a> {
    notification: &'a Notification,
    docs: &'a mut DocumentStorage,
}

impl<'a> UnityNotification<'a> {
    pub fn new(notification: &'a Notification, docs: &'a mut DocumentStorage) -> Self {
        Self { notification, docs }
    }
}

impl<'a> NotificationHandle for UnityNotification<'a> {
    fn handle(&mut self) -> Result<(), Box<dyn Error>> {
        let params = self.notification.params.clone();

        match LspNotificationMethod::from(self.notification.method.clone()) {
            LspNotificationMethod::TextDocumentDidOpen => {
                let params = serde_json::from_value::<DidOpenTextDocumentParams>(params)?;

                self.docs
                    .open(params.text_document.uri, params.text_document.text);
            }
            LspNotificationMethod::TextDocumentDidChange => {
                let params = serde_json::from_value::<DidChangeTextDocumentParams>(params)?;

                if let Some(
                    TextDocumentContentChangeEvent::TextDocumentContentChangeWholeDocument(change),
                ) = params.content_changes.into_iter().next()
                {
                    self.docs.change(
                        params.text_document.text_document_identifier.uri,
                        change.text,
                    );
                }
            }
            LspNotificationMethod::TextDocumentDidClose => {
                let params = serde_json::from_value::<DidCloseTextDocumentParams>(params)?;

                self.docs.close(&params.text_document.uri);
            }
            _ => {}
        }
        Ok(())
    }
}

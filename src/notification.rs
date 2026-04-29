use crate::document_storage::DocumentStorage;
use lsp_server::Notification;
use lsp_types::{
    DidChangeTextDocumentParams, DidCloseTextDocumentParams, DidOpenTextDocumentParams,
    notification::{
        DidChangeTextDocument, DidCloseTextDocument, DidOpenTextDocument, Notification as _,
    },
};
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
        match self.notification.method.as_str() {
            DidOpenTextDocument::METHOD => {
                let params = serde_json::from_value::<DidOpenTextDocumentParams>(
                    self.notification.params.clone(),
                )?;
                self.docs
                    .open(params.text_document.uri.clone(), params.text_document.text);
            }
            DidChangeTextDocument::METHOD => {
                let params = serde_json::from_value::<DidChangeTextDocumentParams>(
                    self.notification.params.clone(),
                )?;

                if let Some(change) = params.content_changes.into_iter().next() {
                    self.docs.change(&params.text_document.uri, change.text);
                }
            }
            DidCloseTextDocument::METHOD => {
                let params = serde_json::from_value::<DidCloseTextDocumentParams>(
                    self.notification.params.clone(),
                )?;
                self.docs.close(&params.text_document.uri);
            }
            _ => {}
        }
        Ok(())
    }
}

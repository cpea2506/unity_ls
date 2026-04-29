mod analyzer;
mod codelens;
mod document_storage;
mod notification;
mod request;

use crate::analyzer::Analyzer;
use crate::notification::NotificationHandle;
use crate::notification::UnityNotification;
use crate::request::{RequestHandle, UnityRequest};
use document_storage::DocumentStorage;
use lsp_server::{Connection, Message};
use lsp_types::{
    CodeLensOptions, InitializeParams, PositionEncodingKind, ServerCapabilities,
    TextDocumentSyncCapability, TextDocumentSyncKind,
};
use std::{error::Error, result::Result};
use tracing::{error, info};

fn main() -> Result<(), Box<dyn Error + Sync + Send>> {
    tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .init();

    info!("Starting Unity LS");

    let (connection, io_threads) = Connection::stdio();

    let server_capabilities = serde_json::to_value(ServerCapabilities {
        position_encoding: Some(PositionEncodingKind::UTF8),
        code_lens_provider: Some(CodeLensOptions {
            resolve_provider: Some(true),
        }),
        text_document_sync: Some(TextDocumentSyncCapability::Kind(
            TextDocumentSyncKind::INCREMENTAL,
        )),
        ..Default::default()
    })?;

    let params = connection.initialize(server_capabilities)?;

    main_loop(connection, params)?;
    io_threads.join()?;

    info!("Unity LS stopped");

    Ok(())
}

fn main_loop(
    connection: Connection,
    params: serde_json::Value,
) -> Result<(), Box<dyn Error + Sync + Send>> {
    let params = serde_json::from_value::<InitializeParams>(params)?;

    let mut docs = DocumentStorage::new();
    let workspace_root = params.workspace_folders.unwrap()[0].uri.clone();
    let analyzer = Analyzer::new(&workspace_root);

    for msg in &connection.receiver {
        match msg {
            Message::Request(request) => {
                if connection.handle_shutdown(&request)? {
                    break;
                }

                let unity_request = UnityRequest::new(&connection, &request, &docs, &analyzer);

                if let Err(err) = unity_request.handle() {
                    error!("[Unity LS] Request {} failed: {err}", &request.method);
                }
            }
            Message::Notification(notification) => {
                let mut unity_notification = UnityNotification::new(&notification, &mut docs);

                if let Err(err) = unity_notification.handle() {
                    error!(
                        "[Unity LS] Notification {} failed: {err}",
                        notification.method
                    );
                }
            }
            Message::Response(response) => {
                info!("Received response: {:?}", response);
            }
        }
    }

    Ok(())
}

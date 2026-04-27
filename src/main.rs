use lsp_server::{
    Connection, ErrorCode, Message, Notification, Request, RequestId, Response, ResponseError,
};
use lsp_types::notification::Notification as _;
use lsp_types::notification::{DidChangeTextDocument, DidCloseTextDocument, DidOpenTextDocument};
use lsp_types::request::Request as _;
use lsp_types::request::{CodeLensRequest, CodeLensResolve};
use lsp_types::{
    CodeLens, CodeLensOptions, CodeLensParams, DidChangeTextDocumentParams,
    DidCloseTextDocumentParams, DidOpenTextDocumentParams, InitializeParams, PositionEncodingKind,
    ServerCapabilities, TextDocumentSyncCapability, TextDocumentSyncKind,
};
use std::{error::Error, result::Result};
use tracing::{error, info};
use url::Url;

mod analyzer;
mod asset_detector;
mod codelens;
mod document_storage;

use document_storage::DocumentStorage;

use crate::analyzer::UnityAnalyzer;

fn main() -> Result<(), Box<dyn Error + Sync + Send>> {
    tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .init();

    info!("Starting Unity LSP Server");

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

    info!("Unity LSP Server stopped");

    Ok(())
}

fn main_loop(
    connection: Connection,
    params: serde_json::Value,
) -> Result<(), Box<dyn Error + Sync + Send>> {
    let params = serde_json::from_value::<InitializeParams>(params)?;

    let mut docs = DocumentStorage::new();
    let analyzer = UnityAnalyzer::new(params.workspace_folders.unwrap()[0].uri.clone());

    info!("Unity LS Analyzer {:?}", analyzer);

    for msg in &connection.receiver {
        match msg {
            Message::Request(req) => {
                if connection.handle_shutdown(&req)? {
                    break;
                }

                if let Err(err) = handle_request(&connection, &req, &mut docs, &analyzer) {
                    error!("[Unity LS] Request {} failed: {err}", &req.method);
                }
            }
            Message::Notification(noti) => {
                if let Err(err) = handle_notification(&connection, &noti, &mut docs) {
                    error!("[Unity LS] Notification {} failed: {err}", noti.method);
                }
            }
            Message::Response(resp) => {
                info!("Received response: {:?}", resp);
            }
        }
    }

    Ok(())
}

fn handle_request(
    conn: &Connection,
    req: &Request,
    docs: &mut DocumentStorage,
    analyzer: &UnityAnalyzer,
) -> Result<(), Box<dyn Error>> {
    match req.method.as_str() {
        CodeLensRequest::METHOD => {
            let p = serde_json::from_value::<CodeLensParams>(req.params.clone())?;
            let uri = p.text_document.uri;
            let content = docs.get(uri.clone()).unwrap();
            let script_path = Url::parse(uri.as_str())?.to_file_path().unwrap();
            let analysis = analyzer.analyze_script(content, &script_path);
            let codelens = codelens::create_codelens(analysis, uri)?;

            send_ok(conn, req.id.clone(), &codelens)?;
        }
        CodeLensResolve::METHOD => {
            let lens = serde_json::from_value::<CodeLens>(req.params.clone())?;
            let lens = codelens::resolve_codelens(lens)?;

            send_ok(conn, req.id.clone(), &lens)?;
        }
        _ => send_err(
            conn,
            req.id.clone(),
            ErrorCode::MethodNotFound,
            "unhandled method",
        )?,
    }

    Ok(())
}

fn handle_notification(
    _conn: &Connection,
    noti: &Notification,
    docs: &mut DocumentStorage,
) -> Result<(), Box<dyn Error>> {
    match noti.method.as_str() {
        DidOpenTextDocument::METHOD => {
            let p = serde_json::from_value::<DidOpenTextDocumentParams>(noti.params.clone())?;
            let uri = p.text_document.uri;
            docs.open(uri.clone(), p.text_document.text);
        }
        DidChangeTextDocument::METHOD => {
            let p = serde_json::from_value::<DidChangeTextDocumentParams>(noti.params.clone())?;
            if let Some(change) = p.content_changes.into_iter().next() {
                let uri = p.text_document.uri;
                docs.change(uri.clone(), change.text);
            }
        }
        DidCloseTextDocument::METHOD => {
            let p = serde_json::from_value::<DidCloseTextDocumentParams>(noti.params.clone())?;
            let uri = p.text_document.uri;
            docs.close(uri.clone());
        }
        _ => {}
    }
    Ok(())
}

fn send_ok<T: serde::Serialize>(
    conn: &Connection,
    id: RequestId,
    result: &T,
) -> Result<(), Box<dyn Error>> {
    let resp = Response {
        id,
        result: Some(serde_json::to_value(result)?),
        error: None,
    };
    conn.sender.send(Message::Response(resp))?;
    Ok(())
}

fn send_err(
    conn: &Connection,
    id: RequestId,
    code: ErrorCode,
    msg: &str,
) -> Result<(), Box<dyn Error>> {
    let resp = Response {
        id,
        result: None,
        error: Some(ResponseError {
            code: code as i32,
            message: msg.into(),
            data: None,
        }),
    };
    conn.sender.send(Message::Response(resp))?;

    Ok(())
}

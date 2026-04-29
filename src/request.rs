use crate::analyzer::Analyzer;
use crate::codelens;
use crate::document_storage::DocumentStorage;
use lsp_server::{Connection, ErrorCode, Message, Request, RequestId, Response, ResponseError};
use lsp_types::request::Request as _;
use lsp_types::{
    CodeLens, CodeLensParams,
    request::{CodeLensRequest, CodeLensResolve},
};
use std::{error::Error, path::PathBuf};

pub trait RequestHandle {
    fn handle(&self) -> Result<(), Box<dyn Error>>;
}

pub struct UnityRequest<'a> {
    connection: &'a Connection,
    request: &'a Request,
    docs: &'a DocumentStorage,
    analyzer: &'a Analyzer<'a>,
}

impl<'a> UnityRequest<'a> {
    pub fn new(
        connection: &'a Connection,
        request: &'a Request,
        docs: &'a DocumentStorage,
        analyzer: &'a Analyzer,
    ) -> Self {
        Self {
            connection,
            request,
            docs,
            analyzer,
        }
    }
}

impl<'a> RequestHandle for UnityRequest<'a> {
    fn handle(&self) -> Result<(), Box<dyn Error>> {
        match self.request.method.as_str() {
            CodeLensRequest::METHOD => {
                let p = serde_json::from_value::<CodeLensParams>(self.request.params.clone())?;
                let uri = p.text_document.uri;

                if let Some(content) = self.docs.get(&uri) {
                    let analysis = self
                        .analyzer
                        .analyze_script(&content, &PathBuf::from(uri.as_str()));
                    let codelens = codelens::create_codelens(analysis)?;

                    send_ok(self.connection, self.request.id.clone(), &codelens)?;
                }
            }
            CodeLensResolve::METHOD => {
                let lens = serde_json::from_value::<CodeLens>(self.request.params.clone())?;
                let lens = codelens::resolve_codelens(lens)?;

                send_ok(self.connection, self.request.id.clone(), &lens)?;
            }
            _ => send_err(
                self.connection,
                self.request.id.clone(),
                ErrorCode::MethodNotFound,
                "unhandled method",
            )?,
        }

        Ok(())
    }
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

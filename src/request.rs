use crate::document_storage::DocumentStorage;
use crate::{analyzer::Analyzer, capabilities::code_lens};
use gen_lsp_types::{CodeLens, CodeLensParams, LspRequestMethod};
use lsp_server::{Connection, ErrorCode, Message, Request, RequestId, Response, ResponseError};
use std::error::Error;

pub trait RequestHandle {
    fn handle(&self) -> Result<(), Box<dyn Error>>;
}

pub struct UnityRequest<'a> {
    connection: &'a Connection,
    request: &'a Request,
    docs: &'a DocumentStorage,
    analyzer: &'a Analyzer,
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
        let params = self.request.params.clone();

        match LspRequestMethod::from(self.request.method.clone()) {
            LspRequestMethod::TextDocumentCodeLens => {
                let params = serde_json::from_value::<CodeLensParams>(params)?;
                let uri = params.text_document.uri;

                let codelens = if let Some(content) = self.docs.get(&uri) {
                    let analysis = self.analyzer.analyze_script(content, uri);
                    code_lens::create_codelens(analysis)?
                } else {
                    Vec::new()
                };

                send_ok(self.connection, self.request.id.clone(), &codelens)?;
            }
            LspRequestMethod::CodeLensResolve => {
                let mut codelens = serde_json::from_value::<CodeLens>(params)?;
                codelens = code_lens::resolve_codelens(codelens)?;

                send_ok(self.connection, self.request.id.clone(), &codelens)?;
            }
            _ => send_err(
                self.connection,
                self.request.id.clone(),
                ErrorCode::MethodNotFound,
                &format!("unhandled method: {}", self.request.method),
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

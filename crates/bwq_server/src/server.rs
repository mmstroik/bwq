use std::collections::HashMap;

use anyhow::Result;
use lsp_server::{Connection, Message, Notification, Request, Response};
use lsp_types::{
    DidChangeTextDocumentParams, DidCloseTextDocumentParams, DidOpenTextDocumentParams,
    InitializeParams, InitializeResult, PublishDiagnosticsParams, ServerCapabilities,
    TextDocumentSyncCapability, TextDocumentSyncKind, Uri,
    notification::{
        DidChangeTextDocument, DidCloseTextDocument, DidOpenTextDocument,
        Notification as NotificationTrait, PublishDiagnostics,
    },
    request::{Initialize, Request as RequestTrait, Shutdown},
};

use crate::diagnostics_handler::DiagnosticsHandler;
use bwq_linter::BrandwatchLinter;

pub struct Server {
    connection: Connection,
    linter: BrandwatchLinter,
    documents: HashMap<Uri, DocumentState>,
    diagnostics_handler: DiagnosticsHandler,
}

#[derive(Debug, Clone)]
struct DocumentState {
    content: String,
    version: i32,
}

impl Server {
    pub fn new(connection: Connection) -> Self {
        Self {
            connection,
            linter: BrandwatchLinter::new(),
            documents: HashMap::new(),
            diagnostics_handler: DiagnosticsHandler::new(),
        }
    }

    pub fn run() -> Result<()> {
        let (connection, io_threads) = Connection::stdio();

        let (initialize_id, initialize_params) = connection.initialize_start()?;
        let _initialize_params: InitializeParams = serde_json::from_value(initialize_params)?;

        let initialize_result = InitializeResult {
            capabilities: ServerCapabilities {
                text_document_sync: Some(TextDocumentSyncCapability::Kind(
                    TextDocumentSyncKind::FULL,
                )),
                ..Default::default()
            },
            server_info: Some(lsp_types::ServerInfo {
                name: "bwq-server".to_string(),
                version: Some(env!("CARGO_PKG_VERSION").to_string()),
            }),
        };

        connection.initialize_finish(initialize_id, serde_json::to_value(initialize_result)?)?;

        let mut server = Server::new(connection);
        server.main_loop()?;

        io_threads.join()?;
        Ok(())
    }

    fn main_loop(&mut self) -> Result<()> {
        eprintln!("bwq language server started");

        while let Ok(msg) = self.connection.receiver.recv() {
            match msg {
                Message::Request(req) => {
                    if self.connection.handle_shutdown(&req)? {
                        break;
                    }
                    self.handle_request(req)?;
                }
                Message::Notification(not) => {
                    self.handle_notification(not)?;
                }
                Message::Response(_) => {}
            }
        }

        eprintln!("bwq language server stopped");
        Ok(())
    }

    fn handle_request(&mut self, req: Request) -> Result<()> {
        match req.method.as_str() {
            <Initialize as RequestTrait>::METHOD => {}
            <Shutdown as RequestTrait>::METHOD => {}
            _ => {
                let response = Response::new_err(
                    req.id,
                    lsp_server::ErrorCode::MethodNotFound as i32,
                    "Method not found".to_string(),
                );
                self.connection.sender.send(Message::Response(response))?;
            }
        }
        Ok(())
    }

    fn handle_notification(&mut self, not: Notification) -> Result<()> {
        match not.method.as_str() {
            <DidOpenTextDocument as NotificationTrait>::METHOD => {
                let params: DidOpenTextDocumentParams = serde_json::from_value(not.params)?;
                self.handle_did_open(params)?;
            }
            <DidChangeTextDocument as NotificationTrait>::METHOD => {
                let params: DidChangeTextDocumentParams = serde_json::from_value(not.params)?;
                self.handle_did_change(params)?;
            }
            <DidCloseTextDocument as NotificationTrait>::METHOD => {
                let params: DidCloseTextDocumentParams = serde_json::from_value(not.params)?;
                self.handle_did_close(params)?;
            }
            _ => {
                eprintln!("Unknown notification: {}", not.method);
            }
        }
        Ok(())
    }

    fn handle_did_open(&mut self, params: DidOpenTextDocumentParams) -> Result<()> {
        let doc = params.text_document;

        if !is_bwq_file(&doc.uri) {
            return Ok(());
        }

        let document_state = DocumentState {
            content: doc.text.clone(),
            version: doc.version,
        };

        self.documents.insert(doc.uri.clone(), document_state);
        self.publish_diagnostics(&doc.uri, &doc.text)?;

        eprintln!("Opened document: {:?}", doc.uri);
        Ok(())
    }

    fn handle_did_change(&mut self, params: DidChangeTextDocumentParams) -> Result<()> {
        let uri = params.text_document.uri;

        if !is_bwq_file(&uri) {
            return Ok(());
        }

        if let Some(document) = self.documents.get_mut(&uri) {
            if let Some(change) = params.content_changes.into_iter().next() {
                document.content = change.text.clone();
                document.version = params.text_document.version;

                self.publish_diagnostics(&uri, &change.text)?;
            }
        }

        Ok(())
    }

    fn handle_did_close(&mut self, params: DidCloseTextDocumentParams) -> Result<()> {
        let uri = params.text_document.uri;
        self.documents.remove(&uri);

        let diagnostics = PublishDiagnosticsParams {
            uri: uri.clone(),
            diagnostics: vec![],
            version: None,
        };

        let notification = Notification::new(
            <PublishDiagnostics as NotificationTrait>::METHOD.to_string(),
            serde_json::to_value(diagnostics)?,
        );

        self.connection
            .sender
            .send(Message::Notification(notification))?;

        eprintln!("Closed document: {uri:?}");
        Ok(())
    }

    fn publish_diagnostics(&mut self, uri: &Uri, content: &str) -> Result<()> {
        let diagnostics = self
            .diagnostics_handler
            .analyze_content(content, &mut self.linter)?;

        let publish_diagnostics = PublishDiagnosticsParams {
            uri: uri.clone(),
            diagnostics,
            version: None,
        };

        let notification = Notification::new(
            <PublishDiagnostics as NotificationTrait>::METHOD.to_string(),
            serde_json::to_value(publish_diagnostics)?,
        );

        self.connection
            .sender
            .send(Message::Notification(notification))?;
        Ok(())
    }
}

fn is_bwq_file(uri: &Uri) -> bool {
    let path = uri.path().as_str();
    path.ends_with(".bwq") || path.ends_with(".txt")
}

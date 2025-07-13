use std::collections::HashMap;
use std::num::NonZeroUsize;

use anyhow::Result;
use crossbeam_channel::Receiver;
use lsp_server::{Connection, Message, Notification, Request, Response};
use lsp_types::{
    DidChangeTextDocumentParams, DidCloseTextDocumentParams, DidOpenTextDocumentParams,
    PublishDiagnosticsParams, Uri,
    notification::{
        DidChangeTextDocument, DidCloseTextDocument, DidOpenTextDocument,
        Notification as NotificationTrait, PublishDiagnostics,
    },
    request::{Initialize, Request as RequestTrait, Shutdown},
};

use crate::connection::{ConnectionInitializer, server_capabilities};
use crate::request_queue::RequestQueue;
use crate::task::{TaskExecutor, TaskResponse};

pub struct Server {
    connection: Connection,
    documents: HashMap<Uri, DocumentState>,
    request_queue: RequestQueue,
    task_executor: TaskExecutor,
    task_response_receiver: Receiver<TaskResponse>,
}
#[derive(Debug, Clone)]
struct DocumentState {
    content: String,
    version: i32,
}
impl Server {
    pub fn new(
        worker_threads: NonZeroUsize,
        connection_initializer: ConnectionInitializer,
    ) -> Result<Self> {
        let (id, _init_params) = connection_initializer.initialize_start()?;

        let capabilities = server_capabilities();

        let connection = connection_initializer.initialize_finish(
            id,
            capabilities,
            "bwq-server",
            env!("CARGO_PKG_VERSION"),
        )?;

        let (task_response_sender, task_response_receiver) = crossbeam_channel::bounded(16);
        let task_executor = TaskExecutor::new(worker_threads, task_response_sender);

        Ok(Self {
            connection,
            documents: HashMap::new(),
            request_queue: RequestQueue::new(),
            task_executor,
            task_response_receiver,
        })
    }

    pub fn run(mut self) -> Result<()> {
        self.main_loop()
    }

    fn main_loop(&mut self) -> Result<()> {
        tracing::info!("bwq language server started");

        loop {
            crossbeam_channel::select! {
                // Handle incoming LSP messages
                recv(self.connection.receiver) -> msg => {
                    match msg {
                        Ok(msg) => {
                            match msg {
                                Message::Request(req) => {
                                    self.request_queue.register(req.id.clone(), req.method.clone());

                                    if self.connection.handle_shutdown(&req)? {
                                        if let Some((start_time, method)) = self.request_queue.complete(&req.id) {
                                            let duration = start_time.elapsed();
                                            tracing::trace!("Request completed: {} ({}μs)", method, duration.as_micros());
                                        }
                                        break;
                                    }

                                    let result = self.handle_request(req.clone());

                                    if let Some((start_time, method)) = self.request_queue.complete(&req.id) {
                                        let duration = start_time.elapsed();
                                        tracing::trace!("Request completed: {} ({}μs)", method, duration.as_micros());
                                    }

                                    result?;
                                }
                                Message::Notification(not) => {
                                    self.handle_notification(not)?;
                                }
                                Message::Response(_) => {}
                            }
                        }
                        Err(_) => {
                            tracing::info!("LSP connection closed");
                            break;
                        }
                    }
                }
                recv(self.task_response_receiver) -> response => {
                    match response {
                        Ok(TaskResponse::Diagnostics(params)) => {
                            self.send_diagnostics(params)?;
                        }
                        Err(_) => {
                            tracing::info!("Task response channel closed - exiting");
                            break;
                        }
                    }
                }
            }
        }

        tracing::info!("bwq language server stopped");
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
            "$/cancelRequest" => {
                self.handle_cancel_request(not.params)?;
            }
            _ => {
                tracing::debug!("Unknown notification: {}", not.method);
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
        self.schedule_diagnostics(&doc.uri, &doc.text)?;

        tracing::debug!("Opened document: {:?}", doc.uri);
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

                self.schedule_diagnostics(&uri, &change.text)?;
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

        tracing::debug!("Closed document: {uri:?}");
        Ok(())
    }

    /// Schedule diagnostics processing in the background
    fn schedule_diagnostics(&mut self, uri: &Uri, content: &str) -> Result<()> {
        // Notifications like didChange/didOpen don't have request IDs,
        // so they cannot be cancelled - pass None for cancellation token
        self.task_executor
            .schedule_diagnostics(uri.clone(), content.to_string(), None)?;

        Ok(())
    }

    /// Send diagnostics notification to client (called from background task results)
    fn send_diagnostics(&mut self, params: PublishDiagnosticsParams) -> Result<()> {
        let notification = Notification::new(
            <PublishDiagnostics as NotificationTrait>::METHOD.to_string(),
            serde_json::to_value(params)?,
        );

        self.connection
            .sender
            .send(Message::Notification(notification))?;
        Ok(())
    }

    fn handle_cancel_request(&mut self, params: serde_json::Value) -> Result<()> {
        #[derive(serde::Deserialize)]
        struct CancelParams {
            id: lsp_server::RequestId,
        }

        if let Ok(cancel_params) = serde_json::from_value::<CancelParams>(params) {
            if let Some(method) = self.request_queue.cancel(&cancel_params.id) {
                tracing::debug!("Cancelled request: {} (id: {:?})", method, cancel_params.id);
            }
        }

        Ok(())
    }
}

fn is_bwq_file(uri: &Uri) -> bool {
    let path = uri.path().as_str();
    path.ends_with(".bwq") || path.ends_with(".txt")
}

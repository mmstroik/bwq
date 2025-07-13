use std::collections::HashMap;
use std::num::NonZeroUsize;

use anyhow::Result;
use crossbeam_channel::Receiver;
use lsp_server::{Connection, Message, Notification, Request, Response};
use lsp_types::{
    DidChangeTextDocumentParams, DidCloseTextDocumentParams, DidOpenTextDocumentParams, Hover,
    HoverContents, HoverParams, MarkupContent, MarkupKind, Position, PublishDiagnosticsParams,
    Range, Uri,
    notification::{
        DidChangeTextDocument, DidCloseTextDocument, DidOpenTextDocument,
        Notification as NotificationTrait, PublishDiagnostics,
    },
    request::{HoverRequest, Initialize, Request as RequestTrait, Shutdown},
};

use crate::connection::{ConnectionInitializer, server_capabilities};
use crate::request_queue::RequestQueue;
use crate::task::{TaskExecutor, TaskResponse};
use crate::wikidata::{EntityInfo, WikiDataClient};

pub struct Server {
    connection: Connection,
    documents: HashMap<Uri, DocumentState>,
    request_queue: RequestQueue,
    task_executor: TaskExecutor,
    task_response_receiver: Receiver<TaskResponse>,
    hover_enabled: bool,
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
        let (id, init_params) = connection_initializer.initialize_start()?;

        // Extract WikiData hover setting from initialization options
        let enable_hover = init_params
            .initialization_options
            .as_ref()
            .and_then(|opts| opts.get("wikidataHoverEnabled"))
            .and_then(|v| v.as_bool())
            .unwrap_or(true); // Default to enabled

        let capabilities = server_capabilities(enable_hover);

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
            hover_enabled: enable_hover,
        })
    }

    pub fn run(mut self) -> Result<()> {
        self.main_loop()
    }

    fn main_loop(&mut self) -> Result<()> {
        tracing::info!("bwq language server started");

        loop {
            crossbeam_channel::select! {
                recv(self.connection.receiver) -> msg => {
                    match msg {
                        Ok(msg) => {
                            match msg {
                                Message::Request(req) => {
                                    self.request_queue.register(req.id.clone(), req.method.clone());


                                    let mut request_completed = false;

                                    let handle_result = (|| -> Result<bool> {
                                        if self.connection.handle_shutdown(&req)? {
                                            return Ok(true);
                                        }
                                        self.handle_request(req.clone())?;
                                        Ok(false)
                                    })();

                                    // always complete the request in our tracking
                                    if let Some((start_time, method)) = self.request_queue.complete(&req.id) {
                                        let duration = start_time.elapsed();
                                        tracing::trace!("Request completed: {} ({}μs)", method, duration.as_micros());
                                        request_completed = true;
                                    }

                                    match handle_result {
                                        Ok(true) => break,
                                        Ok(false) => {},
                                        Err(e) => {
                                            if !request_completed {
                                                tracing::warn!("Request {} failed but was not tracked", req.id);
                                            }
                                            return Err(e);
                                        }
                                    }
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
                        Ok(TaskResponse::EntityInfo { request_id, entity_info }) => {
                            self.send_entity_info_response(request_id, entity_info)?;
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
            <HoverRequest as RequestTrait>::METHOD => {
                self.handle_hover_request(req)?;
            }
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

    fn handle_hover_request(&mut self, req: Request) -> Result<()> {
        if !self.hover_enabled {
            let response = Response::new_ok(req.id, serde_json::Value::Null);
            self.connection.sender.send(Message::Response(response))?;
            return Ok(());
        }

        let params: HoverParams = match serde_json::from_value(req.params) {
            Ok(params) => params,
            Err(e) => {
                let response = Response::new_err(
                    req.id,
                    lsp_server::ErrorCode::InvalidParams as i32,
                    format!("Invalid hover params: {e}"),
                );
                self.connection.sender.send(Message::Response(response))?;
                return Ok(());
            }
        };

        let uri = params.text_document_position_params.text_document.uri;
        let position = params.text_document_position_params.position;

        let document_content = match self.documents.get(&uri) {
            Some(doc) => &doc.content,
            None => {
                let response = Response::new_ok(req.id, serde_json::Value::Null);
                self.connection.sender.send(Message::Response(response))?;
                return Ok(());
            }
        };

        let byte_position = self.lsp_position_to_byte_position(document_content, position);

        if let Some(entity_id) =
            WikiDataClient::extract_entity_id_from_text(document_content, byte_position)
        {
            self.task_executor
                .schedule_entity_lookup(req.id, entity_id)?;
        } else {
            let response = Response::new_ok(req.id, serde_json::Value::Null);
            self.connection.sender.send(Message::Response(response))?;
        }

        Ok(())
    }

    fn send_entity_info_response(
        &mut self,
        request_id: lsp_server::RequestId,
        entity_info: Option<EntityInfo>,
    ) -> Result<()> {
        let response = match entity_info {
            Some(info) => {
                let hover_content = format!(
                    "**{}** ({})\n\n{}\n\n[View on WikiData]({})",
                    info.label,
                    info.id,
                    info.description
                        .unwrap_or_else(|| "No description available.".to_string()),
                    info.url
                );

                let hover = Hover {
                    contents: HoverContents::Markup(MarkupContent {
                        kind: MarkupKind::Markdown,
                        value: hover_content,
                    }),
                    range: None, // We'll simplify this for now
                };

                Response::new_ok(request_id, serde_json::to_value(hover)?)
            }
            None => Response::new_ok(request_id, serde_json::Value::Null),
        };

        self.connection.sender.send(Message::Response(response))?;
        Ok(())
    }

    fn lsp_position_to_byte_position(&self, text: &str, position: Position) -> usize {
        let line_offset = position.line as usize;
        let char_offset = position.character as usize;

        let lines: Vec<&str> = text.lines().collect();
        if line_offset >= lines.len() {
            return text.len();
        }

        let line = lines[line_offset];
        if char_offset > line.len() {
            let mut byte_position = 0;
            for (i, line) in lines.iter().enumerate() {
                if i == line_offset {
                    byte_position += line.len();
                    break;
                }
                byte_position += line.len() + 1; // +1 for newline
            }
            return byte_position;
        }

        // Calculate byte position in the document
        let mut byte_position = 0;
        for (i, line) in lines.iter().enumerate() {
            if i == line_offset {
                byte_position += char_offset;
                break;
            }
            byte_position += line.len() + 1; // +1 for newline
        }

        byte_position
    }

    fn calculate_entity_id_range(&self, text: &str, position: usize, entity_id: &str) -> Range {
        // Find the entityId: field around the position
        let start = position.saturating_sub(20);
        let end = (position + 20).min(text.len());
        let search_text = &text[start..end];

        for (i, _) in search_text.match_indices("entityId:") {
            let field_start = start + i;
            let value_start = field_start + 9;
            let value_end = value_start + entity_id.len();

            if value_start <= position && position <= value_end {
                // Convert byte positions back to line/character positions
                let start_pos = self.byte_position_to_lsp_position(text, field_start);
                let end_pos = self.byte_position_to_lsp_position(text, value_end);

                return Range {
                    start: start_pos,
                    end: end_pos,
                };
            }
        }

        // Fallback to single character range
        let pos = self.byte_position_to_lsp_position(text, position);
        Range {
            start: pos,
            end: Position {
                line: pos.line,
                character: pos.character + 1,
            },
        }
    }

    fn byte_position_to_lsp_position(&self, text: &str, byte_position: usize) -> Position {
        let mut line = 0;
        let mut character = 0;
        let mut current_byte = 0;

        for ch in text.chars() {
            if current_byte >= byte_position {
                break;
            }

            if ch == '\n' {
                line += 1;
                character = 0;
            } else {
                character += 1;
            }

            current_byte += ch.len_utf8();
        }

        Position {
            line: line as u32,
            character: character as u32,
        }
    }
}

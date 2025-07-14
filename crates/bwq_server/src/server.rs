use std::collections::HashMap;
use std::num::NonZeroUsize;

use anyhow::Result;
use bwq_linter::ast::{Expression, FieldType, Query};
use crossbeam_channel::Receiver;
use lru::LruCache;
use lsp_server::{Connection, Message, Notification, Request, Response};
use lsp_types::{
    DidChangeTextDocumentParams, DidCloseTextDocumentParams, DidOpenTextDocumentParams, Hover,
    HoverContents, HoverParams, MarkupContent, MarkupKind, Position, PublishDiagnosticsParams, Uri,
    notification::{
        DidChangeTextDocument, DidCloseTextDocument, DidOpenTextDocument,
        Notification as NotificationTrait, PublishDiagnostics,
    },
    request::{HoverRequest, Initialize, Request as RequestTrait, Shutdown},
};

use crate::connection::{ConnectionInitializer, server_capabilities};
use crate::request_queue::RequestQueue;
use crate::task::{TaskExecutor, TaskResponse};
use crate::wikidata::{EntityInfo, EntitySearchResult};

#[derive(Debug, serde::Deserialize, serde::Serialize)]
struct EntitySearchParams {
    query: String,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct EntitySearchResponse {
    results: Vec<EntitySearchResult>,
}

#[allow(dead_code)] // used for trait implementation only
struct EntitySearchRequest;

impl RequestTrait for EntitySearchRequest {
    type Params = EntitySearchParams;
    type Result = EntitySearchResponse;
    const METHOD: &'static str = "bwq/searchEntities";
}

#[derive(Debug, Clone)]
enum AstState {
    NotParsed,
    Parsing,
    Cached,
}

pub struct Server {
    connection: Connection,
    documents: HashMap<Uri, DocumentState>,
    ast_cache: LruCache<Uri, Query>,
    request_queue: RequestQueue,
    task_executor: TaskExecutor,
    task_response_receiver: Receiver<TaskResponse>,
    hover_enabled: bool,
}

#[derive(Debug, Clone)]
struct DocumentState {
    content: String,
    version: i32,
    ast_state: AstState,
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
            ast_cache: LruCache::new(NonZeroUsize::new(10).unwrap()),
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
                        Ok(TaskResponse::Diagnostics { params, ast, document_version }) => {
                            // Cache AST if available and document version is still current
                            if let Some(ast) = ast {
                                // Check if document version is still current to prevent stale AST caching
                                let current_version = self.documents.get(&params.uri).map(|doc| doc.version);
                                if document_version == current_version {
                                    // Check if this evicts an entry from the LRU cache
                                    let cache_size_before = self.ast_cache.len();
                                    self.ast_cache.put(params.uri.clone(), ast);
                                    let cache_size_after = self.ast_cache.len();

                                    if cache_size_after < cache_size_before {
                                        tracing::debug!("AST CACHE: LRU evicted entry - cache size: {}", cache_size_after);
                                    } else {
                                        tracing::debug!("AST CACHE: Cached AST - size: {}/{}", cache_size_after, self.ast_cache.cap());
                                    }

                                    // Update document state to indicate AST is cached
                                    if let Some(document) = self.documents.get_mut(&params.uri) {
                                        document.ast_state = AstState::Cached;
                                    }
                                } else {
                                    tracing::debug!("AST CACHE: Skipping cache for stale AST - document version changed from {:?} to {:?}", document_version, current_version);
                                }
                            } else {
                                tracing::debug!("Diagnostics completed: {:?} - no AST returned (likely parse error)", params.uri);
                            }

                            self.send_diagnostics(params)?;
                        }
                        Ok(TaskResponse::EntityInfo { request_id, entity_info }) => {
                            self.send_entity_info_response(request_id, entity_info)?;
                        }
                        Ok(TaskResponse::EntitySearchResults { request_id, results }) => {
                            self.send_entity_search_response(request_id, results)?;
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
            <EntitySearchRequest as RequestTrait>::METHOD => {
                self.handle_entity_search_request(req)?;
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

    fn handle_entity_search_request(&mut self, req: Request) -> Result<()> {
        let params: EntitySearchParams = match serde_json::from_value(req.params) {
            Ok(params) => params,
            Err(e) => {
                let response = Response::new_err(
                    req.id,
                    lsp_server::ErrorCode::InvalidParams as i32,
                    format!("Invalid entity search params: {e}"),
                );
                self.connection.sender.send(Message::Response(response))?;
                return Ok(());
            }
        };

        tracing::debug!("Entity search request for query: {}", params.query);

        self.task_executor
            .schedule_entity_search(req.id, params.query)?;
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
            ast_state: AstState::NotParsed,
        };

        self.documents.insert(doc.uri.clone(), document_state);
        tracing::debug!("Document opened: {:?} - AST state: NotParsed", doc.uri);
        self.schedule_diagnostics(&doc.uri, &doc.text)?;
        Ok(())
    }

    fn handle_did_change(&mut self, params: DidChangeTextDocumentParams) -> Result<()> {
        let uri = params.text_document.uri;

        if let Some(document) = self.documents.get_mut(&uri) {
            if let Some(change) = params.content_changes.into_iter().next() {
                document.content = change.text.clone();
                document.version = params.text_document.version;
                document.ast_state = AstState::NotParsed;

                // Clear cached AST since content changed
                let had_cached_ast = self.ast_cache.pop(&uri).is_some();
                if had_cached_ast {
                    tracing::debug!("Document changed: {:?} - AST cache invalidated", uri);
                } else {
                    tracing::debug!("Document changed: {:?} - no cached AST to invalidate", uri);
                }

                self.schedule_diagnostics(&uri, &change.text)?;
            }
        }

        Ok(())
    }

    fn handle_did_close(&mut self, params: DidCloseTextDocumentParams) -> Result<()> {
        let uri = params.text_document.uri;
        self.documents.remove(&uri);
        let had_cached_ast = self.ast_cache.pop(&uri).is_some();
        if had_cached_ast {
            tracing::debug!("Document closed: {:?} - removed from AST cache", uri);
        } else {
            tracing::debug!("Document closed: {:?} - no cached AST to remove", uri);
        }

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
        let document_version = self.documents.get(uri).map(|doc| doc.version);

        // Mark document as parsing
        if let Some(document) = self.documents.get_mut(uri) {
            document.ast_state = AstState::Parsing;
            tracing::debug!("Document parsing started: {:?} - AST state: Parsing", uri);
        }

        // Notifications like didChange/didOpen don't have request IDs,
        // so they cannot be cancelled - pass None for cancellation token
        self.task_executor.schedule_diagnostics(
            uri.clone(),
            content.to_string(),
            document_version,
            None,
        )?;

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

        // Try to get cached AST, wait for parsing if not available
        let entity_id = if let Some(ast) = self.ast_cache.get(&uri).cloned() {
            tracing::debug!(
                "HOVER: Using cached AST for entity extraction (cache size: {}/{})",
                self.ast_cache.len(),
                self.ast_cache.cap()
            );
            self.find_entity_id_at_position(&ast, byte_position)
        } else {
            // Check document state to see if we should wait for parsing
            let document_state = self
                .documents
                .get(&uri)
                .map(|d| (d.ast_state.clone(), d.content.clone()));
            if let Some((ast_state, content)) = document_state {
                match ast_state {
                    AstState::Parsing => {
                        tracing::debug!("HOVER: AST parsing in progress, will retry when ready");
                        // Return null for now, client can retry
                        let response = Response::new_ok(req.id, serde_json::Value::Null);
                        self.connection.sender.send(Message::Response(response))?;
                        return Ok(());
                    }
                    AstState::NotParsed => {
                        tracing::debug!(
                            "HOVER: AST not parsed yet, triggering parse and returning null"
                        );
                        // Trigger parsing and return null for now
                        self.schedule_diagnostics(&uri, &content)?;
                        let response = Response::new_ok(req.id, serde_json::Value::Null);
                        self.connection.sender.send(Message::Response(response))?;
                        return Ok(());
                    }
                    AstState::Cached => {
                        // This shouldn't happen since we just checked the cache
                        tracing::debug!(
                            "HOVER: AST state says cached but not in cache, returning null"
                        );
                        None
                    }
                }
            } else {
                tracing::debug!("HOVER: Document not found, returning null");
                None
            }
        };

        if let Some(entity_id) = entity_id {
            tracing::debug!(
                "Hover request: {:?} - found entityId: {} at position {}",
                uri,
                entity_id,
                byte_position
            );
            self.task_executor
                .schedule_entity_lookup(req.id, entity_id)?;
        } else {
            tracing::debug!(
                "Hover request: {:?} - no entityId found at position {}",
                uri,
                byte_position
            );
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

    fn send_entity_search_response(
        &mut self,
        request_id: lsp_server::RequestId,
        results: Result<Vec<EntitySearchResult>, String>,
    ) -> Result<()> {
        let response = match results {
            Ok(results) => {
                tracing::debug!("Entity search found {} results", results.len());
                let search_response = EntitySearchResponse { results };
                Response::new_ok(request_id, serde_json::to_value(search_response)?)
            }
            Err(e) => {
                tracing::error!("Entity search failed: {}", e);
                Response::new_err(
                    request_id,
                    lsp_server::ErrorCode::InternalError as i32,
                    format!("Entity search failed: {e}"),
                )
            }
        };

        self.connection.sender.send(Message::Response(response))?;
        Ok(())
    }

    fn lsp_position_to_byte_position(&self, text: &str, position: Position) -> usize {
        let line_offset = position.line as usize;
        let utf16_char_offset = position.character as usize;

        // Find line boundaries by examining actual line endings in the text
        let mut line_start_positions = vec![0];
        let mut pos = 0;
        let text_bytes = text.as_bytes();

        while pos < text_bytes.len() {
            if text_bytes[pos] == b'\n' {
                line_start_positions.push(pos + 1);
                pos += 1;
            } else if text_bytes[pos] == b'\r'
                && pos + 1 < text_bytes.len()
                && text_bytes[pos + 1] == b'\n'
            {
                line_start_positions.push(pos + 2);
                pos += 2;
            } else {
                pos += 1;
            }
        }

        if line_offset >= line_start_positions.len() - 1 {
            return text.len();
        }

        let line_start = line_start_positions[line_offset];
        let line_end = if line_offset + 1 < line_start_positions.len() {
            // Subtract line ending bytes to get the actual line content end
            let next_line_start = line_start_positions[line_offset + 1];
            if next_line_start >= 2 && text_bytes.get(next_line_start - 2) == Some(&b'\r') {
                next_line_start - 2 // CRLF
            } else if next_line_start >= 1 {
                next_line_start - 1 // LF
            } else {
                next_line_start
            }
        } else {
            text.len()
        };

        let line = &text[line_start..line_end];

        // Convert UTF-16 character offset to UTF-8 byte offset within the line
        let line_byte_offset = {
            let mut utf16_count = 0;
            let mut byte_offset = 0;

            for ch in line.chars() {
                if utf16_count >= utf16_char_offset {
                    break;
                }
                byte_offset += ch.len_utf8();
                utf16_count += ch.len_utf16();
            }

            // Clamp to line length if offset is beyond line end
            byte_offset.min(line.len())
        };

        line_start + line_byte_offset
    }

    fn find_entity_id_at_position(&self, ast: &Query, position: usize) -> Option<String> {
        Self::find_entity_id_in_expression(&ast.expression, position)
    }

    fn find_entity_id_in_expression(expr: &Expression, position: usize) -> Option<String> {
        match expr {
            Expression::Field {
                field: FieldType::EntityId,
                value,
                span,
            } => {
                if position >= span.start.offset && position <= span.end.offset {
                    if let Expression::Term { term, .. } = value.as_ref() {
                        match term {
                            bwq_linter::ast::Term::Word { value } => Some(value.clone()),
                            _ => None,
                        }
                    } else {
                        None
                    }
                } else {
                    None
                }
            }
            Expression::BooleanOp { left, right, .. } => {
                // recursively search in left and right operands
                if let Some(entity_id) = Self::find_entity_id_in_expression(left, position) {
                    Some(entity_id)
                } else if let Some(right) = right {
                    Self::find_entity_id_in_expression(right, position)
                } else {
                    None
                }
            }
            Expression::Group { expression, .. } => {
                Self::find_entity_id_in_expression(expression, position)
            }
            Expression::Proximity { terms, .. } => {
                // search in all proximity terms
                for term in terms {
                    if let Some(entity_id) = Self::find_entity_id_in_expression(term, position) {
                        return Some(entity_id);
                    }
                }
                None
            }
            _ => None,
        }
    }
}

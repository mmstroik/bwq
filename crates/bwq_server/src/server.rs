use std::num::NonZeroUsize;

use anyhow::Result;
use crossbeam_channel::Receiver;
use lsp_server::{Connection, Message, Notification, Response};
use lsp_types::{
    Hover, HoverContents, MarkupContent, MarkupKind, PublishDiagnosticsParams,
    notification::{Notification as NotificationTrait, PublishDiagnostics},
    request::Request as RequestTrait,
};

use crate::connection::{ConnectionInitializer, server_capabilities};
use crate::server::handlers::EntitySearchParams;
use crate::task::{TaskExecutor, TaskResponse};
use crate::wikidata::{EntityInfo, EntitySearchResult};

mod client;
mod handlers;
mod session;
mod utils;

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

pub struct Server {
    connection: Connection,
    session: session::Session,
    task_executor: TaskExecutor,
    task_response_receiver: Receiver<TaskResponse>,
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
            session: session::Session::new(enable_hover),
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
                recv(self.connection.receiver) -> msg => {
                    match msg {
                        Ok(msg) => {
                            match msg {
                                Message::Request(req) => {
                                    self.session.request_queue.register(req.id.clone(), req.method.clone());

                                    let mut request_completed = false;

                                    let handle_result = (|| -> Result<bool> {
                                        if self.connection.handle_shutdown(&req)? {
                                            return Ok(true);
                                        }
                                        let client = client::Client::new(&self.connection);
                                        handlers::dispatch_request(&mut self.session, &client, &self.task_executor, req.clone())?;
                                        Ok(false)
                                    })();

                                    // always complete the request in our tracking
                                    if let Some((start_time, method)) = self.session.request_queue.complete(&req.id) {
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
                                    let client = client::Client::new(&self.connection);
                                    handlers::dispatch_notification(&mut self.session, &client, &self.task_executor, not)?;
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
                                let current_version = self.session.documents.get(&params.uri).map(|doc| doc.version);
                                if document_version == current_version {
                                    // Check if this will evict an entry from the LRU cache
                                    let will_evict = self.session.ast_cache.len() == self.session.ast_cache.cap().get() && !self.session.ast_cache.contains(&params.uri);
                                    self.session.ast_cache.put(params.uri.clone(), ast);

                                    if will_evict {
                                        tracing::debug!("AST CACHE: LRU evicted entry - cache size: {}", self.session.ast_cache.len());
                                    } else {
                                        tracing::debug!("AST CACHE: Cached AST - size: {}/{}", self.session.ast_cache.len(), self.session.ast_cache.cap().get());
                                    }

                                    // Update document state to indicate AST is cached
                                    if let Some(document) = self.session.documents.get_mut(&params.uri) {
                                        document.ast_state = session::AstState::Cached;
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
}

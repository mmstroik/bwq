use anyhow::Result;
use crossbeam_channel::{Receiver, Sender};
use lsp_types::{PublishDiagnosticsParams, Uri};
use std::num::NonZeroUsize;
use std::thread;

use crate::request_queue::CancellationToken;

/// Background task executor for async operations
pub struct TaskExecutor {
    task_sender: Sender<BackgroundTask>,
    _handles: Vec<thread::JoinHandle<()>>,
}

impl TaskExecutor {
    pub fn new(worker_threads: NonZeroUsize, response_sender: Sender<TaskResponse>) -> Self {
        let (task_sender, task_receiver) = crossbeam_channel::bounded(8);

        let mut handles = Vec::new();

        for i in 0..worker_threads.get() {
            let receiver = task_receiver.clone();
            let sender = response_sender.clone();

            let handle = thread::Builder::new()
                .name(format!("bwq-worker-{i}"))
                .spawn(move || {
                    worker_loop(receiver, sender);
                })
                .expect("Failed to spawn worker thread");

            handles.push(handle);
        }

        Self {
            task_sender,
            _handles: handles,
        }
    }

    /// Schedule a diagnostics task for background processing (internal)
    pub(crate) fn schedule_diagnostics(
        &self,
        uri: Uri,
        content: String,
        cancellation_token: Option<CancellationToken>,
    ) -> Result<()> {
        let task = BackgroundTask::Diagnostics {
            uri,
            content,
            cancellation_token,
        };

        self.task_sender.send(task)?;
        Ok(())
    }

    /// Schedule a diagnostics task without cancellation token (for tests)
    pub fn schedule_diagnostics_simple(&self, uri: Uri, content: String) -> Result<()> {
        self.schedule_diagnostics(uri, content, None)
    }

    /// Schedule an entity lookup task for background processing
    pub(crate) fn schedule_entity_lookup(
        &self,
        request_id: lsp_server::RequestId,
        entity_id: String,
    ) -> Result<()> {
        let task = BackgroundTask::EntityLookup {
            request_id,
            entity_id,
        };

        self.task_sender.send(task)?;
        Ok(())
    }
}

enum BackgroundTask {
    Diagnostics {
        uri: Uri,
        content: String,
        cancellation_token: Option<CancellationToken>,
    },
    EntityLookup {
        request_id: lsp_server::RequestId,
        entity_id: String,
    },
}

pub enum TaskResponse {
    Diagnostics(PublishDiagnosticsParams),
    EntityInfo {
        request_id: lsp_server::RequestId,
        entity_info: Option<crate::wikidata::EntityInfo>,
    },
}

fn worker_loop(receiver: Receiver<BackgroundTask>, sender: Sender<TaskResponse>) {
    use crate::diagnostics_handler::DiagnosticsHandler;
    use crate::wikidata::WikiDataClient;
    use bwq_linter::BrandwatchLinter;

    let mut linter = BrandwatchLinter::new();
    let diagnostics_handler = DiagnosticsHandler::new();
    let rt = tokio::runtime::Runtime::new().expect("Failed to create Tokio runtime");
    let mut wikidata_client = WikiDataClient::new().expect("Failed to create WikiData client");

    tracing::debug!("Worker thread started");

    while let Ok(task) = receiver.recv() {
        match task {
            BackgroundTask::Diagnostics {
                uri,
                content,
                cancellation_token,
            } => {
                // Check if request was cancelled before processing
                if let Some(ref token) = cancellation_token {
                    if token.is_cancelled() {
                        tracing::debug!("Skipping cancelled diagnostics for {:?}", uri);
                        continue;
                    }
                }

                tracing::trace!("Processing diagnostics for {:?}", uri);

                match diagnostics_handler.analyze_content(&content, &mut linter) {
                    Ok(diagnostics) => {
                        // Check cancellation again before sending result
                        if let Some(ref token) = cancellation_token {
                            if token.is_cancelled() {
                                tracing::debug!("Discarding cancelled diagnostics for {:?}", uri);
                                continue;
                            }
                        }

                        let response = TaskResponse::Diagnostics(PublishDiagnosticsParams {
                            uri,
                            diagnostics,
                            version: None,
                        });

                        if sender.send(response).is_err() {
                            tracing::debug!(
                                "Failed to send diagnostics response (receiver dropped)"
                            );
                            break;
                        }
                    }
                    Err(e) => {
                        tracing::error!("Failed to analyze content for {:?}: {}", uri, e);
                        let response = TaskResponse::Diagnostics(PublishDiagnosticsParams {
                            uri,
                            diagnostics: vec![],
                            version: None,
                        });

                        if sender.send(response).is_err() {
                            tracing::debug!("Failed to send error diagnostics response");
                            break;
                        }
                    }
                }
            }
            BackgroundTask::EntityLookup {
                request_id,
                entity_id,
            } => {
                tracing::trace!("Processing entity lookup for ID: {}", entity_id);

                let entity_info =
                    rt.block_on(async { wikidata_client.get_entity_info(&entity_id).await });

                match entity_info {
                    Ok(info) => {
                        let response = TaskResponse::EntityInfo {
                            request_id,
                            entity_info: info,
                        };

                        if sender.send(response).is_err() {
                            tracing::debug!(
                                "Failed to send entity info response (receiver dropped)"
                            );
                            break;
                        }
                    }
                    Err(e) => {
                        tracing::error!("Failed to fetch entity info for {}: {}", entity_id, e);
                        let response = TaskResponse::EntityInfo {
                            request_id,
                            entity_info: None,
                        };

                        if sender.send(response).is_err() {
                            tracing::debug!("Failed to send error entity info response");
                            break;
                        }
                    }
                }
            }
        }
    }

    tracing::debug!("Worker thread stopped");
}

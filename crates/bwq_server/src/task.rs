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
}

enum BackgroundTask {
    Diagnostics {
        uri: Uri,
        content: String,
        cancellation_token: Option<CancellationToken>,
    },
}

pub enum TaskResponse {
    Diagnostics(PublishDiagnosticsParams),
}

fn worker_loop(receiver: Receiver<BackgroundTask>, sender: Sender<TaskResponse>) {
    use crate::diagnostics_handler::DiagnosticsHandler;
    use bwq_linter::BrandwatchLinter;

    let mut linter = BrandwatchLinter::new();
    let diagnostics_handler = DiagnosticsHandler::new();

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
        }
    }

    tracing::debug!("Worker thread stopped");
}

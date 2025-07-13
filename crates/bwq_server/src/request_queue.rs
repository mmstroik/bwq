use lsp_server::RequestId;
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Instant;

/// Tracks pending requests from client to server.
#[derive(Default, Debug)]
pub(crate) struct RequestQueue {
    pending: HashMap<RequestId, PendingRequest>,
}

impl RequestQueue {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    pub(crate) fn register(&mut self, request_id: RequestId, method: String) {
        self.pending.insert(request_id, PendingRequest::new(method));
    }

    /// Cancels the pending request with the given id.
    /// Returns the method name if the request was still pending.
    pub(crate) fn cancel(&mut self, request_id: &RequestId) -> Option<String> {
        self.pending.remove(request_id).map(|pending| {
            pending.cancellation_token.cancel();
            pending.method
        })
    }

    /// Marks the request as completed.
    /// Returns the start time and method name if the request was pending.
    pub(crate) fn complete(&mut self, request_id: &RequestId) -> Option<(Instant, String)> {
        self.pending
            .remove(request_id)
            .map(|pending| (pending.start_time, pending.method))
    }
}

/// A request from the client that hasn't been responded to yet.
#[derive(Debug)]
struct PendingRequest {
    start_time: Instant,
    method: String,
    cancellation_token: CancellationToken,
}

impl PendingRequest {
    fn new(method: String) -> Self {
        Self {
            start_time: Instant::now(),
            method,
            cancellation_token: CancellationToken::new(),
        }
    }
}

/// Token to cancel a specific request.
/// Can be shared between threads to check for cancellation.
#[derive(Debug, Clone)]
pub(crate) struct CancellationToken(Arc<AtomicBool>);

impl CancellationToken {
    pub(crate) fn new() -> Self {
        Self(Arc::new(AtomicBool::new(false)))
    }

    pub(crate) fn is_cancelled(&self) -> bool {
        self.0.load(Ordering::Relaxed)
    }

    fn cancel(&self) {
        self.0.store(true, Ordering::Relaxed);
    }
}

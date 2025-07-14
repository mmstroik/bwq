use anyhow::Result;
use bwq_server::server::{client::Client, handlers, session::Session};
use bwq_server::task::TaskExecutor;
use crossbeam_channel::{Receiver, Sender, bounded};
use lsp_server::{Connection, Message, Request};
use lsp_types::{HoverParams, Position, TextDocumentIdentifier, TextDocumentPositionParams};
use serde_json::Value;
use std::num::NonZeroUsize;

#[test]
fn test_unknown_request_method_handling() -> Result<()> {
    let mut session = Session::new(true);
    let (tx, _rx): (Sender<Message>, Receiver<Message>) = bounded(1);
    let (_req_tx, req_rx) = bounded(1);
    let connection = Connection {
        sender: tx,
        receiver: req_rx,
    };
    let client = Client::new(&connection);

    // minimal task executor
    let worker_threads = NonZeroUsize::new(1).unwrap();
    let (response_sender, _response_receiver) = crossbeam_channel::bounded(16);
    let task_executor = TaskExecutor::new(worker_threads, response_sender);

    // Test unknown request method
    let unknown_request = Request {
        id: lsp_server::RequestId::from(1),
        method: "unknown/method".to_string(),
        params: Value::Null,
    };

    // this should not panic
    let result = handlers::dispatch_request(&mut session, &client, &task_executor, unknown_request);
    assert!(
        result.is_ok(),
        "Unknown request method should be handled gracefully"
    );
    Ok(())
}

#[test]
fn test_hover_request_without_document() -> Result<()> {
    let mut session = Session::new(true); // Enable hover
    let (tx, _rx): (Sender<Message>, Receiver<Message>) = bounded(1);
    let (_req_tx, req_rx) = bounded(1);
    let connection = Connection {
        sender: tx,
        receiver: req_rx,
    };
    let client = Client::new(&connection);

    let worker_threads = NonZeroUsize::new(1).unwrap();
    let (response_sender, _response_receiver) = crossbeam_channel::bounded(16);
    let task_executor = TaskExecutor::new(worker_threads, response_sender);

    // Create hover request for non-existent document
    let hover_params = HoverParams {
        text_document_position_params: TextDocumentPositionParams {
            text_document: TextDocumentIdentifier {
                uri: "file:///nonexistent.bwq".parse().unwrap(),
            },
            position: Position {
                line: 0,
                character: 5,
            },
        },
        work_done_progress_params: Default::default(),
    };

    let hover_request = Request {
        id: lsp_server::RequestId::from(1),
        method: "textDocument/hover".to_string(),
        params: serde_json::to_value(hover_params)?,
    };

    let result = handlers::dispatch_request(&mut session, &client, &task_executor, hover_request);
    assert!(
        result.is_ok(),
        "Hover request for non-existent document should be handled gracefully"
    );
    Ok(())
}

#[test]
fn test_hover_disabled_handling() -> Result<()> {
    let mut session = Session::new(false); // Disable hover
    let (tx, _rx): (Sender<Message>, Receiver<Message>) = bounded(1);
    let (_req_tx, req_rx) = bounded(1);
    let connection = Connection {
        sender: tx,
        receiver: req_rx,
    };
    let client = Client::new(&connection);

    let worker_threads = NonZeroUsize::new(1).unwrap();
    let (response_sender, _response_receiver) = crossbeam_channel::bounded(16);
    let task_executor = TaskExecutor::new(worker_threads, response_sender);

    let hover_params = HoverParams {
        text_document_position_params: TextDocumentPositionParams {
            text_document: TextDocumentIdentifier {
                uri: "file:///test.bwq".parse().unwrap(),
            },
            position: Position {
                line: 0,
                character: 5,
            },
        },
        work_done_progress_params: Default::default(),
    };

    let hover_request = Request {
        id: lsp_server::RequestId::from(1),
        method: "textDocument/hover".to_string(),
        params: serde_json::to_value(hover_params)?,
    };

    let result = handlers::dispatch_request(&mut session, &client, &task_executor, hover_request);
    assert!(
        result.is_ok(),
        "Hover request should be handled when hover is disabled"
    );
    Ok(())
}

#[test]
fn test_entity_search_malformed_params() -> Result<()> {
    let mut session = Session::new(true);
    let (tx, _rx): (Sender<Message>, Receiver<Message>) = bounded(1);
    let (_req_tx, req_rx) = bounded(1);
    let connection = Connection {
        sender: tx,
        receiver: req_rx,
    };
    let client = Client::new(&connection);

    let worker_threads = NonZeroUsize::new(1).unwrap();
    let (response_sender, _response_receiver) = crossbeam_channel::bounded(16);
    let task_executor = TaskExecutor::new(worker_threads, response_sender);

    let malformed_request = Request {
        id: lsp_server::RequestId::from(1),
        method: "bwq/searchEntities".to_string(),
        params: serde_json::json!({"not_query": "apple"}), // Wrong field name
    };

    let result =
        handlers::dispatch_request(&mut session, &client, &task_executor, malformed_request);
    assert!(
        result.is_ok(),
        "Malformed entity search params should be handled gracefully"
    );
    Ok(())
}

#[test]
fn test_valid_entity_search_params() -> Result<()> {
    let mut session = Session::new(true);
    let (tx, _rx): (Sender<Message>, Receiver<Message>) = bounded(1);
    let (_req_tx, req_rx) = bounded(1);
    let connection = Connection {
        sender: tx,
        receiver: req_rx,
    };
    let client = Client::new(&connection);

    let worker_threads = NonZeroUsize::new(1).unwrap();
    let (response_sender, _response_receiver) = crossbeam_channel::bounded(16);
    let task_executor = TaskExecutor::new(worker_threads, response_sender);

    let valid_request = Request {
        id: lsp_server::RequestId::from(1),
        method: "bwq/searchEntities".to_string(),
        params: serde_json::json!({"query": "apple"}), // Correct field name
    };

    let result = handlers::dispatch_request(&mut session, &client, &task_executor, valid_request);
    assert!(
        result.is_ok(),
        "Valid entity search params should be handled gracefully"
    );
    Ok(())
}

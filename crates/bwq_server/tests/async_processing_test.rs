use anyhow::Result;
use bwq_server::task::{TaskExecutor, TaskResponse};
use crossbeam_channel::select;
use lsp_types::Uri;
use std::num::NonZeroUsize;
use std::time::Duration;

#[test]
fn test_async_diagnostics_processing() -> Result<()> {
    let worker_threads = NonZeroUsize::new(2).unwrap();
    let (response_sender, response_receiver) = crossbeam_channel::bounded(16);
    let task_executor = TaskExecutor::new(worker_threads, response_sender);
    let test_cases = vec![
        ("file:///test1.bwq", "apple AND juice"), // Valid query
        ("file:///test2.bwq", "rating:6 AND *invalid"), // Invalid query with errors
        ("file:///test3.bwq", ""),                // Empty query
    ];

    for (uri_str, content) in &test_cases {
        let uri = uri_str.parse::<Uri>().unwrap();
        task_executor.schedule_diagnostics_simple(uri, content.to_string())?;
    }

    let mut responses = Vec::new();
    let timeout = Duration::from_secs(5);

    for _ in 0..test_cases.len() {
        select! {
            recv(response_receiver) -> result => {
                match result {
                    Ok(TaskResponse::Diagnostics(params)) => {
                        responses.push(params);
                    }
                    Err(e) => {
                        panic!("Failed to receive response: {e}");
                    }
                }
            }
            default(timeout) => {
                panic!("Timeout waiting for async responses");
            }
        }
    }

    assert_eq!(responses.len(), 3, "Should receive 3 diagnostic responses");

    let received_uris: Vec<String> = responses.iter().map(|r| r.uri.to_string()).collect();

    for (uri_str, _) in &test_cases {
        assert!(
            received_uris.contains(&uri_str.to_string()),
            "Should have received response for {uri_str}"
        );
    }

    for response in &responses {
        match response.uri.path().as_str() {
            "/test1.bwq" => {
                let error_count = response
                    .diagnostics
                    .iter()
                    .filter(|d| d.severity == Some(lsp_types::DiagnosticSeverity::ERROR))
                    .count();
                assert_eq!(error_count, 0, "Valid query should not have errors");
            }
            "/test2.bwq" => {
                let error_count = response
                    .diagnostics
                    .iter()
                    .filter(|d| d.severity == Some(lsp_types::DiagnosticSeverity::ERROR))
                    .count();
                assert!(error_count > 0, "Invalid query should have errors");
            }
            "/test3.bwq" => {
                let error_count = response
                    .diagnostics
                    .iter()
                    .filter(|d| d.severity == Some(lsp_types::DiagnosticSeverity::ERROR))
                    .count();
                assert_eq!(error_count, 0, "Empty query should not have errors");
            }
            _ => panic!("Unexpected URI in response"),
        }
    }

    println!("✓ Async diagnostics processing test passed");
    Ok(())
}

#[test]
fn test_task_executor_creation() {
    let worker_threads = NonZeroUsize::new(1).unwrap();
    let (response_sender, _response_receiver) = crossbeam_channel::bounded(16);
    let _task_executor = TaskExecutor::new(worker_threads, response_sender);

    println!("✓ Task executor creation test passed");
}

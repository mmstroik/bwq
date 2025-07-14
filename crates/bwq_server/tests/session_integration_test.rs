use anyhow::Result;
use bwq_server::server::session::{AstState, DocumentState, Session};
use lsp_types::Uri;

#[test]
fn test_malformed_cancel_request_handling() -> Result<()> {
    let mut session = Session::new(true);

    let malformed_payloads = vec![
        serde_json::json!({}),                // missing id field
        serde_json::json!({"id": true}),      // wrong type for id
        serde_json::json!({"not_id": "123"}), // invalid JSON structure
        serde_json::json!({"id": null}),      // null values
    ];

    for payload in malformed_payloads {
        // this should not panic or crash
        let result = bwq_server::server::handlers::handle_cancel_request(&mut session, payload);
        assert!(
            result.is_ok(),
            "Malformed cancel request should not cause errors"
        );
    }

    let valid_payload = serde_json::json!({"id": "test-request-123"});
    let result = bwq_server::server::handlers::handle_cancel_request(&mut session, valid_payload);
    assert!(result.is_ok(), "Valid cancel request should succeed");
    Ok(())
}

#[test]
fn test_session_diagnostics_preparation() -> Result<()> {
    let mut session = Session::new(true);
    let uri: Uri = "file:///test.bwq".parse().unwrap();

    let result = session.prepare_diagnostics(&uri, "test content");
    assert!(result.is_none(), "Should return None for unknown document");

    let document_state = DocumentState {
        content: "apple AND juice".to_string(),
        version: 1,
        ast_state: AstState::NotParsed,
    };
    session.documents.insert(uri.clone(), document_state);

    let result = session.prepare_diagnostics(&uri, "updated content");
    assert!(
        result.is_some(),
        "Should return DiagnosticsRequest for existing document"
    );

    let diagnostics_request = result.unwrap();
    assert_eq!(diagnostics_request.uri, uri);
    assert_eq!(diagnostics_request.content, "updated content");
    assert_eq!(diagnostics_request.document_version, Some(1));

    // Verify document state was updated to Parsing
    assert!(matches!(
        session.documents.get(&uri).unwrap().ast_state,
        AstState::Parsing
    ));
    Ok(())
}

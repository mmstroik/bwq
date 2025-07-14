use anyhow::Result;
use lsp_server::{Notification, Request, Response};
use lsp_types::*;
use serde_json::Value;

use crate::server::client::Client;
use crate::server::session::{AstState, DocumentState, Session};
use crate::server::utils;
use crate::task::TaskExecutor;

#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct EntitySearchParams {
    pub query: String,
}

pub fn handle_did_close(
    session: &mut Session,
    client: &Client,
    params: DidCloseTextDocumentParams,
) -> Result<()> {
    let uri = params.text_document.uri;
    session.documents.remove(&uri);
    let had_cached_ast = session.ast_cache.pop(&uri).is_some();
    if had_cached_ast {
        tracing::debug!("Document closed: {:?} - removed from AST cache", uri);
    } else {
        tracing::debug!("Document closed: {:?} - no cached AST to remove", uri);
    }

    // clear diagnostics for closed document
    use lsp_server::Notification;
    use lsp_types::notification::{Notification as NotificationTrait, PublishDiagnostics};

    let diagnostics = PublishDiagnosticsParams {
        uri: uri.clone(),
        diagnostics: vec![],
        version: None,
    };

    let notification = Notification::new(
        PublishDiagnostics::METHOD.to_string(),
        serde_json::to_value(diagnostics)?,
    );

    client.send_notification(notification)?;
    tracing::debug!("Closed document: {uri:?}");
    Ok(())
}

pub fn handle_cancel_request(session: &mut Session, params: Value) -> Result<()> {
    let cancel_params: CancelParams = match serde_json::from_value(params) {
        Ok(params) => params,
        Err(e) => {
            tracing::debug!("Invalid cancel request params: {}", e);
            return Ok(()); // silently ignore malformed cancel requests
        }
    };

    let request_id = match cancel_params.id {
        lsp_types::NumberOrString::Number(n) => lsp_server::RequestId::from(n),
        lsp_types::NumberOrString::String(s) => lsp_server::RequestId::from(s),
    };

    if let Some(method) = session.request_queue.cancel(&request_id) {
        tracing::debug!("Cancelled request: {} (id: {:?})", method, request_id);
    } else {
        tracing::debug!("Cancel request for unknown id: {:?}", request_id);
    }

    Ok(())
}

pub fn handle_entity_search_request(
    client: &Client,
    task_executor: &TaskExecutor,
    req: Request,
) -> Result<()> {
    let params: EntitySearchParams = match serde_json::from_value(req.params) {
        Ok(params) => params,
        Err(e) => {
            let response = Response::new_err(
                req.id,
                lsp_server::ErrorCode::InvalidParams as i32,
                format!("Invalid entity search params: {e}"),
            );
            client.send_response(response)?;
            return Ok(());
        }
    };

    task_executor.schedule_entity_search(req.id, params.query)?;
    Ok(())
}

pub fn handle_did_open(
    session: &mut Session,
    task_executor: &TaskExecutor,
    params: DidOpenTextDocumentParams,
) -> Result<()> {
    let doc = params.text_document;

    let document_state = DocumentState {
        content: doc.text.clone(),
        version: doc.version,
        ast_state: AstState::NotParsed,
    };

    session.documents.insert(doc.uri.clone(), document_state);
    tracing::debug!("Document opened: {:?} - AST state: NotParsed", doc.uri);

    if let Some(diagnostics_request) = session.prepare_diagnostics(&doc.uri, &doc.text) {
        task_executor.schedule_diagnostics(
            diagnostics_request.uri,
            diagnostics_request.content,
            diagnostics_request.document_version,
            None,
        )?;
    }
    Ok(())
}

pub fn handle_did_change(
    session: &mut Session,
    task_executor: &TaskExecutor,
    params: DidChangeTextDocumentParams,
) -> Result<()> {
    let uri = params.text_document.uri;

    if let Some(document) = session.documents.get_mut(&uri) {
        if let Some(change) = params.content_changes.into_iter().next() {
            document.content = change.text.clone();
            document.version = params.text_document.version;
            document.ast_state = AstState::NotParsed;

            // Clear cached AST since content changed
            let had_cached_ast = session.ast_cache.pop(&uri).is_some();
            if had_cached_ast {
                tracing::debug!("Document changed: {:?} - AST cache invalidated", uri);
            } else {
                tracing::debug!("Document changed: {:?} - no cached AST to invalidate", uri);
            }

            if let Some(diagnostics_request) = session.prepare_diagnostics(&uri, &change.text) {
                task_executor.schedule_diagnostics(
                    diagnostics_request.uri,
                    diagnostics_request.content,
                    diagnostics_request.document_version,
                    None,
                )?;
            }
        }
    }

    Ok(())
}

pub fn handle_hover_request(
    session: &mut Session,
    client: &Client,
    task_executor: &TaskExecutor,
    req: Request,
) -> Result<()> {
    if !session.hover_enabled {
        let response = Response::new_ok(req.id, serde_json::Value::Null);
        client.send_response(response)?;
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
            client.send_response(response)?;
            return Ok(());
        }
    };

    let uri = params.text_document_position_params.text_document.uri;
    let position = params.text_document_position_params.position;

    // Get document content and calculate byte position
    let (content, byte_position) = match session.documents.get(&uri) {
        Some(doc) => {
            let content = doc.content.clone();
            let byte_position = utils::lsp_position_to_byte_position(&content, position);
            (content, byte_position)
        }
        None => {
            let response = Response::new_ok(req.id, serde_json::Value::Null);
            client.send_response(response)?;
            return Ok(());
        }
    };

    let entity_id = session.find_entity_id_at_position(&uri, byte_position);

    // If no entity found and no cached AST, check document state for parsing status
    if entity_id.is_none() && !session.ast_cache.contains(&uri) {
        match session.documents.get(&uri).map(|doc| &doc.ast_state) {
            Some(AstState::NotParsed) => {
                tracing::debug!("HOVER: AST not parsed yet, triggering parse and returning null");
                if let Some(diagnostics_request) = session.prepare_diagnostics(&uri, &content) {
                    task_executor.schedule_diagnostics(
                        diagnostics_request.uri,
                        diagnostics_request.content,
                        diagnostics_request.document_version,
                        None,
                    )?;
                }
                let response = Response::new_ok(req.id, serde_json::Value::Null);
                client.send_response(response)?;
                return Ok(());
            }
            Some(AstState::Parsing) => {
                tracing::debug!("HOVER: AST currently parsing, returning null");
                let response = Response::new_ok(req.id, serde_json::Value::Null);
                client.send_response(response)?;
                return Ok(());
            }
            Some(AstState::Cached) => {
                tracing::debug!("HOVER: AST state says cached but not in cache, returning null");
            }
            None => {}
        }
    }

    if let Some(entity_id) = entity_id {
        // Schedule entity lookup task
        task_executor.schedule_entity_lookup(req.id, entity_id)?;
    } else {
        let response = Response::new_ok(req.id, serde_json::Value::Null);
        client.send_response(response)?;
    }

    Ok(())
}

// Direct dispatch functions - no trait wrapper indirection
pub fn dispatch_request(
    session: &mut Session,
    client: &Client,
    task_executor: &TaskExecutor,
    req: Request,
) -> Result<()> {
    match req.method.as_str() {
        "initialize" | "shutdown" => {
            // These are handled elsewhere in the protocol flow
            Ok(())
        }
        "textDocument/hover" => handle_hover_request(session, client, task_executor, req),
        "bwq/searchEntities" => handle_entity_search_request(client, task_executor, req),
        _ => {
            let response = Response::new_err(
                req.id,
                lsp_server::ErrorCode::MethodNotFound as i32,
                "Method not found".to_string(),
            );
            client.send_response(response)?;
            Ok(())
        }
    }
}

pub fn dispatch_notification(
    session: &mut Session,
    client: &Client,
    task_executor: &TaskExecutor,
    not: Notification,
) -> Result<()> {
    match not.method.as_str() {
        "textDocument/didOpen" => {
            let params: DidOpenTextDocumentParams = serde_json::from_value(not.params)?;
            handle_did_open(session, task_executor, params)
        }
        "textDocument/didChange" => {
            let params: DidChangeTextDocumentParams = serde_json::from_value(not.params)?;
            handle_did_change(session, task_executor, params)
        }
        "textDocument/didClose" => {
            let params: DidCloseTextDocumentParams = serde_json::from_value(not.params)?;
            handle_did_close(session, client, params)
        }
        "$/cancelRequest" => handle_cancel_request(session, not.params),
        _ => {
            tracing::debug!("Unknown notification: {}", not.method);
            Ok(())
        }
    }
}

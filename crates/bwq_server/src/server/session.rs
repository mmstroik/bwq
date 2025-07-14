use std::collections::HashMap;
use std::num::NonZeroUsize;

use bwq_linter::ast::Query;
use lru::LruCache;
use lsp_types::Uri;

use crate::request_queue::RequestQueue;

#[derive(Debug, Clone)]
pub enum AstState {
    NotParsed,
    Parsing,
    Cached,
}

#[derive(Debug, Clone)]
pub struct DocumentState {
    pub content: String,
    pub version: i32,
    pub ast_state: AstState,
}

#[derive(Debug)]
pub struct DiagnosticsRequest {
    pub uri: Uri,
    pub content: String,
    pub document_version: Option<i32>,
}

/// Manages the server's business logic state
pub struct Session {
    pub documents: HashMap<Uri, DocumentState>,
    pub ast_cache: LruCache<Uri, Query>,
    pub request_queue: RequestQueue,
    pub hover_enabled: bool,
}

impl Session {
    pub fn new(hover_enabled: bool) -> Self {
        Self {
            documents: HashMap::new(),
            ast_cache: LruCache::new(NonZeroUsize::new(10).unwrap()),
            request_queue: RequestQueue::new(),
            hover_enabled,
        }
    }

    /// Find entity ID in cached AST (updates LRU order)
    pub fn find_entity_id_at_position(
        &mut self,
        uri: &lsp_types::Uri,
        position: usize,
    ) -> Option<String> {
        let cache_len = self.ast_cache.len();
        let cache_cap = self.ast_cache.cap().get();

        if let Some(ast) = self.ast_cache.get(uri) {
            tracing::debug!(
                "HOVER: Using cached AST for entity extraction (cache size: {}/{})",
                cache_len,
                cache_cap
            );
            Self::find_entity_id_in_expression(&ast.expression, position)
        } else {
            None
        }
    }

    /// Prepare diagnostics processing and update document state
    pub fn prepare_diagnostics(
        &mut self,
        uri: &lsp_types::Uri,
        content: &str,
    ) -> Option<DiagnosticsRequest> {
        let document_version = self.documents.get(uri).map(|doc| doc.version);

        // Mark document as parsing
        if let Some(document) = self.documents.get_mut(uri) {
            document.ast_state = AstState::Parsing;
            tracing::debug!("Document parsing started: {:?} - AST state: Parsing", uri);

            Some(DiagnosticsRequest {
                uri: uri.clone(),
                content: content.to_string(),
                document_version,
            })
        } else {
            None
        }
    }

    fn find_entity_id_in_expression(
        expr: &bwq_linter::ast::Expression,
        position: usize,
    ) -> Option<String> {
        use bwq_linter::ast::{Expression, FieldType};

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

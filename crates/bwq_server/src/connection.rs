use anyhow::Result;
use lsp_server::{self as lsp, Connection};
use lsp_types::{
    InitializeParams, ServerCapabilities, TextDocumentSyncCapability, TextDocumentSyncKind,
};

pub(crate) struct ConnectionInitializer {
    connection: Connection,
}

impl ConnectionInitializer {
    pub(crate) fn stdio() -> (Self, lsp::IoThreads) {
        let (connection, threads) = Connection::stdio();
        (Self { connection }, threads)
    }

    pub(crate) fn initialize_start(&self) -> Result<(lsp::RequestId, InitializeParams)> {
        let (id, params) = self.connection.initialize_start()?;
        Ok((id, serde_json::from_value(params)?))
    }

    pub(crate) fn initialize_finish(
        self,
        id: lsp::RequestId,
        server_capabilities: ServerCapabilities,
        name: &str,
        version: &str,
    ) -> Result<Connection> {
        self.connection.initialize_finish(
            id,
            serde_json::json!({
                "capabilities": server_capabilities,
                "serverInfo": {
                    "name": name,
                    "version": version
                }
            }),
        )?;
        Ok(self.connection)
    }
}

pub(crate) fn server_capabilities() -> ServerCapabilities {
    ServerCapabilities {
        text_document_sync: Some(TextDocumentSyncCapability::Kind(TextDocumentSyncKind::FULL)),
        ..Default::default()
    }
}

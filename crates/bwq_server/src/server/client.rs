use anyhow::Result;
use lsp_server::{Connection, Message, Notification, Response};

/// Simple wrapper for LSP client communication
/// Separates transport concerns from business logic
pub struct Client<'a> {
    connection: &'a Connection,
}

impl<'a> Client<'a> {
    pub fn new(connection: &'a Connection) -> Self {
        Self { connection }
    }

    pub fn send_response(&self, response: Response) -> Result<()> {
        self.connection.sender.send(Message::Response(response))?;
        Ok(())
    }

    pub fn send_notification(&self, notification: Notification) -> Result<()> {
        self.connection
            .sender
            .send(Message::Notification(notification))?;
        Ok(())
    }
}

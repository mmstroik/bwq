mod diagnostics_handler;
mod server;
mod utils;
use crate::server::Server;

pub(crate) type Result<T> = anyhow::Result<T>;
pub fn run() -> Result<()> {
    let server_result = Server::run();

    let result = match server_result {
        Ok(()) => Ok(()),
        Err(server) => Err(server),
    };

    if let Err(err) = result.as_ref() {
        tracing::warn!("Server shut down with an error: {err}");
    } else {
        tracing::info!("Server shut down");
    }

    result
}

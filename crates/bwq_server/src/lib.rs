mod connection;
mod diagnostics_handler;
mod request_queue;
mod server;
pub mod task;
mod utils;
mod wikidata;

use crate::connection::ConnectionInitializer;
use crate::server::Server;
use anyhow::Context;
use std::num::NonZeroUsize;

pub(crate) type Result<T> = anyhow::Result<T>;

pub fn run() -> Result<()> {
    tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .with_max_level(tracing::Level::INFO)
        .init();

    let four = NonZeroUsize::new(4).unwrap();

    // set number of threads to num_cpus with a maximum of 4
    let worker_threads = std::thread::available_parallelism()
        .unwrap_or(four)
        .min(four);

    let (connection_initializer, io_threads) = ConnectionInitializer::stdio();

    let server_result = Server::new(worker_threads, connection_initializer)
        .context("Failed to start server")?
        .run();

    let io_result = io_threads.join();

    let result = match (server_result, io_result) {
        (Ok(()), Ok(())) => Ok(()),
        (Err(server), Err(io)) => Err(server).context(format!("IO thread error: {io}")),
        (Err(server), _) => Err(server),
        (_, Err(io)) => Err(io).context("IO thread error"),
    };

    if let Err(err) = result.as_ref() {
        tracing::warn!("Server shut down with an error: {err}");
    } else {
        tracing::info!("Server shut down");
    }

    result
}

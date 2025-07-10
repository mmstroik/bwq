use crate::ExitStatus;
use bwq_server::LspServer;

pub fn run_server() -> Result<ExitStatus, anyhow::Error> {
    match LspServer::run() {
        Ok(()) => Ok(ExitStatus::Success),
        Err(e) => {
            eprintln!("Server error: {e}");
            Ok(ExitStatus::Error)
        }
    }
}

use crate::ExitStatus;
use anyhow::Result;

pub(crate) fn run_server() -> Result<ExitStatus> {
    bwq_server::run()?;
    Ok(ExitStatus::Success)
}

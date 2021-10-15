use anyhow::{Context, Error, Result};
use nix::sys::signal;

use crate::runs::{Run, RunDataState};

pub fn send_signal(run: &Run, signal: signal::Signal) -> Result<()> {
    match run.get_data()?.state {
        RunDataState::Running { pid } => {
            signal::kill(pid, signal).with_context(|| "Couldn't send signal to run's process")
        }
        RunDataState::Done { .. } => Err(Error::msg(format!("Still running: {}", run.id))),
    }
}

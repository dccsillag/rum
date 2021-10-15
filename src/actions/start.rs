use anyhow::{Error, Result};

use crate::runs::Runs;

pub fn start_run(runs: &Runs, command: Vec<String>, label: Option<String>) -> Result<()> {
    if command.is_empty() {
        return Err(Error::msg("Given command is empty"));
    }

    runs.new_run()?.start(command, label)
}

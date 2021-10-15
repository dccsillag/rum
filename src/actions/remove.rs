use anyhow::{Error, Result};

use crate::runs::{Run, RunData, RunDataState, Runs};

pub fn remove_run(runs: &Runs, run: Run) -> Result<()> {
    match run.get_data()? {
        RunData {
            label,
            command,
            start_datetime,
            state:
                RunDataState::Done {
                    end_datetime,
                    exit_code,
                },
        } => {
            label.map(|l| println!("Label: {}", l));
            println!("Command: {}", shell_words::join(command));
            println!("Started running: {}", start_datetime);
            println!("Finished running: {}", end_datetime);
            println!("Exit code: {}", exit_code);

            if dialoguer::Confirm::with_theme(&dialoguer::theme::ColorfulTheme::default())
                .with_prompt("Are you sure you want to delete this run?")
                .interact()?
            {
                runs.remove_run(run)?;
                println!("Deleted.");
            }
            Ok(())
        }
        RunData {
            state: RunDataState::Running { .. },
            ..
        } => Err(Error::msg(format!("Still running: {}", run.id))),
    }
}

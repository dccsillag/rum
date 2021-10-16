use anyhow::{Error, Result};

use crate::{
    actions::show_info::show_run_info,
    runs::{Run, RunData, RunDataState, Runs},
};

pub fn remove_run(runs: &Runs, run: Run) -> Result<()> {
    match run.get_data()? {
        RunData {
            state: RunDataState::Done { .. },
            ..
        } => {
            show_run_info(&run)?;

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

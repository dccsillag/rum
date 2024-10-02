use anyhow::{Error, Result};
use colored::Colorize;

use crate::{
    actions::show_info::show_run_info,
    runs::{Run, RunData, RunDataState, Runs},
};

pub fn remove_run(runs: &Runs, run: Run, ask_for_confirmation: bool) -> Result<()> {
    match run.get_data()? {
        RunData {
            state: RunDataState::Done { .. },
            ..
        } => {
            if ask_for_confirmation {
                show_run_info(&run)?;

                println!();

                if dialoguer::Confirm::with_theme(&dialoguer::theme::ColorfulTheme::default())
                    .with_prompt("Are you sure you want to delete this run?")
                    .interact()?
                {
                    let id = run.id.clone();
                    runs.remove_run(run)?;
                    println!("Deleted run '{id}'.");
                }
            } else {
                let id = run.id.clone();
                runs.remove_run(run)?;
                println!("Deleted run '{id}'.");
            }
            Ok(())
        }
        RunData {
            state: RunDataState::Running { .. },
            ..
        } => Err(Error::msg(format!("Still running: {}", run.id))),
    }
}

pub fn remove_runs(
    runs: &Runs,
    runs_to_remove: &[String],
    ask_for_confirmation: bool,
) -> Result<()> {
    let (good_runs, bad_runs): (Vec<_>, Vec<_>) = runs_to_remove
        .iter()
        .map(|r| runs.get_run(r))
        .partition(Result::is_ok);
    let good_runs = good_runs.into_iter().map(Result::unwrap);
    let bad_runs = bad_runs.into_iter().map(Result::unwrap_err);

    for run in good_runs {
        remove_run(runs, run, ask_for_confirmation)?;
    }

    for error in bad_runs {
        println!("{} {}", "ERROR".red().bold(), error,)
    }

    Ok(())
}

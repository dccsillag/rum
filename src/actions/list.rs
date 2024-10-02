use anyhow::Result;
use colored::Colorize;

use crate::{
    runs::{RunData, RunDataState, Runs},
    utils::format_datetime,
};

pub fn list_runs(runs: &Runs) -> Result<()> {
    let (runs, bad_runs): (Vec<_>, Vec<_>) = runs
        .get_all()?
        .iter()
        .map(|r| {
            r.get_data()
                .map(|d| (r.id.clone(), d))
                .map_err(|_| r.id.clone())
        })
        .partition(Result::is_ok);
    let mut runs = runs.into_iter().map(Result::unwrap).collect::<Vec<_>>();
    let bad_runs = bad_runs.into_iter().map(Result::unwrap_err);
    runs.sort_by_key(|(_, r)| r.start_datetime);
    runs.sort_by_key(|(_, r)| match r.state {
        RunDataState::Running { .. } => 0,
        RunDataState::Done { .. } => 1,
    });

    for bad_run in bad_runs {
        // TODO change into logging
        println!(
            "{}: Could not read run '{}'; ignoring it.",
            "WARNING".bold().yellow(),
            bad_run,
        );
    }

    for (
        run_id,
        RunData {
            label,
            command,
            start_datetime,
            state,
        },
    ) in runs.into_iter()
    {
        print!("{} ", &run_id[..8]);
        match state {
            RunDataState::Done { exit_code: 0, .. } => {
                print!("{}", "[done] ".green().bold())
            }
            RunDataState::Done { exit_code: -1, .. } => {
                print!("{}", "[killed] ".yellow().bold())
            }
            RunDataState::Done { exit_code: -2, .. } => {
                print!("{}", "[crashed] ".magenta().bold())
            }
            RunDataState::Done { exit_code, .. } => {
                print!("{}", format!("[failed:{exit_code}] ").red().bold())
            }
            RunDataState::Running { .. } => {
                print!("{}", "[running] ".bold())
            }
        }
        println!("{}", shell_words::join(command).bold(),);
        print!("         ");
        match state {
            RunDataState::Done { end_datetime, .. } => {
                println!(
                    "{} {}, {} {}",
                    "Started".dimmed(),
                    format_datetime(start_datetime),
                    "Finished".dimmed(),
                    format_datetime(end_datetime),
                )
            }
            RunDataState::Running { .. } => {
                println!("{} {}", "Started".dimmed(), format_datetime(start_datetime),);
            }
        }
    }

    Ok(())
}

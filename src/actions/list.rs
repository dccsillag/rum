use anyhow::Result;
use termion::{color, style};

use crate::{
    runs::{RunData, RunDataState, Runs},
    utils::format_datetime,
};

pub fn list_runs(runs: &Runs) -> Result<()> {
    let (runs, bad_runs): (Vec<_>, Vec<_>) = runs
        .get_all()?
        .iter()
        .map(|r| r.get_data().map(|d| (r.id.clone(), d)).map_err(|_| r.id.clone()))
        .partition(Result::is_ok);
    let mut runs = runs.into_iter().map(Result::unwrap).collect::<Vec<_>>();
    let bad_runs = bad_runs
        .into_iter()
        .map(Result::unwrap_err)
        .collect::<Vec<_>>();
    runs.sort_by_key(|(_, r)| r.start_datetime);
    runs.sort_by_key(|(_, r)| match r.state {
        RunDataState::Running { .. } => 0,
        RunDataState::Done { .. } => 1,
    });

    for bad_run in bad_runs.into_iter() {
        // TODO change into logging
        println!(
            "{}{}WARNING{}{}: Could not read run '{}'; ignoring it.",
            style::Bold,
            color::Fg(color::Yellow),
            color::Fg(color::Reset),
            style::Reset,
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
        print!("{} {}", &run_id[..8], style::Bold);
        match state {
            RunDataState::Done { exit_code: 0, .. } => {
                print!("{}[done] ", color::Fg(color::Green))
            }
            RunDataState::Done { exit_code: -1, .. } => {
                print!("{}[killed] ", color::Fg(color::Yellow))
            }
            RunDataState::Done { exit_code, .. } => {
                print!("{}[failed:{}] ", color::Fg(color::Red), exit_code)
            }
            RunDataState::Running { .. } => {
                print!("[running] ")
            }
        }
        println!(
            "{}{}{}",
            color::Fg(color::Reset),
            shell_words::join(command),
            style::Reset
        );
        print!("         ");
        match state {
            RunDataState::Done { end_datetime, .. } => {
                println!(
                    "{}Started{} {}, {}Finished{} {}",
                    style::Faint,
                    style::Reset,
                    format_datetime(start_datetime),
                    style::Faint,
                    style::Reset,
                    format_datetime(end_datetime),
                )
            }
            RunDataState::Running { .. } => {
                println!(
                    "{}Started{} {}",
                    style::Faint,
                    style::Reset,
                    format_datetime(start_datetime),
                );
            }
        }
    }

    Ok(())
}

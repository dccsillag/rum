use anyhow::Result;
use termion::{color, style};

use crate::runs::{RunData, RunDataState, RunId, Runs};

pub fn list_runs(runs: &Runs) -> Result<()> {
    let mut runs = runs
        .get_all()?
        .iter()
        .filter_map(|(i, r)| r.get_data().map(|d| (i.clone(), d)).ok())
        .collect::<Vec<(RunId, RunData)>>();
    runs.sort_by_key(|(_, r)| r.start_datetime);
    runs.sort_by_key(|(_, r)| match r.state {
        RunDataState::Running { .. } => 0,
        RunDataState::Done { .. } => 1,
    });

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
                    start_datetime.format("%c"),
                    style::Faint,
                    style::Reset,
                    end_datetime.format("%c")
                )
            }
            RunDataState::Running { .. } => {
                println!(
                    "{}Started{} {}",
                    style::Faint,
                    style::Reset,
                    start_datetime.format("%c")
                );
            }
        }
    }

    Ok(())
}

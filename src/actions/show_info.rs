use anyhow::Result;
use termion::color;

use crate::{runs::{Run, RunData, RunDataState}, utils::format_datetime};

pub fn show_run_info(run: &Run) -> Result<()> {
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
            println!("Command:   {}", shell_words::join(command));
            label.map(|l| println!("Label:     {}", l));
            println!("Status:    finished");
            println!(
                "Exit code: {}",
                match exit_code {
                    0 => format!(
                        "0 ({}success{})",
                        color::Fg(color::Green),
                        color::Fg(color::Reset)
                    ),
                    -1 => format!(
                        "none ({}killed{})",
                        color::Fg(color::Yellow),
                        color::Fg(color::Reset)
                    ),
                    -2 => format!(
                        "none ({}crashed{})",
                        color::Fg(color::Magenta),
                        color::Fg(color::Reset)
                    ),
                    c => format!(
                        "{} ({}failed{})",
                        c,
                        color::Fg(color::Red),
                        color::Fg(color::Reset)
                    ),
                }
            );
            println!("Started:   {}", format_datetime(start_datetime));
            println!("Finished:  {}", format_datetime(end_datetime));
        }
        RunData {
            label,
            command,
            start_datetime,
            state: RunDataState::Running { pgid: _ },
        } => {
            println!("Command:   {}", shell_words::join(command));
            label.map(|l| println!("Label:     {}", l));
            println!("Status:    running");
            println!("Started:   {}", format_datetime(start_datetime));
        }
    }
    Ok(())
}

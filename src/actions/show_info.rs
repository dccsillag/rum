use anyhow::Result;
use colored::Colorize;

use crate::{
    runs::{Run, RunData, RunDataState},
    utils::format_datetime,
};

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
            if let Some(label) = label {
                println!("Label:     {label}");
            }
            println!("Status:    finished");
            println!(
                "Exit code: {}",
                match exit_code {
                    0 => format!("0 ({})", "success".green()),
                    -1 => format!("none ({})", "killed".yellow()),
                    -2 => format!("none ({})", "crashed".magenta()),
                    c => format!("{} ({})", c, "failed".red()),
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
            if let Some(label) = label {
                println!("Label:     {label}");
            }
            println!("Status:    running");
            println!("Started:   {}", format_datetime(start_datetime));
        }
    }
    Ok(())
}

pub mod runs;
pub mod utils;

use std::io::Write;

use anyhow::{Context, Error, Result};
use chrono::{DateTime, Utc};
use nix::sys::signal;
use structopt::StructOpt;
use tabled::{Table, Tabled};

use runs::{Run, RunData, RunDataState, RunId, Runs};
use termion::{event::Key, input::TermRead, raw::IntoRawMode};

#[derive(StructOpt)]
#[structopt(name = "rum", about = "A tool to manage running jobs.")]
enum Args {
    /// Start a new run
    Start {
        /// Command to run
        command: Vec<String>,

        /// Optional label for this run
        #[structopt(short, long)]
        label: Option<String>,
    },

    /// List runs
    List,

    /// Open a run
    OpenRun {
        /// Which run to open
        run: RunId,
    },

    /// Delete a run
    DeleteRun {
        /// Which run to delete
        run: RunId,
    },

    /// Interrupt (Ctrl+C) a running run
    InterruptRun {
        /// Which run to open
        run: RunId,
    },

    /// Terminate (SIGTERM) a running run
    TerminateRun {
        /// Which run to open
        run: RunId,
    },

    /// Kill (SIGKILL) a running run
    KillRun {
        /// Which run to open
        run: RunId,
    },
}

fn start(runs: &Runs, command: Vec<String>, label: Option<String>) -> Result<()> {
    if command.is_empty() {
        return Err(Error::msg("Given command is empty"));
    }

    runs.new_run()?.start(command, label)
}

fn list_runs(runs: &Runs) -> Result<()> {
    let mut runs = runs
        .get_all()?
        .iter()
        .filter_map(|(i, r)| r.get_data().map(|d| (i.clone(), d)).ok())
        .collect::<Vec<(RunId, RunData)>>();
    runs.sort_by_key(|(_, r)| r.start_datetime);

    fn display_option<T>(o: &Option<T>) -> String
    where
        T: std::fmt::Display,
    {
        match o {
            Some(s) => format!("{}", s),
            None => "".to_string(),
        }
    }

    #[derive(Tabled)]
    struct Row {
        #[header("ID")]
        id: RunId,
        #[header("Status")]
        status: String,
        #[header("Label")]
        #[field(display_with = "display_option")]
        label: Option<String>,
        #[header("Command")]
        command: String,
        #[header("Start DateTime")]
        start_datetime: DateTime<Utc>,
        #[header("End DateTime")]
        #[field(display_with = "display_option")]
        end_datetime: Option<DateTime<Utc>>,
    }

    let rows = runs
        .into_iter()
        .map(
            |(
                run_id,
                RunData {
                    label,
                    command,
                    start_datetime,
                    state,
                },
            )| match state {
                RunDataState::Done {
                    exit_code,
                    end_datetime,
                } => Row {
                    id: run_id,
                    label,
                    status: format!("done (exit code = {})", exit_code),
                    command: shell_words::join(command),
                    end_datetime: Some(end_datetime),
                    start_datetime,
                },
                RunDataState::Running { pid: _pid } => Row {
                    id: run_id,
                    label,
                    status: "running".to_string(),
                    command: shell_words::join(command),
                    end_datetime: None,
                    start_datetime,
                },
            },
        )
        .collect::<Vec<Row>>();

    let table = Table::new(rows).with(tabled::Style::noborder());

    print!("{}", table);

    Ok(())
}

fn delete(runs: &Runs, run: Run) -> Result<()> {
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

fn open_run(run: &Run) -> Result<()> {
    let output_file_path = run.get_output_file();

    let mut screen = termion::screen::AlternateScreen::from(std::io::stdout()).into_raw_mode()?;
    let mut input = termion::async_stdin().keys();

    utils::tail::follow_tail(
        &output_file_path,
        |new_text: &str| -> Result<()> {
            let new_text = new_text.replace('\n', "\r\n");
            write!(screen, "{}", new_text)?;

            // FIXME what if the output is already styled?
            write!(
                screen,
                "{}{}{}{}",
                termion::cursor::Save,
                termion::cursor::Goto(1, 1),
                termion::clear::CurrentLine,
                termion::style::Faint,
            )?;
            write!(
                screen,
                "You are currently viewing a run. Press Ctrl+C to exit."
            )?;
            write!(
                screen,
                "{}",
                termion::cursor::Goto(termion::terminal_size()?.0 - (run.id.len() as u16) + 1, 1),
            )?;
            write!(screen, "{}", run.id)?;
            write!(
                screen,
                "{}{}",
                termion::style::NoFaint,
                termion::cursor::Restore
            )?;

            screen.flush()?;

            Ok(())
        },
        || {
            while let Some(key) = input.next() {
                match key? {
                    Key::Ctrl('c') => return Ok(true),
                    _ => (),
                }
            }
            Ok(false)
        },
    )
}

pub fn send_signal(run: &Run, signal: signal::Signal) -> Result<()> {
    match run.get_data()?.state {
        RunDataState::Running { pid } => {
            signal::kill(pid, signal).with_context(|| "Couldn't send signal to run's process")
        }
        RunDataState::Done { .. } => Err(Error::msg(format!("Still running: {}", run.id))),
    }
}

fn main() -> Result<()> {
    let args = Args::from_args();

    let runs = Runs::new().with_context(|| "Could not acquire runs")?;

    match args {
        Args::Start { command, label } => start(&runs, command, label),
        Args::List => list_runs(&runs),
        Args::OpenRun { run } => open_run(&runs.get_run(&run)?),
        Args::DeleteRun { run } => delete(&runs, runs.get_run(&run)?),
        Args::InterruptRun { run } => send_signal(&runs.get_run(&run)?, signal::Signal::SIGINT),
        Args::TerminateRun { run } => send_signal(&runs.get_run(&run)?, signal::Signal::SIGTERM),
        Args::KillRun { run } => send_signal(&runs.get_run(&run)?, signal::Signal::SIGKILL),
    }
}

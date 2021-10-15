pub mod runs;

use anyhow::{Context, Error, Result};
use chrono::{DateTime, Utc};
use fork::{daemon, Fork};
use nix::{sys::signal};
use structopt::StructOpt;
use tabled::{Table, Tabled};

use runs::{Run, RunData, RunDoneData, RunId, Runs};

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

        #[structopt(short = "i")]
        interactable: bool,
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

    if let Ok(Fork::Child) = daemon(true, false) {
        let run = runs.new_run()?;
        run.start(command, label)?;
    }

    Ok(())
}

fn list(runs: &Runs) -> Result<()> {
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
                    done_data,
                    pid: _pid,
                },
            )| match done_data {
                Some(done_data) => Row {
                    id: run_id,
                    label,
                    status: "done".to_string(), // TODO check exit code
                    command: shell_words::join(command),
                    end_datetime: Some(done_data.end_datetime),
                    start_datetime,
                },
                None => Row {
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
            done_data:
                Some(RunDoneData {
                    end_datetime,
                    exit_code,
                }),
            pid: _pid,
        } => {
            label.map(|l| println!("Label: {}", l));
            println!("Command: {}", shell_words::join(command));
            println!("Started running: {}", start_datetime);
            println!("Finished running: {}", end_datetime);
            println!("Exit code: {}", exit_code.code().unwrap_or(-1));

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
            done_data: None, ..
        } => Err(Error::msg(format!("Still running: {}", run.id))),
    }
}

fn main() -> Result<()> {
    let args = Args::from_args();

    let runs = Runs::new().with_context(|| "Could not acquire runs")?;

    match args {
        Args::Start { command, label } => start(&runs, command, label),
        Args::List => list(&runs),
        Args::OpenRun { run, interactable } => runs.get_run(&run)?.open(interactable),
        Args::DeleteRun { run } => delete(&runs, runs.get_run(&run)?),
        Args::InterruptRun { run } => runs.get_run(&run)?.send_signal(signal::Signal::SIGINT),
        Args::TerminateRun { run } => runs.get_run(&run)?.send_signal(signal::Signal::SIGTERM),
        Args::KillRun { run } => runs.get_run(&run)?.send_signal(signal::Signal::SIGKILL),
    }
}

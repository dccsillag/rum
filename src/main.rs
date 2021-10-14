pub mod runs;

use std::{convert::TryInto, os::unix::prelude::{AsRawFd, FromRawFd}};

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use fork::{daemon, Fork};
use nix::{sys::signal, unistd::Pid};
use structopt::StructOpt;
use tabled::{Table, Tabled};

use runs::{RunData, RunDoneData, RunId, Runs};

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
    // TODO check that `command` is valid

    if let Ok(Fork::Child) = daemon(true, false) {
        let run = runs.new_run()?;

        let output_file_path = run.get_output_file();
        let output_file = std::fs::File::create(&output_file_path)?;
        let output_file_raw = output_file.as_raw_fd();

        let mut process: std::process::Child = std::process::Command::new(command.first().unwrap())
            .args(&command[1..])
            .stdout(unsafe { std::process::Stdio::from_raw_fd(output_file_raw) })
            .stderr(unsafe { std::process::Stdio::from_raw_fd(output_file_raw) })
            .stdin(std::process::Stdio::null())
            .spawn()?;

        run.set_data(&RunData {
            command: command,
            label: label,
            done_data: None,
            start_datetime: Utc::now(),

            pid: Pid::from_raw(process.id().try_into().unwrap()),
            output_file: output_file_path,
        })?;

        let exit_status = process.wait()?;

        run.update_data(|run_data| {
            Ok(RunData {
                done_data: Some(RunDoneData {
                    exit_code: exit_status,
                    end_datetime: Utc::now(),
                }),
                ..run_data
            })
        })?;
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
                    output_file: _output_file,
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

fn open(run: &RunData, interactable: bool) -> Result<()> {
    todo!()
}

fn send_signal(run: &RunData, signal: signal::Signal) -> Result<()> {
    signal::kill(run.pid, signal).with_context(|| "Couldn't send signal to run's process")
}

fn main() -> Result<()> {
    let args = Args::from_args();

    let runs = Runs::new().with_context(|| "Could not acquire runs")?;

    match args {
        Args::Start { command, label } => start(&runs, command, label),
        Args::List => list(&runs),
        Args::OpenRun { run, interactable } => open(&runs.get_run(&run)?.get_data()?, interactable),
        Args::InterruptRun { run } => {
            send_signal(&runs.get_run(&run)?.get_data()?, signal::Signal::SIGINT)
        }
        Args::TerminateRun { run } => {
            send_signal(&runs.get_run(&run)?.get_data()?, signal::Signal::SIGTERM)
        }
        Args::KillRun { run } => {
            send_signal(&runs.get_run(&run)?.get_data()?, signal::Signal::SIGKILL)
        }
    }
}

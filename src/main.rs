pub mod actions;
pub mod runs;
pub mod utils;

use anyhow::{Context, Result};
use nix::sys::signal;
use structopt::StructOpt;

use runs::{RunId, Runs};

#[derive(StructOpt)]
#[structopt(name = "rum", about = "A tool to manage running jobs.")]
struct Args {
    #[structopt(subcommand)]
    subcommand: Subcommand,
}

#[derive(StructOpt)]
enum Subcommand {
    /// Start a new run
    #[structopt(name = "start")]
    Start {
        /// Command to run
        command: Vec<String>,

        /// Optional label for this run
        #[structopt(short, long)]
        label: Option<String>,
    },

    /// List runs
    #[structopt(name = "list")]
    List,

    /// Open a run
    #[structopt(name = "view")]
    OpenRun {
        /// Which run to open
        run: RunId,
    },

    /// Delete a run
    #[structopt(name = "remove")]
    DeleteRun {
        /// Which run to delete
        run: RunId,
    },

    /// Interrupt (Ctrl+C) a running run
    #[structopt(name = "interrupt")]
    InterruptRun {
        /// Which run to open
        run: RunId,
    },

    /// Terminate (SIGTERM) a running run
    #[structopt(name = "terminate")]
    TerminateRun {
        /// Which run to open
        run: RunId,
    },

    /// Kill (SIGKILL) a running run
    #[structopt(name = "kill")]
    KillRun {
        /// Which run to open
        run: RunId,
    },
}

fn main() -> Result<()> {
    let args = Args::from_args();

    let runs = Runs::new().with_context(|| "Could not acquire runs")?;

    match args.subcommand {
        Subcommand::Start { command, label } => actions::start::start_run(&runs, command, label),
        Subcommand::List => actions::list::list_runs(&runs),
        Subcommand::OpenRun { run } => actions::open::open_run(&runs.get_run(&run)?),
        Subcommand::DeleteRun { run } => actions::remove::remove_run(&runs, runs.get_run(&run)?),
        Subcommand::InterruptRun { run } => {
            actions::send_signal::send_signal(&runs.get_run(&run)?, signal::Signal::SIGINT)
        }
        Subcommand::TerminateRun { run } => {
            actions::send_signal::send_signal(&runs.get_run(&run)?, signal::Signal::SIGTERM)
        }
        Subcommand::KillRun { run } => {
            actions::send_signal::send_signal(&runs.get_run(&run)?, signal::Signal::SIGKILL)
        }
    }
}

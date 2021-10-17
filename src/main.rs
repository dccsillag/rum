pub mod actions;
pub mod runs;
pub mod utils;

use anyhow::{Context, Result};
use clap::{crate_name, crate_version, Clap};
use nix::sys::signal;

use runs::Runs;

#[derive(Clap)]
#[clap(about = "A tool to manage running jobs.", name = crate_name!(), version = crate_version!())]
struct Args {
    #[clap(subcommand)]
    subcommand: Subcommand,
}

#[derive(Clap)]
enum Subcommand {
    /// List runs
    #[clap(name = "-list", short_flag = 'l', long_flag = "list")]
    List,

    /// Show information about a run
    #[clap(name = "-info", short_flag = 'i', long_flag = "info")]
    Info {
        /// Which run to show information on
        run: String,
    },

    /// View a run
    #[clap(name = "-view", short_flag = 'v', long_flag = "view")]
    View {
        /// Which run to view
        run: String,
    },

    /// Remove a run
    #[clap(name = "-remove", short_flag = 'r', long_flag = "remove")]
    Remove {
        /// Which runs to remove
        runs: Vec<String>,
    },

    /// Interrupt (SIGINT, i.e., Ctrl+C) a run
    #[clap(name = "-interrupt", short_flag = 'c', long_flag = "interrupt")]
    Interrupt {
        /// Which run to interrupt
        run: String,
    },

    /// Terminate (SIGTERM, i.e., kill <PID>) a run
    #[clap(name = "-terminate", short_flag = 't', long_flag = "terminate")]
    Terminate {
        /// Which run to terminate
        run: String,
    },

    /// Kill (SIGKILL, i.e., kill -9 <PID>) a run
    #[clap(name = "-kill", short_flag = 'K', long_flag = "kill")]
    Kill {
        /// Which run to kill
        run: String,
    },

    #[clap(external_subcommand)]
    Start(Vec<String>),
}

fn main() -> Result<()> {
    let args = Args::parse();

    let runs = Runs::new().with_context(|| "Could not acquire runs")?;

    match args.subcommand {
        Subcommand::Start(command) => {
            actions::start::start_run(&runs, command, /*TODO label*/ None)
        }
        Subcommand::List => actions::list::list_runs(&runs),
        Subcommand::Info { run } => actions::show_info::show_run_info(&runs.get_run(&run)?),
        Subcommand::View { run } => actions::open::open_run(&runs.get_run(&run)?),
        Subcommand::Remove { runs: to_remove } => actions::remove::remove_runs(&runs, &to_remove),
        Subcommand::Interrupt { run } => {
            actions::send_signal::send_signal(&runs.get_run(&run)?, signal::Signal::SIGINT)
        }
        Subcommand::Terminate { run } => {
            actions::send_signal::send_signal(&runs.get_run(&run)?, signal::Signal::SIGTERM)
        }
        Subcommand::Kill { run } => {
            actions::send_signal::send_signal(&runs.get_run(&run)?, signal::Signal::SIGKILL)
        }
    }
}

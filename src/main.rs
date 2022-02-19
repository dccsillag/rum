pub mod actions;
pub mod runs;
pub mod utils;

use anyhow::{Context, Result};
use clap::Parser;
use nix::sys::signal;

use runs::Runs;

#[derive(Parser)]
#[clap(
    about = "A tool to manage running jobs.",
    version,
    override_usage = "rum <COMMAND> [<ARG> [<ARG> [...]]]\n    rum <SUBCOMMAND>"
)]
#[clap(disable_help_subcommand = true)]
struct Args {
    #[clap(subcommand)]
    subcommand: Subcommand,
}

#[derive(Parser)]
enum Subcommand {
    /// List runs
    #[clap(name = "-list", short_flag = 'l', long_flag = "list", display_order = 0)]
    List,

    /// Show information about a run
    #[clap(name = "-info", short_flag = 'i', long_flag = "info", display_order = 1)]
    Info {
        /// Which run to show information on
        run: String,
    },

    /// View a run
    #[clap(name = "-view", short_flag = 'v', long_flag = "view", display_order = 2)]
    View {
        /// Which run to view
        run: String,
    },

    /// Remove a run
    #[clap(name = "-remove", short_flag = 'r', long_flag = "remove", display_order = 3)]
    Remove {
        /// Which runs to remove
        runs: Vec<String>,
    },

    /// Interrupt (SIGINT, i.e., Ctrl+C) a run
    #[clap(name = "-interrupt", short_flag = 'c', long_flag = "interrupt", display_order = 4)]
    Interrupt {
        /// Which run to interrupt
        run: String,
    },

    /// Terminate (SIGTERM, i.e., kill <PID>) a run
    #[clap(name = "-terminate", short_flag = 't', long_flag = "terminate", display_order = 5)]
    Terminate {
        /// Which run to terminate
        run: String,
    },

    /// Kill (SIGKILL, i.e., kill -9 <PID>) a run
    #[clap(name = "-kill", short_flag = 'K', long_flag = "kill", display_order = 6)]
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

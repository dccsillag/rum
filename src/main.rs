pub mod actions;
pub mod runs;
pub mod utils;

use anyhow::{Context, Result};
use clap::{crate_name, crate_version, App, AppSettings, Arg};
use nix::sys::signal;

use runs::Runs;

fn main() -> Result<()> {
    let matches = App::new(crate_name!())
        .about("A tool to manage running jobs.")
        .version(crate_version!())
        .setting(AppSettings::SubcommandRequiredElseHelp)
        .setting(AppSettings::AllowExternalSubcommands)
        .subcommand_placeholder("ACTION", "ACTIONS")
        .subcommand(
            App::new("-list")
                .short_flag('l')
                .long_flag("list")
                .about("List runs"),
        )
        .subcommand(
            App::new("-info")
                .short_flag('i')
                .long_flag("info")
                .about("Show information about a run")
                .arg(Arg::new("run").about("Which run to show information on")),
        )
        .subcommand(
            App::new("-view")
                .short_flag('v')
                .long_flag("view")
                .about("View a run")
                .arg(Arg::new("run").about("Which run to view")),
        )
        .subcommand(
            App::new("-remove")
                .short_flag('r')
                .long_flag("remove")
                .about("Remove a run")
                .arg(Arg::new("run").about("Which run to remove")),
        )
        .subcommand(
            App::new("-interrupt")
                .short_flag('c')
                .long_flag("interrupt")
                .about("Interrupt (SIGINT, Ctrl+C) a run")
                .arg(Arg::new("run").about("Which run to interrupt")),
        )
        .subcommand(
            App::new("-terminate")
                .short_flag('t')
                .long_flag("terminate")
                .about("Terminate (SIGTERM) a run")
                .arg(Arg::new("run").about("Which run to terminate")),
        )
        .subcommand(
            App::new("-kill")
                .short_flag('K')
                .long_flag("kill9")
                .about("Kill (SIGKILL, kill -9) a run")
                .arg(Arg::new("run").about("Which run to kill")),
        )
        .get_matches();

    let runs = Runs::new().with_context(|| "Could not acquire runs")?;

    match matches.subcommand() {
        Some(("-list", _)) => actions::list::list_runs(&runs),
        Some(("-info", submatches)) => {
            actions::show_info::show_run_info(&runs.get_run(&submatches.value_of_t_or_exit("run"))?)
        }
        Some(("-view", submatches)) => {
            actions::open::open_run(&runs.get_run(&submatches.value_of_t_or_exit("run"))?)
        }
        Some(("-remove", submatches)) => {
            actions::remove::remove_run(&runs, runs.get_run(&submatches.value_of_t_or_exit("run"))?)
        }
        Some(("-interrupt", submatches)) => actions::send_signal::send_signal(
            &runs.get_run(&submatches.value_of_t_or_exit("run"))?,
            signal::Signal::SIGINT,
        ),
        Some(("-terminate", submatches)) => actions::send_signal::send_signal(
            &runs.get_run(&submatches.value_of_t_or_exit("run"))?,
            signal::Signal::SIGTERM,
        ),
        Some(("-kill", submatches)) => actions::send_signal::send_signal(
            &runs.get_run(&submatches.value_of_t_or_exit("run"))?,
            signal::Signal::SIGKILL,
        ),
        Some((argv0, submatches)) => {
            let command = match submatches.values_of("") {
                Some(s) => std::iter::once(argv0)
                    .chain(s)
                    .map(|s| s.to_string())
                    .collect(),
                None => std::iter::once(argv0).map(|s| s.to_string()).collect(),
            };
            actions::start::start_run(&runs, command, /*TODO label*/ None)
        }
        _ => unreachable!(),
    }
}

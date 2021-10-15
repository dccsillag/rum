use anyhow::Result;
use chrono::{DateTime, Utc};
use tabled::{Table, Tabled};

use crate::runs::{RunData, RunDataState, RunId, Runs};

pub fn list_runs(runs: &Runs) -> Result<()> {
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

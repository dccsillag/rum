use std::{
    collections::HashMap,
    convert::TryInto,
    os::unix::prelude::{AsRawFd, FromRawFd},
    path::PathBuf,
};

use anyhow::{Context, Error, Result};
use chrono::{DateTime, Utc};
use fork::{daemon, Fork};
use nix::unistd::Pid;
use serde::{Deserialize, Serialize};

use uuid::Uuid;

pub type RunId = String;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunData {
    pub label: Option<String>,
    pub command: Vec<String>,
    pub start_datetime: DateTime<Utc>,

    pub state: RunDataState,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RunDataState {
    Running {
        #[serde(with = "serde_nix_pid")]
        pid: Pid,
    },
    Done {
        end_datetime: DateTime<Utc>,
        exit_code: i32,
    },
}

#[derive(Debug, Clone)]
pub struct Run {
    pub id: RunId,
    pub run_directory: PathBuf,
}

pub struct Runs {
    run_directory: PathBuf,
}

fn ensure_dir_exists(path: PathBuf) -> Result<PathBuf> {
    std::fs::create_dir_all(&path)?;
    Ok(path)
}

impl Runs {
    pub fn new() -> Result<Self> {
        let project_dirs = directories::ProjectDirs::from("com.github", "dccsillag", "rum")
            .ok_or(Error::msg("Couldn't get project directories"))?;

        let data_dir = project_dirs.data_local_dir().to_path_buf();

        Ok(Self {
            run_directory: ensure_dir_exists(data_dir.join("runs"))?,
        })
    }

    pub fn get_run(&self, id: &RunId) -> Result<Run> {
        Ok(Run {
            id: id.clone(),
            run_directory: self.run_directory.join(id),
        })
    }

    pub fn get_all(&self) -> Result<HashMap<RunId, Run>> {
        let mut out = HashMap::new();

        for run_file in self
            .run_directory
            .read_dir()
            .with_context(|| "Could not open data directory")?
            .filter_map(|x| x.ok())
            .map(|x| x.path())
        {
            // TODO: errors in this block should be turned into warnings, not fatal errors.
            let run_id = run_file.file_name().unwrap().to_str().unwrap().to_string();
            let run = self.get_run(&run_id)?;
            out.insert(run_id, run);
        }

        Ok(out)
    }

    pub fn new_run(&self) -> Result<Run> {
        let id = Uuid::new_v4().to_string();
        Ok(Run {
            run_directory: ensure_dir_exists(self.run_directory.join(&id))?,
            id,
        })
    }

    pub fn remove_run(&self, run: Run) -> Result<()> {
        std::fs::remove_dir_all(run.run_directory)?;
        Ok(())
    }
}

impl Run {
    fn get_data_file(&self) -> PathBuf {
        self.run_directory.join("data.json")
    }

    pub fn get_output_file(&self) -> PathBuf {
        self.run_directory.join("output.log")
    }

    pub fn get_data(&self) -> Result<RunData> {
        let data_file = self.get_data_file();
        serde_json::from_reader(
            std::fs::File::open(&data_file)
                .with_context(|| format!("Could not open {:?}", &data_file))?,
        )
        .with_context(|| format!("Could not parse JSON in {:?}", &data_file))
    }

    fn set_data(&self, run: &RunData) -> Result<()> {
        serde_json::to_writer(std::fs::File::create(self.get_data_file())?, run)?;
        Ok(())
    }

    fn update_data<F>(&self, f: F) -> Result<()>
    where
        F: Fn(RunData) -> Result<RunData>,
    {
        let data = self.get_data()?;
        let data = f(data)?;
        self.set_data(&data)?;

        Ok(())
    }

    pub fn start(&self, command: Vec<String>, label: Option<String>) -> Result<()> {
        assert!(!command.is_empty());

        if let Fork::Child = daemon(true, false)
            .map_err(|e| Error::msg(format!("Failed to fork: error code {}", e)))?
        {
            let output_file_path = self.get_output_file();
            let output_file = std::fs::File::create(output_file_path)?;
            let output_file_raw = output_file.as_raw_fd();

            let mut process: std::process::Child =
                std::process::Command::new(command.first().unwrap())
                    .args(&command[1..])
                    .stdout(unsafe { std::process::Stdio::from_raw_fd(output_file_raw) })
                    .stderr(unsafe { std::process::Stdio::from_raw_fd(output_file_raw) })
                    .stdin(std::process::Stdio::null())
                    .spawn()?;

            self.set_data(&RunData {
                command: command,
                label: label,
                start_datetime: Utc::now(),

                state: RunDataState::Running {
                    pid: Pid::from_raw(process.id().try_into().unwrap()),
                },
            })?;

            let exit_status = process.wait()?;

            self.update_data(|run_data| {
                Ok(RunData {
                    state: RunDataState::Done {
                        exit_code: exit_status.code().unwrap_or(-1),
                        end_datetime: Utc::now(),
                    },
                    ..run_data
                })
            })?;
        }

        Ok(())
    }
}

mod serde_nix_pid {
    use nix::unistd::Pid;
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S>(pid: &Pid, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_i32(pid.as_raw())
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Pid, D::Error>
    where
        D: Deserializer<'de>,
    {
        Ok(Pid::from_raw(i32::deserialize(deserializer)?))
    }
}

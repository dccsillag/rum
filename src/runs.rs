use std::{collections::HashMap, path::PathBuf, process::ExitStatus};

use anyhow::{Context, Error, Result};
use chrono::{DateTime, Utc};
use nix::unistd::Pid;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub type RunId = String;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunData {
    pub label: Option<String>,
    pub command: Vec<String>,
    pub start_datetime: DateTime<Utc>,

    pub done_data: Option<RunDoneData>,

    #[serde(with = "serde_nix_pid")]
    pub pid: Pid,

    pub output_file: PathBuf,
}

#[derive(Debug, Clone)]
pub struct Run {
    pub id: RunId,
    pub run_directory: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunDoneData {
    pub end_datetime: DateTime<Utc>,
    #[serde(with = "serde_exitstatus")]
    pub exit_code: ExitStatus,
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

    pub fn set_data(&self, run: &RunData) -> Result<()> {
        serde_json::to_writer(std::fs::File::create(self.get_data_file())?, run)?;
        Ok(())
    }

    pub fn update_data<F>(&self, f: F) -> Result<()>
    where
        F: Fn(RunData) -> Result<RunData>,
    {
        let data = self.get_data()?;
        let data = f(data)?;
        self.set_data(&data)?;

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

mod serde_exitstatus {
    use serde::{Deserialize, Deserializer, Serializer};
    use std::{os::unix::prelude::ExitStatusExt, process::ExitStatus};

    pub fn serialize<S>(exit_status: &ExitStatus, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_i32(exit_status.code().unwrap_or(-1).into())
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<ExitStatus, D::Error>
    where
        D: Deserializer<'de>,
    {
        Ok(ExitStatus::from_raw(i32::deserialize(deserializer)?))
    }
}

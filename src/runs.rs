use std::{
    os::unix::prelude::{AsRawFd, FromRawFd},
    path::PathBuf,
    process::Child,
};

use anyhow::{Context, Error, Result};
use chrono::{DateTime, Utc};
use fork::{close_fd, fork, Fork};
use nix::unistd::{getpgid, setpgid, Pid};
use serde::{Deserialize, Serialize};
use thiserror::Error;

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
        pgid: Pid,
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
            .ok_or_else(|| Error::msg("Couldn't get project directories"))?;

        let data_dir = project_dirs.data_local_dir().to_path_buf();

        Ok(Self {
            run_directory: ensure_dir_exists(data_dir.join("runs"))?,
        })
    }

    fn run_paths_iter(&self) -> Result<impl Iterator<Item = (RunId, PathBuf)>> {
        Ok(self
            .run_directory
            .read_dir()
            .with_context(|| "Could not open data directory")?
            .filter_map(|x| x.ok())
            .map(|x| (x.file_name().to_str().unwrap().to_string(), x.path())))
    }

    pub fn get_run(&self, id: &RunId) -> Result<Run> {
        let matching_ids = self
            .run_paths_iter()?
            .filter(|(run_id, _)| run_id.starts_with(id))
            .map(|(run_id, run_path)| Run {
                id: run_id,
                run_directory: run_path,
            })
            .collect::<Vec<_>>();

        match &matching_ids[..] {
            [] => Err(Error::msg(format!("No matching ID for query '{}'", id))),
            [run] => Ok(run.clone()),
            _ => Err(Error::msg(format!(
                "Multiple matching IDs for query '{}'",
                id
            ))),
        }
    }

    pub fn get_all(&self) -> Result<Vec<Run>> {
        Ok(self
            .run_paths_iter()?
            .map(|(run_id, run_path)| Run {
                id: run_id,
                run_directory: run_path,
            })
            .collect())
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

#[derive(Serialize, Deserialize, Error, Debug, Clone)]
enum ForkedError {
    #[error("couldn't create output file: {message}")]
    CouldntCreateOutputFile { message: String },
    #[error("couldn't set process group: {0}")]
    CouldntSetProcessGroup(String),
    #[error("couldn't save run data: {message}")]
    CouldntSetData { message: String },
    #[error("failed to spawn process: {command}: {message}")]
    FailedToSpawn { command: String, message: String },
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

    fn spawn_process(
        &self,
        command: Vec<String>,
        label: Option<String>,
    ) -> std::result::Result<Child, ForkedError> {
        let output_file_path = self.get_output_file();
        let output_file = std::fs::File::create(output_file_path).map_err(|e| {
            ForkedError::CouldntCreateOutputFile {
                message: e.to_string(),
            }
        })?;
        let output_file_raw = output_file.as_raw_fd();

        let gid = getpgid(None).unwrap(); // this will always succeed, since we are getting the pgid of the current process

        let process = std::process::Command::new(command.first().unwrap())
            .args(&command[1..])
            .stdout(unsafe { std::process::Stdio::from_raw_fd(output_file_raw) })
            .stderr(unsafe { std::process::Stdio::from_raw_fd(output_file_raw) })
            .stdin(std::process::Stdio::null())
            .spawn()
            .map_err(|e| ForkedError::FailedToSpawn {
                command: command.first().unwrap().to_string(),
                message: e.to_string(),
            })?;

        setpgid(Pid::from_raw(0), Pid::from_raw(0))
            .map_err(|e| ForkedError::CouldntSetProcessGroup(e.desc().to_string()))?;

        self.set_data(&RunData {
            command,
            label,
            start_datetime: Utc::now(),

            state: RunDataState::Running { pgid: gid },
        })
        .map_err(|e| ForkedError::CouldntSetData {
            message: e.to_string(),
        })?;

        Ok(process)
    }

    pub fn start(&self, command: Vec<String>, label: Option<String>) -> Result<()> {
        assert!(!command.is_empty());

        let (sender, receiver) = ipc_channel::ipc::channel::<Message>()?;

        #[derive(Serialize, Deserialize, Debug)]
        enum Message {
            Started,
            Err(ForkedError),
        }

        setpgid(Pid::from_raw(0), Pid::from_raw(0))
            .map_err(|e| Error::msg(format!("couldnt set run pgid: {}", e.desc())))?;

        match fork().map_err(|e| Error::msg(format!("Failed to fork: error code {}", e)))? {
            Fork::Child => {
                close_fd().expect("couldn't close file descriptors in forked child process");
                match self.spawn_process(command, label) {
                    Ok(mut process) => {
                        sender.send(Message::Started)?;

                        match process.wait() {
                            Ok(exit_status) => self.update_data(|run_data| {
                                Ok(RunData {
                                    state: RunDataState::Done {
                                        exit_code: exit_status.code().unwrap_or(-1),
                                        end_datetime: Utc::now(),
                                    },
                                    ..run_data
                                })
                            }),
                            Err(_) => self.update_data(|run_data| {
                                Ok(RunData {
                                    state: RunDataState::Done {
                                        exit_code: -2,
                                        end_datetime: Utc::now(),
                                    },
                                    ..run_data
                                })
                            }),
                        }
                    }
                    Err(e) => {
                        sender.send(Message::Err(e.clone()))?;
                        std::fs::remove_dir_all(&self.run_directory)?;
                        Err(Error::from(e))
                    }
                }
            }
            Fork::Parent(_) => {
                let message = receiver
                    .recv()
                    .map_err(|_| Error::msg("Failed to communicate with forked process"))?;
                match message {
                    Message::Err(e) => Err(Error::from(e)),
                    Message::Started => {
                        println!("Started run {}", self.id);
                        Ok(())
                    }
                }
            }
        }
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

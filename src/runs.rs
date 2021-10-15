use std::{
    collections::HashMap,
    convert::TryInto,
    io::{Read, Seek, Write},
    os::unix::prelude::{AsRawFd, FromRawFd},
    path::PathBuf,
    sync::mpsc::channel,
};

use anyhow::{Context, Error, Result};
use chrono::{DateTime, Utc};
use nix::{sys::signal, unistd::Pid};
use notify::Watcher;
use serde::{Deserialize, Serialize};
use termion::{event::Key, input::TermRead, raw::IntoRawMode};
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
        let output_file_path = self.get_output_file();
        let output_file = std::fs::File::create(output_file_path)?;
        let output_file_raw = output_file.as_raw_fd();

        let mut process: std::process::Child = std::process::Command::new(command.first().unwrap())
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

        Ok(())
    }

    pub fn open(&self, interactable: bool) -> Result<()> {
        let output_file_path = self.get_output_file();

        let (tx, rx) = channel();
        let mut watcher: notify::RecommendedWatcher =
            Watcher::new(tx, std::time::Duration::from_millis(50))?;
        watcher.watch(&output_file_path, notify::RecursiveMode::NonRecursive)?;

        let mut screen =
            termion::screen::AlternateScreen::from(std::io::stdout()).into_raw_mode()?;
        let mut input = std::io::stdin().keys();

        let mut file = std::fs::File::open(&output_file_path)?;
        let mut buffer = String::new();
        let mut seek_location = 0; // TODO what happens when the file is really big?

        let mut update_output = || -> Result<()> {
            file.seek(std::io::SeekFrom::Start(seek_location))?;
            let how_much_was_read = file.read_to_string(&mut buffer)?;
            buffer = buffer.replace('\n', "\r\n");
            seek_location += how_much_was_read as u64;
            write!(screen, "{}", buffer)?;
            buffer.clear();

            // FIXME what if the output is already styled?
            write!(
                screen,
                "{}{}{}{}",
                termion::cursor::Save,
                termion::cursor::Goto(1, 1),
                termion::clear::CurrentLine,
                termion::style::Faint,
            )?;
            write!(
                screen,
                "You are currently viewing a run. Press Ctrl+C to exit."
            )?;
            write!(
                screen,
                "{}",
                termion::cursor::Goto(termion::terminal_size()?.0 - (self.id.len() as u16) + 1, 1),
            )?;
            write!(screen, "{}", self.id)?;
            write!(
                screen,
                "{}{}",
                termion::style::NoFaint,
                termion::cursor::Restore
            )?;

            screen.flush()?;

            Ok(())
        };

        update_output()?;
        'mainloop: loop {
            match rx.try_recv() {
                Ok(notify::DebouncedEvent::Write(_)) => update_output()?,
                Ok(_) => (),
                Err(_) => std::thread::sleep(std::time::Duration::from_millis(10)),
            }

            while let Some(key) = input.next() {
                match key? {
                    Key::Ctrl('c') => break 'mainloop,
                    _ => (),
                }
            }
        }

        Ok(())
    }

    pub fn send_signal(&self, signal: signal::Signal) -> Result<()> {
        match self.get_data()?.state {
            RunDataState::Running { pid } => {
                signal::kill(pid, signal).with_context(|| "Couldn't send signal to run's process")
            }
            RunDataState::Done { .. } => Err(Error::msg(format!("Still running: {}", self.id))),
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

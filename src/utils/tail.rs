use std::{io::{Read, Seek}, path::Path, sync::mpsc::{TryRecvError, channel}};

use anyhow::{Result, Error};
use notify::Watcher;

pub fn follow_tail<F, G>(path: &Path, mut on_new_text: F, mut on_iter: G) -> Result<()>
where
    F: FnMut(&str) -> Result<()>,
    G: FnMut() -> Result<bool>,
{
    let (tx, rx) = channel();
    let mut watcher: notify::RecommendedWatcher =
        Watcher::new(tx, std::time::Duration::from_millis(50))?;
    watcher.watch(&path, notify::RecursiveMode::NonRecursive)?;

    let mut file = std::fs::File::open(path)?;
    let mut buffer = String::new();
    let mut seek_location = 0; // TODO what happens when the file is really big?

    let mut update = || -> Result<()> {
        buffer.clear();
        file.seek(std::io::SeekFrom::Start(seek_location))?;
        let how_much_was_read = file.read_to_string(&mut buffer)?;
        seek_location += how_much_was_read as u64;
        on_new_text(&buffer)?;
        Ok(())
    };

    update()?;
    loop {
        match rx.try_recv() {
            Ok(notify::DebouncedEvent::Write(_)) => {
                update()?;
            },
            Ok(_) => (),
            Err(TryRecvError::Empty) => std::thread::sleep(std::time::Duration::from_millis(10)),
            Err(TryRecvError::Disconnected) => {
                return Err(Error::msg("Output file watcher disconnected"));
            },
        }

        if on_iter()? {
            break;
        }
    }

    Ok(())
}

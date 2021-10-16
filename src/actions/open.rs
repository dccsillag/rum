use std::io::Write;

use anyhow::Result;
use termion::{event::Key, input::TermRead, raw::IntoRawMode};

use crate::runs::Run;
use crate::utils::tail;

pub fn open_run(run: &Run) -> Result<()> {
    let output_file_path = run.get_output_file();

    let mut screen = termion::screen::AlternateScreen::from(std::io::stdout()).into_raw_mode()?;
    let mut input = termion::async_stdin().keys();

    write!(
        screen,
        "{}{}",
        termion::clear::All,
        termion::cursor::Goto(1, 2)
    )?;

    tail::follow_tail(
        &output_file_path,
        |new_text: &str| -> Result<()> {
            let new_text = new_text.replace('\n', "\r\n");
            write!(screen, "{}", new_text)?;

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
                termion::cursor::Goto(termion::terminal_size()?.0 - (run.id.len() as u16) + 1, 1),
            )?;
            write!(screen, "{}", run.id)?;
            write!(
                screen,
                "{}{}",
                termion::style::NoFaint,
                termion::cursor::Restore
            )?;

            screen.flush()?;

            Ok(())
        },
        || {
            while let Some(key) = input.next() {
                match key? {
                    Key::Ctrl('c') => return Ok(true),
                    _ => (),
                }
            }
            Ok(false)
        },
    )
}

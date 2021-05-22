extern crate termion;

use std::io::{self, Write};
use termion::console;
use termion::cursor;
use termion::event::*;
use termion::input::MouseTerminal;
use termion::raw::IntoRawMode;

fn main() {
    let mut console = console().unwrap();
    let mut stdout = MouseTerminal::from(io::stdout().into_raw_mode().unwrap());

    writeln!(
        stdout,
        "{}{}q to exit. Type stuff, use alt, click around...",
        termion::clear::All,
        termion::cursor::Goto(1, 1)
    )
    .unwrap();

    loop {
        let c = console.get_event();
        let evt = c.unwrap();
        match evt {
            Event::Key(Key::Char('q')) => break,
            Event::Mouse(me) => match me {
                MouseEvent::Press(_, a, b) | MouseEvent::Release(a, b) | MouseEvent::Hold(a, b) => {
                    write!(stdout, "{}", cursor::Goto(a, b)).unwrap();
                    let (x, y) = console.cursor_pos().unwrap();
                    write!(
                        stdout,
                        "{}{}Cursor is at: ({},{}){}",
                        cursor::Goto(5, 5),
                        termion::clear::UntilNewline,
                        x,
                        y,
                        cursor::Goto(a, b)
                    )
                    .unwrap();
                }
            },
            _ => {}
        }

        stdout.flush().unwrap();
    }
}

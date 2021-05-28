extern crate sl_console;

use std::io::Write;
use sl_console::console::*;
use sl_console::cursor::{self, CursorPos};
use sl_console::event::*;
use sl_console::input::MouseTerminal;

fn main() {
    let console = console().unwrap();
    let mut console = MouseTerminal::from(console);

    writeln!(
        console,
        "{}{}q to exit. Type stuff, use alt, click around...",
        sl_console::clear::All,
        sl_console::cursor::Goto(1, 1)
    )
    .unwrap();

    loop {
        let c = console.get_event();
        let evt = c.unwrap();
        match evt {
            Event::Key(Key::Char('q')) => break,
            Event::Mouse(me) => match me {
                MouseEvent::Press(_, a, b) | MouseEvent::Release(a, b) | MouseEvent::Hold(a, b) => {
                    write!(console, "{}", cursor::Goto(a, b)).unwrap();
                    let (x, y) = console.cursor_pos().unwrap();
                    write!(
                        console,
                        "{}{}Cursor is at: ({},{}){}",
                        cursor::Goto(5, 5),
                        sl_console::clear::UntilNewline,
                        x,
                        y,
                        cursor::Goto(a, b)
                    )
                    .unwrap();
                }
            },
            _ => {}
        }

        console.flush().unwrap();
    }
}

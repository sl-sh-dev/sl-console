use sl_console::cursor::{self, CursorPos};
use sl_console::event::*;
use sl_console::input::*;
use sl_console::*;
use std::io::Write;

fn main() {
    con_init().unwrap();
    let mut console = conout();
    let _raw = console.raw_mode_guard().unwrap();
    let mut console = MouseTerminal::from(console);

    writeln!(
        console,
        "{}{}q to exit. Type stuff, use alt, click around...",
        sl_console::clear::All,
        sl_console::cursor::Goto(1, 1)
    )
    .unwrap();

    let mut conin = conin();
    loop {
        let c = conin.get_event();
        let evt = c.unwrap();
        match evt {
            Event::Key(Key::Char('q')) => break,
            Event::Mouse(me) => match me {
                MouseEvent::Press(_, a, b) | MouseEvent::Release(a, b) | MouseEvent::Hold(a, b) => {
                    write!(console, "{}", cursor::Goto(a, b)).unwrap();
                    let (x, y) = conin.cursor_pos().unwrap();
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

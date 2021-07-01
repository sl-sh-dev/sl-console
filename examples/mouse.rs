use sl_console::cursor::{self, cursor_pos};
use sl_console::event::*;
use sl_console::input::*;
use sl_console::*;
use std::io::Write;

fn main() {
    con_init().unwrap();

    let console = conout().into_raw_mode().unwrap();
    let mut console = MouseTerminal::from(console);

    writeln!(
        console,
        "{}{}q to exit. Type stuff, use alt, click around...",
        sl_console::clear::All,
        sl_console::cursor::Goto(1, 1)
    )
    .unwrap();

    for c in conin().lock().events() {
        let evt = c.unwrap();
        match evt {
            Event::Key(key) => match key.code {
                KeyCode::Char('q') => break,
                _ => {}
            },
            Event::Mouse(me) => match me {
                MouseEvent::Press(_, a, b) | MouseEvent::Release(a, b) | MouseEvent::Hold(a, b) => {
                    write!(console, "{}", cursor::Goto(a, b)).unwrap();
                    let (x, y) = cursor_pos().unwrap();
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

extern crate sl_console;

use sl_console::event::Key;
use sl_console::input::TermRead;
use sl_console::{conin, conout, ConsoleWrite};
use std::io::Write;

fn rainbow<W: Write>(stdout: &mut W, blue: u8) {
    write!(
        stdout,
        "{}{}",
        sl_console::cursor::Goto(1, 1),
        sl_console::clear::All
    )
    .unwrap();

    for red in 0..32 {
        let red = red * 8;
        for green in 0..64 {
            let green = green * 4;
            write!(
                stdout,
                "{} ",
                sl_console::color::Bg(sl_console::color::Rgb(red, green, blue))
            )
            .unwrap();
        }
        write!(stdout, "\n\r").unwrap();
    }

    writeln!(stdout, "{}b = {}", sl_console::style::Reset, blue).unwrap();
}

fn main() {
    let stdin = conin().unwrap();
    let mut stdout = conout().unwrap();
    let _raw = stdout.raw_mode_guard().unwrap();

    writeln!(
        stdout,
        "{}{}{}Use the up/down arrow keys to change the blue in the rainbow.",
        sl_console::clear::All,
        sl_console::cursor::Goto(1, 1),
        sl_console::cursor::Hide
    )
    .unwrap();

    let mut blue = 172u8;

    for c in stdin.keys() {
        match c.unwrap() {
            Key::Up => {
                blue = blue.saturating_add(4);
                rainbow(&mut stdout, blue);
            }
            Key::Down => {
                blue = blue.saturating_sub(4);
                rainbow(&mut stdout, blue);
            }
            Key::Char('q') => break,
            _ => {}
        }
        stdout.flush().unwrap();
    }

    write!(stdout, "{}", sl_console::cursor::Show).unwrap();
}

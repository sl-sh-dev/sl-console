extern crate sl_console;

use sl_console::event::Key;
use sl_console::input::TermRead;
use sl_console::{conin, conout, ConsoleWrite};
use std::io::Write;

fn main() {
    let stdin = conin().unwrap();
    let mut stdout = conout().unwrap();
    let _raw = stdout.raw_mode_guard().unwrap();

    write!(
        stdout,
        "{}{}q to exit. Type stuff, use alt, and so on.{}",
        sl_console::clear::All,
        sl_console::cursor::Goto(1, 1),
        sl_console::cursor::Hide
    )
    .unwrap();
    stdout.flush().unwrap();

    for c in stdin.keys() {
        write!(
            stdout,
            "{}{}",
            sl_console::cursor::Goto(1, 1),
            sl_console::clear::CurrentLine
        )
        .unwrap();

        match c.unwrap() {
            Key::Char('q') => break,
            Key::Char(c) => println!("{}", c),
            Key::Alt(c) => println!("^{}", c),
            Key::Ctrl(c) => println!("*{}", c),
            Key::Esc => println!("ESC"),
            Key::Left => println!("←"),
            Key::Right => println!("→"),
            Key::Up => println!("↑"),
            Key::Down => println!("↓"),
            Key::Backspace => println!("×"),
            _ => {}
        }
        stdout.flush().unwrap();
    }

    write!(stdout, "{}", sl_console::cursor::Show).unwrap();
}

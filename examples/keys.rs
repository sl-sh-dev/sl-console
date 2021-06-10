extern crate sl_console;

use sl_console::event::Key;
use sl_console::input::TermRead;
use sl_console::*;
use std::io::Write;

fn main() {
    con_init().unwrap();
    let conin = conin();
    let mut conout = conout();
    let _raw = conout.raw_mode_guard().unwrap();

    write!(
        conout,
        "{}{}q to exit. Type stuff, use alt, and so on.{}",
        sl_console::clear::All,
        sl_console::cursor::Goto(1, 1),
        sl_console::cursor::Hide
    )
    .unwrap();
    conout.flush().unwrap();

    for c in conin.keys() {
        write!(
            conout,
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
        conout.flush().unwrap();
    }

    write!(conout, "{}", sl_console::cursor::Show).unwrap();
}

use sl_console::event::KeyCode;
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

        let key = c.unwrap();
        match key.code {
            KeyCode::Char('q') => break,
            KeyCode::Char(c) => println!("{}", c),
            KeyCode::Alt(c) => println!("^{}", c),
            KeyCode::Ctrl(c) => println!("*{}", c),
            KeyCode::Esc => println!("ESC"),
            KeyCode::Left => println!("←"),
            KeyCode::Right => println!("→"),
            KeyCode::Up => println!("↑"),
            KeyCode::Down => println!("↓"),
            KeyCode::Backspace => println!("×"),
            _ => {}
        }
        conout.flush().unwrap();
    }

    write!(conout, "{}", sl_console::cursor::Show).unwrap();
}

use sl_console::color;
use sl_console::*;
use std::io::{Read, Write};

fn main() {
    // Initialize 'em all.
    con_init().unwrap();
    let mut conout = conout();
    let _raw = conout.raw_mode_guard().unwrap();
    let conin = conin();

    write!(
        conout,
        "{}{}{}yo, 'q' will exit.{}{}",
        sl_console::clear::All,
        sl_console::cursor::Goto(5, 5),
        sl_console::style::Bold,
        sl_console::style::Reset,
        sl_console::cursor::Goto(20, 10)
    )
    .unwrap();
    conout.flush().unwrap();

    let mut bytes = conin.bytes();
    loop {
        let b = bytes.next().unwrap().unwrap();

        match b {
            // Quit
            b'q' => return,
            // Clear the screen
            b'c' => write!(conout, "{}", sl_console::clear::All),
            // Set red color
            b'r' => write!(conout, "{}", color::Fg(color::Rgb(255, 0, 0))),
            // Write it to conout.
            a => write!(conout, "{}", a),
        }
        .unwrap();

        conout.flush().unwrap();
    }
}

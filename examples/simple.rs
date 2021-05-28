extern crate sl_console;

use std::io::{stdin, stdout, Read, Write};
use sl_console::color;
use sl_console::raw::IntoRawMode;

fn main() {
    // Initialize 'em all.
    let stdout = stdout();
    let mut stdout = stdout.lock().into_raw_mode().unwrap();
    let stdin = stdin();
    let stdin = stdin.lock();

    write!(
        stdout,
        "{}{}{}yo, 'q' will exit.{}{}",
        sl_console::clear::All,
        sl_console::cursor::Goto(5, 5),
        sl_console::style::Bold,
        sl_console::style::Reset,
        sl_console::cursor::Goto(20, 10)
    )
    .unwrap();
    stdout.flush().unwrap();

    let mut bytes = stdin.bytes();
    loop {
        let b = bytes.next().unwrap().unwrap();

        match b {
            // Quit
            b'q' => return,
            // Clear the screen
            b'c' => write!(stdout, "{}", sl_console::clear::All),
            // Set red color
            b'r' => write!(stdout, "{}", color::Fg(color::Rgb(5, 0, 0))),
            // Write it to stdout.
            a => write!(stdout, "{}", a),
        }
        .unwrap();

        stdout.flush().unwrap();
    }
}

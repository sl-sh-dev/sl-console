extern crate sl_console;

use sl_console::input::TermRead;
use sl_console::{conin, conout};
use std::io::Write;

fn main() {
    let mut stdout = conout().unwrap();
    let mut stdin = conin().unwrap();

    stdout.write_all(b"password: ").unwrap();
    stdout.flush().unwrap();

    let pass = stdin.read_line();

    if let Ok(Some(pass)) = pass {
        stdout.write_all(pass.as_bytes()).unwrap();
        stdout.write_all(b"\n").unwrap();
    } else {
        stdout.write_all(b"Error\n").unwrap();
    }
}

extern crate sl_console;

use std::io::{stdin, stdout, Write};
use sl_console::input::TermRead;

fn main() {
    sl_console::set_virtual_terminal().unwrap();
    let stdout = stdout();
    let mut stdout = stdout.lock();
    let stdin = stdin();
    let mut stdin = stdin.lock();

    stdout.write_all(b"password: ").unwrap();
    stdout.flush().unwrap();

    let pass = stdin.read_passwd(&mut stdout);

    if let Ok(Some(pass)) = pass {
        stdout.write_all(pass.as_bytes()).unwrap();
        stdout.write_all(b"\n").unwrap();
    } else {
        stdout.write_all(b"Error\n").unwrap();
    }
}

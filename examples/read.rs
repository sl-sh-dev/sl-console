extern crate sl_console;

use sl_console::input::TermRead;
use sl_console::*;
use std::io::Write;

fn main() {
    con_init().unwrap();
    let mut conout = conout();
    let _raw = conout.raw_mode_guard().unwrap();
    let mut conin = conin();

    conout.write_all(b"password: ").unwrap();
    conout.flush().unwrap();

    let pass = conin.read_line();

    if let Ok(Some(pass)) = pass {
        conout.write_all(pass.as_bytes()).unwrap();
        conout.write_all(b"\n").unwrap();
    } else {
        conout.write_all(b"Error\n").unwrap();
    }
}

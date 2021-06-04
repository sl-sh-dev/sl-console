extern crate sl_console;

use sl_console::event::*;
use sl_console::*;
use std::io::{self, Write};
use std::thread;
use std::time::Duration;

fn main() {
    coninit().unwrap();
    let mut conin = conin();
    conin.set_blocking(false); // Console to async read.
    let mut conout = conout();
    let _raw = conout.raw_mode_guard().unwrap();

    write!(
        conout,
        "{}{}",
        sl_console::clear::All,
        sl_console::cursor::Goto(1, 1)
    )
    .unwrap();

    loop {
        let evt = conin.get_event();

        write!(conout, "{}", sl_console::clear::CurrentLine).unwrap();
        write!(conout, "\r{:?}    <- This demonstrates the async read input char. Between each update a 100 ms. is waited, simply to demonstrate the async fashion. \n\r", evt).unwrap();
        match evt {
            Ok(evt) => match evt {
                Event::Key(Key::Char('q')) => break,
                Event::Key(Key::Char('b')) => conin.set_blocking(!conin.is_blocking()),
                _ => {}
            },
            Err(err) if err.kind() == io::ErrorKind::WouldBlock => {
                // Just means no data was ready (non-blocking)
            }
            Err(err) => panic!("Got BAD error {}", err),
        }

        conout.flush().unwrap();

        thread::sleep(Duration::from_millis(50));
        conout.write_all(b"# ").unwrap();
        conout.flush().unwrap();
        thread::sleep(Duration::from_millis(50));
        conout.write_all(b"\r #").unwrap();
        write!(conout, "{}", sl_console::cursor::Goto(1, 1)).unwrap();
        conout.flush().unwrap();
    }
}

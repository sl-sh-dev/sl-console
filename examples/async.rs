extern crate sl_console;

use std::io::{self, Write};
use std::thread;
use std::time::Duration;
use sl_console::console::*;
use sl_console::event::*;

fn main() {
    let mut console = console().unwrap();
    console.set_blocking(false); // Console to async read.

    write!(
        console,
        "{}{}",
        sl_console::clear::All,
        sl_console::cursor::Goto(1, 1)
    )
    .unwrap();

    loop {
        let evt = console.get_event();

        write!(console, "{}", sl_console::clear::CurrentLine).unwrap();
        write!(console, "\r{:?}    <- This demonstrates the async read input char. Between each update a 100 ms. is waited, simply to demonstrate the async fashion. \n\r", evt).unwrap();
        match evt {
            Ok(evt) => match evt {
                Event::Key(Key::Char('q')) => break,
                Event::Key(Key::Char('b')) => console.set_blocking(!console.is_blocking()),
                _ => {}
            },
            Err(err) if err.kind() == io::ErrorKind::WouldBlock => {
                // Just means no data was ready (non-blocking)
            }
            Err(err) => panic!("Got BAD error {}", err),
        }

        console.flush().unwrap();

        thread::sleep(Duration::from_millis(50));
        console.write_all(b"# ").unwrap();
        console.flush().unwrap();
        thread::sleep(Duration::from_millis(50));
        console.write_all(b"\r #").unwrap();
        write!(console, "{}", sl_console::cursor::Goto(1, 1)).unwrap();
        console.flush().unwrap();
    }
}

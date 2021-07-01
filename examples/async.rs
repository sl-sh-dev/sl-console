use sl_console::event::*;
use sl_console::input::ConsoleReadExt;
use sl_console::*;

use std::io::{self, Write};
//use std::thread;
use std::time::Duration;

fn main() {
    con_init().unwrap();
    let mut conin = conin();
    let mut conout = conout().into_raw_mode().unwrap();
    let mut blocking = false;

    write!(
        conout,
        "{}{}",
        sl_console::clear::All,
        sl_console::cursor::Goto(1, 1)
    )
    .unwrap();

    loop {
        let evt = if blocking {
            conin.get_event().unwrap()
        } else {
            conin.get_event_timeout(Duration::from_millis(100)).unwrap()
        };

        write!(conout, "{}", sl_console::clear::CurrentLine).unwrap();
        write!(conout, "\r{:?}    <- This demonstrates the async read input char. Between each update a 100 ms. is waited, simply to demonstrate the async fashion. \n\r", evt).unwrap();
        match evt {
            Ok(evt) => match evt {
                Event::Key(key) => match key.code {
                    KeyCode::Char('q') => break,
                    KeyCode::Char('b') => blocking = !blocking,
                    _ => {}
                },
                _ => {}
            },
            Err(err) if err.kind() == io::ErrorKind::WouldBlock => {
                // Just means no data was ready (non-blocking)
            }
            Err(err) => panic!("Got BAD error {}", err),
        }

        conout.flush().unwrap();

        /* thread::sleep(Duration::from_millis(50));
        conout.write_all(b"# ").unwrap();
        conout.flush().unwrap();
        thread::sleep(Duration::from_millis(50));
        conout.write_all(b"\r #").unwrap();*/
        write!(conout, "{}", sl_console::cursor::Goto(1, 1)).unwrap();
        conout.flush().unwrap();
    }
}

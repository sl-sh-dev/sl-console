extern crate sl_console;

use sl_console::event::{Event, Key, MouseEvent};
use sl_console::input::*;
use sl_console::*;
use std::io::Write; //MouseTerminal;

fn main() {
    coninit().unwrap();
    let mut console = conout();
    let _raw = console.raw_mode_guard().unwrap();
    let mut console = MouseTerminal::from(console);

    write!(
        console,
        "{}{}q to exit. Click, click, click!",
        sl_console::clear::All,
        sl_console::cursor::Goto(1, 1)
    )
    .unwrap();
    console.flush().unwrap();

    let conin = conin();
    for c in conin.events() {
        //loop {
        //let c = console.get_event();
        let evt = c.unwrap();
        match evt {
            Event::Key(Key::Char('q')) => break,
            Event::Mouse(me) => match me {
                MouseEvent::Press(_, x, y) => {
                    write!(console, "{}x", sl_console::cursor::Goto(x, y)).unwrap();
                    //print!("{}x", sl_console::cursor::Goto(x, y));
                }
                _ => (),
            },
            _ => {}
        }
        //stdout().flush().unwrap();
        console.flush().unwrap();
    }
}

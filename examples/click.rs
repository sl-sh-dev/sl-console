extern crate termion;

use std::io::Write;
use termion::console::*;
use termion::event::{Event, Key, MouseEvent};
use termion::input::MouseTerminal;

fn main() {
    let console = console().unwrap();
    let mut console = MouseTerminal::from(console);

    write!(
        console,
        "{}{}q to exit. Click, click, click!",
        termion::clear::All,
        termion::cursor::Goto(1, 1)
    )
    .unwrap();
    console.flush().unwrap();

    //for c in console.events() {
    loop {
        let c = console.get_event();
        let evt = c.unwrap();
        match evt {
            Event::Key(Key::Char('q')) => break,
            Event::Mouse(me) => match me {
                MouseEvent::Press(_, x, y) => {
                    write!(console, "{}x", termion::cursor::Goto(x, y)).unwrap();
                    //print!("{}x", termion::cursor::Goto(x, y));
                }
                _ => (),
            },
            _ => {}
        }
        //stdout().flush().unwrap();
        console.flush().unwrap();
    }
}

use sl_console::event::{Event, KeyCode, MouseEvent};
use sl_console::input::*;
use sl_console::*;
use std::io::Write; //MouseTerminal;

fn main() {
    con_init().unwrap();
    let console = conout().into_raw_mode().unwrap();
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
            Event::Key(key) => match key.code {
                KeyCode::Char('q') => break,
                _ => (),
            },
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

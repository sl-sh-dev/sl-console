use log::LevelFilter;
use simple_logger::SimpleLogger;
use sl_console::event::{Event, KeyCode};
use sl_console::input::MouseTerminal;
use sl_console::*;
use std::io::Write;

fn main() {
    con_init().unwrap();
    SimpleLogger::new()
        .with_level(LevelFilter::Trace)
        .init()
        .unwrap();
    let mut conin = conin();
    let conout = conout().into_raw_mode().unwrap();
    let mut conout = MouseTerminal::from(conout);

    write!(
        conout,
        "{}{}q to exit. Type stuff, use alt, and so on.",
        sl_console::clear::All,
        sl_console::cursor::Goto(1, 1),
    )
    .unwrap();

    loop {
        conout.flush().unwrap();
        let c = conin.get_event();
        write!(
            conout,
            "{}{}",
            sl_console::cursor::Goto(5, 5),
            sl_console::clear::UntilNewline,
        )
        .unwrap();
        let evt = c.unwrap().unwrap();
        match evt {
            Event::Key(key) => match (key.code, key.mods) {
                (KeyCode::Char('q'), None) => break,
                _ => {
                    log::info!("Key: {:?}.", key);
                }
            },
            Event::Mouse(me) => {
                log::info!("Mouse Event: {:?}.", me);
            }
            Event::Unsupported(uns) => {
                log::info!("Unsupported: {:?}.", uns);
            }
        }
    }
}

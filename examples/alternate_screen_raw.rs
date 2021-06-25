use simple_logger::SimpleLogger;
use sl_console::event::KeyCode;
use sl_console::input::TermRead;
use sl_console::screen::*;
use sl_console::*;
use std::io::Write;

fn write_alt_screen_msg<W: Write>(screen: &mut W) {
    write!(screen, "{}{}Welcome to the alternate screen.{}Press '1' to switch to the main screen or '2' to switch to the alternate screen.{}Press 'q' to exit (and switch back to the main screen).",
           sl_console::clear::All,
           sl_console::cursor::Goto(1, 1),
           sl_console::cursor::Goto(1, 3),
           sl_console::cursor::Goto(1, 4)).unwrap();
}

fn main() {
    con_init().unwrap();
    SimpleLogger::new().init().unwrap();
    let stdin = conin();
    let _raw = conout().raw_mode_guard().unwrap();
    let mut screen = AlternateScreen::from(conout());
    write!(screen, "{}", sl_console::cursor::Hide).unwrap();
    write_alt_screen_msg(&mut screen);

    screen.flush().unwrap();

    for c in stdin.keys() {
        let key = c.unwrap();
        match key.code {
            KeyCode::Char('q') => break,
            KeyCode::Char('1') => {
                write!(screen, "{}", ToMainScreen).unwrap();
            }
            KeyCode::Char('2') => {
                write!(screen, "{}", ToAlternateScreen).unwrap();
                write_alt_screen_msg(&mut screen);
            }
            _ => {}
        }
        screen.flush().unwrap();
    }
    write!(screen, "{}", sl_console::cursor::Show).unwrap();
}

extern crate sl_console;

use sl_console::event::Key;
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
    let stdin = conin().unwrap();
    let _raw = conout().unwrap().raw_mode_guard().unwrap();
    let mut screen = AlternateScreen::from(conout().unwrap());
    write!(screen, "{}", sl_console::cursor::Hide).unwrap();
    write_alt_screen_msg(&mut screen);

    screen.flush().unwrap();

    for c in stdin.keys() {
        match c.unwrap() {
            Key::Char('q') => break,
            Key::Char('1') => {
                write!(screen, "{}", ToMainScreen).unwrap();
            }
            Key::Char('2') => {
                write!(screen, "{}", ToAlternateScreen).unwrap();
                write_alt_screen_msg(&mut screen);
            }
            _ => {}
        }
        screen.flush().unwrap();
    }
    write!(screen, "{}", sl_console::cursor::Show).unwrap();
}

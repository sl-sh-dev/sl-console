use sl_console::event::KeyCode;
use sl_console::input::ConsoleReadExt;
use sl_console::*;
use std::io::Write;

fn rainbow<W: Write>(out: &mut W, blue: u8) {
    write!(
        out,
        "{}{}",
        sl_console::cursor::Goto(1, 1),
        sl_console::clear::All
    )
    .unwrap();

    for red in 0..32 {
        let red = red * 8;
        for green in 0..64 {
            let green = green * 4;
            write!(
                out,
                "{} ",
                sl_console::color::Bg(sl_console::color::Rgb(red, green, blue))
            )
            .unwrap();
        }
        write!(out, "\n\r").unwrap();
    }

    writeln!(out, "{}b = {}", sl_console::style::Reset, blue).unwrap();
}

fn main() {
    con_init().unwrap();
    let conin = conin();
    let mut conout = conout().into_raw_mode().unwrap();

    writeln!(
        conout,
        "{}{}{}Use the up/down arrow keys to change the blue in the rainbow.",
        sl_console::clear::All,
        sl_console::cursor::Goto(1, 1),
        sl_console::cursor::Hide
    )
    .unwrap();

    let mut blue = 172u8;

    for c in conin.keys() {
        let key = c.unwrap();
        match key.code {
            KeyCode::Up => {
                blue = blue.saturating_add(4);
                rainbow(&mut conout, blue);
            }
            KeyCode::Down => {
                blue = blue.saturating_sub(4);
                rainbow(&mut conout, blue);
            }
            KeyCode::Char('q') => break,
            _ => {}
        }
        conout.flush().unwrap();
    }

    write!(conout, "{}", sl_console::cursor::Show).unwrap();
}

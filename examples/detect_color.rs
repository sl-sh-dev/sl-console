use sl_console::color::{AnsiValue, AvailableColors, Bg};
use sl_console::*;
use std::io::*;

fn main() {
    con_init().unwrap();
    let mut term = conin();
    let count = term.available_colors().unwrap();
    let mut conout = conout();

    println!("This terminal supports {} colors.", count);
    for i in 0..count {
        // If you know that stdout is connected to the console then a basic
        // println! will suffice.  Can use is_tty() to detect this... or just
        // use conout().
        //print!("{} {}", Bg(AnsiValue(i as u8)), Bg(AnsiValue(0)));
        write!(conout, "{} {}", Bg(AnsiValue(i as u8)), Bg(AnsiValue(0))).unwrap();
    }
    println!();
}

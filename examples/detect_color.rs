extern crate sl_console;

use std::io::*;
use sl_console::color::{AnsiValue, AvailableColors, Bg};

fn main() {
    let mut term = sl_console::console().unwrap();
    let count = term.available_colors().unwrap();

    sl_console::set_virtual_terminal().unwrap();
    println!("This terminal supports {} colors.", count);
    for i in 0..count {
        //print!("{} {}", Bg(AnsiValue(i as u8)), Bg(AnsiValue(0)));
        write!(term, "{} {}", Bg(AnsiValue(i as u8)), Bg(AnsiValue(0))).unwrap();
    }
    println!();
}

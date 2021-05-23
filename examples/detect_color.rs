extern crate termion;

use std::io::*;
use termion::color::{AnsiValue, Bg};

fn main() {
    let mut term = termion::console().unwrap();
    let count = term.available_colors().unwrap();

    termion::set_virtual_terminal().unwrap();
    println!("This terminal supports {} colors.", count);
    for i in 0..count {
        //print!("{} {}", Bg(AnsiValue(i as u8)), Bg(AnsiValue(0)));
        write!(term, "{} {}", Bg(AnsiValue(i as u8)), Bg(AnsiValue(0))).unwrap();
    }
    println!();
}

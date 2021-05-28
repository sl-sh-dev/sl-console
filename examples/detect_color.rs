extern crate sl_console;

//use std::io::*;
use sl_console::color::{AnsiValue, AvailableColors, Bg};

fn main() {
    let mut term = sl_console::conin().unwrap();
    let count = term.available_colors().unwrap();
    //let mut conout = sl_console::conout().unwrap();

    sl_console::set_virtual_terminal().unwrap();
    println!("This terminal supports {} colors.", count);
    for i in 0..count {
        print!("{} {}", Bg(AnsiValue(i as u8)), Bg(AnsiValue(0)));
        //write!(conout, "{} {}", Bg(AnsiValue(i as u8)), Bg(AnsiValue(0))).unwrap();
    }
    println!();
}

extern crate termion;

use termion::color::{AnsiValue, Bg};

fn main() {
    let count;
    {
        let mut term = termion::console().unwrap();
        count = term.available_colors().unwrap();
    }

    termion::set_virtual_terminal().unwrap();
    println!("This terminal supports {} colors.", count);
    for i in 0..count {
        print!("{} {}", Bg(AnsiValue(i as u8)), Bg(AnsiValue(0)));
    }
    println!();
}

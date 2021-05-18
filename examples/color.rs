extern crate termion;

use termion::{color, style};

fn main() {
    termion::set_virtual_terminal().unwrap();
    println!("{}Red", color::Fg(color::Red));
    println!("{}Blue", color::Fg(color::Blue));
    println!("{}Blue'n'Bold{}", style::Bold, style::Reset);
    println!("{}Just plain italic{}", style::Italic, style::Reset);
}

extern crate sl_console;

use sl_console::{color, style};

fn main() {
    println!("{}Red", color::Fg(color::Red));
    println!("{}Blue", color::Fg(color::Blue));
    println!("{}Blue'n'Bold{}", style::Bold, style::Reset);
    println!("{}Just plain italic{}", style::Italic, style::Reset);
}

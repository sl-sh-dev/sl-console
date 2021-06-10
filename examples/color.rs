extern crate sl_console;

use sl_console::{color, con_init, style};

fn main() {
    // Not using conin/conout so this is not needed on unix but on windows it
    // will make sure that the console can handle escape codes.
    // XXX Add a better way to enable escape codes on windows for something
    // lightweight like this?
    con_init().unwrap();
    println!("{}Red", color::Fg(color::Red));
    println!("{}Blue", color::Fg(color::Blue));
    println!("{}Blue'n'Bold{}", style::Bold, style::Reset);
    println!("{}Just plain italic{}", style::Italic, style::Reset);
}

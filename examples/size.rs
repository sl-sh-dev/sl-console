extern crate sl_console;

use sl_console::terminal_size;

fn main() {
    println!("Size is {:?}", terminal_size().unwrap())
}

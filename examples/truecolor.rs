extern crate termion;

use std::{thread, time};
use termion::{clear, color, cursor};

fn main() {
    termion::set_virtual_terminal().unwrap();
    for r in 0..255 {
        let c = color::Rgb(r, !r, 2 * ((r % 128) as i8 - 64).abs() as u8);
        println!("{}{}{}wow", cursor::Goto(1, 1), color::Bg(c), clear::All);
        thread::sleep(time::Duration::from_millis(100));
    }
}

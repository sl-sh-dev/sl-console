extern crate sl_console;

use sl_console::conout;
use sl_console::screen::*;
use std::io::Write;
use std::{thread, time};

fn main() {
    {
        let mut screen = AlternateScreen::from(conout().unwrap());
        write!(screen, "Welcome to the alternate screen.\n\nPlease wait patiently until we arrive back at the main screen in a about three seconds.").unwrap();
        screen.flush().unwrap();

        thread::sleep(time::Duration::from_secs(3));
    }

    println!("Phew! We are back.");
}

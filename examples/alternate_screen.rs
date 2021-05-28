extern crate sl_console;

use std::io::{stdout, Write};
use std::{thread, time};
use sl_console::screen::*;

fn main() {
    sl_console::set_virtual_terminal().unwrap();
    {
        let mut screen = AlternateScreen::from(stdout());
        write!(screen, "Welcome to the alternate screen.\n\nPlease wait patiently until we arrive back at the main screen in a about three seconds.").unwrap();
        screen.flush().unwrap();

        thread::sleep(time::Duration::from_secs(3));
    }

    println!("Phew! We are back.");
}

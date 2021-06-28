use sl_console::screen::*;
use sl_console::{con_init, conout};

use std::io::Write;
use std::{thread, time};

fn main() {
    {
        con_init().unwrap();
        let mut screen = AlternateScreen::from(conout());
        write!(screen, "Welcome to the alternate screen.\n\nPlease wait patiently until we arrive back at the main screen in a about three seconds.").unwrap();
        screen.flush().unwrap();

        thread::sleep(time::Duration::from_secs(3));
    }

    println!("Phew! We are back.");
}

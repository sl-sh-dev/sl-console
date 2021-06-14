use simple_logger::SimpleLogger;
use sl_console::terminal_size;

fn main() {
    SimpleLogger::new().init().unwrap();
    println!("Size is {:?}", terminal_size().unwrap())
}

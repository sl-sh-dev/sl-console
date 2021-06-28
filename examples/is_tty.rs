use std::io;

fn main() {
    let streamin = io::stdin();
    let streamout = io::stdout();

    if sl_console::is_tty(&streamin) {
        println!("STDIN is a TTY!");
    } else {
        println!("STDIN is NOT a TTY :(");
    }
    if sl_console::is_tty(&streamout) {
        println!("STDOUT is a TTY!");
    } else {
        println!("STDOUT is NOT a TTY :(");
    }
}

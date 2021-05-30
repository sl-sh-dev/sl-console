extern crate winapi;

#[derive(Clone, Copy, Debug)]
pub struct Termios(u32, u32); // (input flags, output flags)

pub mod attr;
pub mod console;
pub mod size;
pub mod tty;

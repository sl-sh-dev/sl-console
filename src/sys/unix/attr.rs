use std::{io, mem};

use super::libc::c_int;
use super::{cvt, Termios};

extern "C" {
    pub fn tcgetattr(fd: c_int, termptr: *mut Termios) -> c_int;
    pub fn tcsetattr(fd: c_int, opt: c_int, termptr: *const Termios) -> c_int;
    pub fn cfmakeraw(termptr: *mut Termios);
}

pub fn get_terminal_attr() -> io::Result<Termios> {
    get_terminal_attr_fd(1)
}

pub fn get_terminal_attr_fd(fd: i32) -> io::Result<Termios> {
    unsafe {
        let mut termios = mem::zeroed();
        cvt(tcgetattr(fd, &mut termios))?;
        Ok(termios)
    }
}

pub fn set_terminal_attr(termios: &Termios) -> io::Result<()> {
    set_terminal_attr_fd(1, termios)
}

pub fn set_terminal_attr_fd(fd: i32, termios: &Termios) -> io::Result<()> {
    cvt(unsafe { tcsetattr(fd, 0, termios) }).and(Ok(()))
}

pub fn raw_terminal_attr(termios: &mut Termios) {
    unsafe { cfmakeraw(termios) }
}

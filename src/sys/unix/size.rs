use std::ffi::CString;
use std::{io, mem};

use super::cvt;
use libc::{c_ushort, close, ioctl, open, TIOCGWINSZ};

#[repr(C)]
struct TermSize {
    row: c_ushort,
    col: c_ushort,
    x: c_ushort,
    y: c_ushort,
}
/// Get the size of the terminal.
pub fn terminal_size() -> io::Result<(u16, u16)> {
    let f = CString::new("/dev/tty").unwrap();
    unsafe {
        let mut size: TermSize = mem::zeroed();
        let fd = open(f.as_ptr(), 0);
        cvt(ioctl(fd, TIOCGWINSZ, &mut size as *mut _))?;
        close(fd);
        Ok((size.col as u16, size.row as u16))
    }
}

/// Get the size of the terminal, in pixels
pub fn terminal_size_pixels() -> io::Result<(u16, u16)> {
    let f = CString::new("/dev/tty").unwrap();
    unsafe {
        let mut size: TermSize = mem::zeroed();
        let fd = open(f.as_ptr(), 0);
        cvt(ioctl(fd, TIOCGWINSZ, &mut size as *mut _))?;
        close(fd);
        Ok((size.x as u16, size.y as u16))
    }
}

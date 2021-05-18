use std::os::unix::io::AsRawFd;
use std::{
    fs,
    io::{self, Read, Write},
};

use super::libc;

/// Is this stream a TTY?
pub fn is_tty<T: AsRawFd>(stream: &T) -> bool {
    unsafe { libc::isatty(stream.as_raw_fd()) == 1 }
}

/// Get the TTY device.
///
/// This allows for getting stdio representing _only_ the TTY, and not other streams.
pub fn get_tty() -> io::Result<impl Read + Write> {
    fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open("/dev/tty")
}

/// Make sure the Windows console will handle terminal escape codes.
///
/// This is a noop everywhere but Windows.
pub fn set_virtual_terminal() -> io::Result<()> {
    Ok(())
}

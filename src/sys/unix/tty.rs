use std::os::unix::io::AsRawFd;

use super::libc;

/// Is this stream a TTY?
pub fn is_tty<T: AsRawFd>(stream: &T) -> bool {
    unsafe { libc::isatty(stream.as_raw_fd()) == 1 }
}

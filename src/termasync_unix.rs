//! Support async reading of the tty/console.

use libc::{self, timeval};
use std::fs::{File, OpenOptions};
use std::io::{self, Read};
use std::os::unix::fs::OpenOptionsExt;
use std::os::unix::io::{AsRawFd, RawFd};
use std::time::Duration;

/// Construct an asynchronous handle to the TTY standard input, with a delimiter byte.
///
/// This version use non-blocking IO not a thread so is the same as async_stdin.
pub fn async_stdin_until(_delimiter: u8) -> io::Result<AsyncReader> {
    async_stdin()
}

/// Construct an asynchronous handle to the TTY standard input.
///
/// This allows you to read from standard input _without blocking_ the current thread.
/// Specifically, it works by opening up the tty device non-blocking.
///
/// This will not read the piped standard input, but rather read from the TTY device, since reading
/// asyncronized from piped input would rarely make sense. In other words, if you pipe standard
/// output from another process, it won't be reflected in the stream returned by this function, as
/// this represents the TTY device, and not the piped standard input.
pub fn async_stdin() -> io::Result<AsyncReader> {
    let tty = OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NONBLOCK)
        .open("/dev/tty")
        .unwrap();
    Ok(AsyncReader { tty })
}

/// An asynchronous reader.
///
/// This acts as any other stream, with the exception that reading from it won't block. Instead,
/// the buffer will only be partially updated based on how much the internal buffer holds.
pub struct AsyncReader {
    tty: File,
}

/// A blocker for an asynchronous reader.
///
/// This is useful when you need to block waiting on new data withoug a spin
/// loop or sleeps.
pub struct AsyncBlocker {
    tty_fd: RawFd,
}

impl AsyncBlocker {
    /// Block until more data is ready.
    ///
    /// Assume this can be interupted.
    pub fn block(&mut self) {
        let mut rfdset = unsafe { std::mem::MaybeUninit::uninit().assume_init() };
        unsafe {
            libc::FD_ZERO(&mut rfdset);
            libc::FD_SET(self.tty_fd, &mut rfdset);
        }
        unsafe {
            libc::select(
                self.tty_fd + 1,
                &mut rfdset,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
            );
        }
    }

    /// Block until more data is ready with a timeout.
    ///
    /// Assume this can be interupted.
    /// Returns true if the block timed out vs more data was ready.
    pub fn block_timeout(&mut self, timeout: Duration) -> bool {
        let mut rfdset = unsafe { std::mem::MaybeUninit::uninit().assume_init() };
        unsafe {
            libc::FD_ZERO(&mut rfdset);
            libc::FD_SET(self.tty_fd, &mut rfdset);
        }
        let timeout_us = if timeout.as_micros() < i64::MAX as u128 {
            timeout.as_micros() as i64
        } else {
            i64::MAX
        };
        let mut tv = timeval {
            tv_sec: 0,
            tv_usec: timeout_us,
        };
        unsafe {
            libc::select(
                self.tty_fd + 1,
                &mut rfdset,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                &mut tv,
            ) == 0
        }
    }
}

impl AsyncReader {
    /// Return a blocker struct.
    ///
    /// This can be used to block or block with a timeout on the AsyncReader.
    pub fn blocker(&mut self) -> AsyncBlocker {
        let tty_fd = self.tty.as_raw_fd();
        AsyncBlocker { tty_fd }
    }
}

impl Read for AsyncReader {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.tty.read(buf)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use std::io::Read;

    #[test]
    fn test_async_stdin() {
        let stdin = async_stdin().unwrap();
        stdin.bytes().next();
    }
}

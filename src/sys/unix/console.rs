//! Support access to the tty/console.

use lazy_static::lazy_static;
use libc::{self, timeval};
use std::fs::{File, OpenOptions};
use std::io::{self, Read, Write};
use std::os::unix::fs::OpenOptionsExt;
use std::os::unix::io::AsRawFd;
use std::sync::{Mutex, MutexGuard};
use std::time::Duration;

fn open_tty() -> File {
    OpenOptions::new()
        .read(true)
        .write(true)
        .custom_flags(libc::O_NONBLOCK)
        .open("/dev/tty")
        .unwrap()
}

lazy_static! {
    // Provide a protected singleton for the tty.  There is only one so try to
    // enforce that to avoid a myriad of issues.
    static ref INTERNAL_TTY: Mutex<File> = Mutex::new(open_tty());
}

/// Lock and return the terminal (tty/console) for the application.
///
/// This provides a Read/Write object that is connected to /dev/tty.
/// This will not read the piped standard input, but rather read from the TTY device, since reading
/// asyncronized from piped input would rarely make sense. In other words, if you pipe standard
/// output from another process, it won't be reflected in the stream returned by this function, as
/// this represents the TTY device, and not the piped standard input.
pub fn console<'a>() -> io::Result<Console<'a>> {
    Ok(Console {
        tty: INTERNAL_TTY.lock().unwrap(),
        leftover: None,
        blocking: true,
    })
}

/// Represents a tty/console terminal.
///
/// This is a singleton that aquires a lock when grabbed via get_term.  It
/// should be used to access the tty/terminal to avoid conflicts and other
/// issues.
pub struct Console<'a> {
    tty: MutexGuard<'a, File>,
    pub(crate) leftover: Option<u8>,
    pub(crate) blocking: bool,
}

impl<'a> Console<'a> {
    /// Return when more data is avialable.
    ///
    /// Calls to a get_* function should return a value now.
    /// Assume this can be interupted.
    pub fn poll(&mut self) {
        let tty_fd = self.tty.as_raw_fd();
        let mut rfdset = unsafe { std::mem::MaybeUninit::uninit().assume_init() };
        unsafe {
            libc::FD_ZERO(&mut rfdset);
            libc::FD_SET(tty_fd, &mut rfdset);
        }
        unsafe {
            libc::select(
                tty_fd + 1,
                &mut rfdset,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
            );
        }
    }

    /// Return more data is ready or the timeout is reached.
    ///
    /// Assume this can be interupted.
    /// Returns true if the more data was ready, false if timed out.
    pub fn poll_timeout(&mut self, timeout: Duration) -> bool {
        let tty_fd = self.tty.as_raw_fd();
        let mut rfdset = unsafe { std::mem::MaybeUninit::uninit().assume_init() };
        unsafe {
            libc::FD_ZERO(&mut rfdset);
            libc::FD_SET(tty_fd, &mut rfdset);
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
                tty_fd + 1,
                &mut rfdset,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                &mut tv,
            ) == 1
        }
    }
}

impl<'a> Read for Console<'a> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        if self.blocking {
            self.poll();
        }
        self.tty.read(buf)
    }
}

impl<'a> Write for Console<'a> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.tty.write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.tty.flush()
    }
}

//! Support access to the tty/console.

use lazy_static::lazy_static;
use libc::{self, timeval};
use std::fs::{File, OpenOptions};
use std::io::{self, Read, Write};
use std::os::unix::fs::OpenOptionsExt;
use std::os::unix::io::AsRawFd;
use std::sync::{Mutex, MutexGuard};
use std::time::Duration;

use super::Termios;
use crate::sys::attr::{get_terminal_attr_fd, raw_terminal_attr, set_terminal_attr_fd};

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

/// Lock and return the system specific part of the tty/console for the application.
///
/// This provides a Read/Write object that is connected to /dev/tty.
/// This will not read the piped standard input, but rather read from the TTY device, since reading
/// asyncronized from piped input would rarely make sense. In other words, if you pipe standard
/// output from another process, it won't be reflected in the stream returned by this function, as
/// this represents the TTY device, and not the piped standard input.
pub fn sys_console<'a>() -> io::Result<SysConsole<'a>> {
    Ok(SysConsole {
        tty: INTERNAL_TTY.lock().unwrap(),
        prev_ios: None,
    })
}

/// Represents system specific part of a tty/console terminal.
///
/// This is a singleton that aquires a lock when grabbed via get_term.  It
/// is part of the general Console struct.
pub struct SysConsole<'a> {
    tty: MutexGuard<'a, File>,
    prev_ios: Option<Termios>,
}

impl<'a> Drop for SysConsole<'a> {
    fn drop(&mut self) {
        if self.suspend_raw_mode().is_err() {}
    }
}

impl<'a> SysConsole<'a> {
    /// Temporarily switch to original mode
    pub fn suspend_raw_mode(&self) -> io::Result<()> {
        if let Some(prev_ios) = self.prev_ios {
            set_terminal_attr_fd(self.tty.as_raw_fd(), &prev_ios)?;
        }
        Ok(())
    }

    /// Temporarily switch to raw mode
    pub fn activate_raw_mode(&mut self) -> io::Result<()> {
        let tty_fd = self.tty.as_raw_fd();
        let mut ios = get_terminal_attr_fd(tty_fd)?;
        raw_terminal_attr(&mut ios);
        set_terminal_attr_fd(tty_fd, &ios)?;
        if self.prev_ios.is_none() {
            self.prev_ios = Some(ios);
        }
        Ok(())
    }

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

impl<'a> Read for SysConsole<'a> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.tty.read(buf)
    }
}

impl<'a> Write for SysConsole<'a> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.tty.write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.tty.flush()
    }
}

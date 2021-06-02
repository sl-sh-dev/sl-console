//! Support access to the tty/console.

use libc::{self, suseconds_t, timeval};
use std::fs::{File, OpenOptions};
use std::io::{self, Read, Write};
use std::os::unix::fs::OpenOptionsExt;
use std::os::unix::io::AsRawFd;
use std::time::Duration;

use super::Termios;
use crate::sys::attr::{get_terminal_attr_fd, raw_terminal_attr, set_terminal_attr_fd};

/// Open and return the read side of a tty.
pub fn open_syscon_in() -> io::Result<SysConsoleIn> {
    let tty = OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NONBLOCK)
        .open("/dev/tty")?;
    Ok(SysConsoleIn { tty })
}

/// Open and return the write side of a tty.
pub fn open_syscon_out() -> io::Result<SysConsoleOut> {
    let tty = OpenOptions::new().write(true).open("/dev/tty")?;
    let tty_fd = tty.as_raw_fd();
    let ios = get_terminal_attr_fd(tty_fd)?;
    let prev_ios = ios;
    Ok(SysConsoleOut { tty, prev_ios })
}

/// Represents system specific part of a tty/console output.
pub struct SysConsoleOut {
    tty: File,
    prev_ios: Termios,
}

impl Drop for SysConsoleOut {
    fn drop(&mut self) {
        if self.suspend_raw_mode().is_err() {}
    }
}

impl SysConsoleOut {
    /// Temporarily switch to original mode
    pub fn suspend_raw_mode(&self) -> io::Result<()> {
        set_terminal_attr_fd(self.tty.as_raw_fd(), &self.prev_ios)?;
        Ok(())
    }

    /// Switch back to raw mode
    pub fn activate_raw_mode(&mut self) -> io::Result<()> {
        let tty_fd = self.tty.as_raw_fd();
        let mut ios = get_terminal_attr_fd(tty_fd)?;
        raw_terminal_attr(&mut ios);
        set_terminal_attr_fd(tty_fd, &ios)?;
        Ok(())
    }
}

impl Write for SysConsoleOut {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.tty.write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.tty.flush()
    }
}

/// Represents system specific part of a tty/console input.
pub struct SysConsoleIn {
    tty: File,
}

impl SysConsoleIn {
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
        let timeout_us = if timeout.as_micros() < suseconds_t::MAX as u128 {
            timeout.as_micros() as suseconds_t
        } else {
            suseconds_t::MAX
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

    /// Read from the byte stream.
    ///
    /// This version blocks, the read from the Read trait does not.
    pub fn read_block(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.poll();
        self.read(buf)
    }
}

impl Read for SysConsoleIn {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.tty.read(buf)
    }
}

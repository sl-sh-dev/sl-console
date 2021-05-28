//! Support async reading of the tty/console.

use std::fs::{File, OpenOptions};
use std::io::{self, Read, Write};
use std::thread;
use std::time::Duration;

use crossbeam_channel::*;

use super::Termios;
use crate::sys::attr::{get_terminal_attr, raw_terminal_attr, set_terminal_attr};
use crate::sys::tty::set_virtual_terminal;

/// Open and return the read side of a console.
pub fn open_syscon_in() -> io::Result<SysConsoleIn> {
    let tty = OpenOptions::new().read(true).open("CONIN$")?;
    set_virtual_terminal()?;
    let (send, recv) = unbounded();
    thread::spawn(move || {
        for i in tty.bytes() {
            if send.send(i).is_err() {
                return;
            }
        }
    });
    Ok(SysConsoleIn { recv })
}

/// Open and return the write side of a console.
pub fn open_syscon_out() -> io::Result<SysConsoleOut> {
    let tty = OpenOptions::new().write(true).open("CONOUT$")?;
    Ok(SysConsoleOut {
        tty,
        prev_ios: None,
    })
}

/// Represents system specific part of a tty/console output.
pub struct SysConsoleOut {
    tty: File,
    prev_ios: Option<Termios>,
}

/// An asynchronous reader.
///
/// This acts as any other stream, with the exception that reading from it won't block. Instead,
/// the buffer will only be partially updated based on how much the internal buffer holds.
pub struct SysConsoleIn {
    /// The underlying receiver.
    recv: Receiver<io::Result<u8>>,
}

impl Drop for SysConsoleOut {
    fn drop(&mut self) {
        if self.suspend_raw_mode().is_err() {}
    }
}

impl SysConsoleOut {
    /// Temporarily switch to original mode
    pub fn suspend_raw_mode(&self) -> io::Result<()> {
        if let Some(prev_ios) = self.prev_ios {
            set_terminal_attr(&prev_ios)?;
        }
        Ok(())
    }

    /// Temporarily switch to raw mode
    pub fn activate_raw_mode(&mut self) -> io::Result<()> {
        let mut ios = get_terminal_attr()?;
        if self.prev_ios.is_none() {
            self.prev_ios = Some(ios);
        }
        raw_terminal_attr(&mut ios);
        set_terminal_attr(&ios)?;
        Ok(())
    }
}

impl SysConsoleIn {
    /// Return when more data is avialable.
    ///
    /// Calls to a get_* function should return a value now.
    /// Assume this can be interupted.
    pub fn poll(&mut self) {
        let mut sel = Select::new();
        sel.recv(&self.recv);
        sel.ready();
    }

    /// Return more data is ready or the timeout is reached.
    ///
    /// Assume this can be interupted.
    /// Returns true if the more data was ready, false if timed out.
    pub fn poll_timeout(&mut self, timeout: Duration) -> bool {
        let mut sel = Select::new();
        sel.recv(&self.recv);
        sel.ready_timeout(timeout).is_ok()
    }
}

impl Read for SysConsoleIn {
    /// Read from the byte stream.
    ///
    /// This will never block, but try to drain the event queue until empty. If the total number of
    /// bytes written is lower than the buffer's length, the event queue is empty or that the event
    /// stream halted.
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let mut total = 0;

        loop {
            if total >= buf.len() {
                break;
            }

            match self.recv.try_recv() {
                Ok(Ok(b)) => {
                    buf[total] = b;
                    total += 1;
                }
                Ok(Err(e)) => return Err(e),
                Err(err) if err == TryRecvError::Empty && total == 0 => {
                    return Err(io::Error::new(io::ErrorKind::WouldBlock, ""))
                }
                Err(_) => break,
            }
        }

        Ok(total)
    }
}

impl Write for SysConsoleOut {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.tty.write(buf)
        //io::stdout().write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.tty.flush()
        //io::stdout().flush()
    }
}

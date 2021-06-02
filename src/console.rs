//! Support access to the tty/console.

use std::cell::RefCell;
use std::io::{self, Read, Write};
use std::time::Duration;

use lazy_static::lazy_static;
use parking_lot::*;

use crate::event::{Event, Key};
use crate::input::event_and_raw;
use crate::sys::console::*;

fn make_tty_in() -> io::Result<ReentrantMutex<RefCell<ConsoleIn>>> {
    let syscon = open_syscon_in()?;
    Ok(ReentrantMutex::new(RefCell::new(ConsoleIn {
        syscon,
        leftover: None,
        blocking: true,
    })))
}

fn make_tty_out() -> io::Result<ReentrantMutex<RefCell<ConsoleOut>>> {
    let syscon = open_syscon_out()?;
    Ok(ReentrantMutex::new(RefCell::new(ConsoleOut {
        syscon,
        raw_mode: false,
    })))
}

lazy_static! {
    // Provide a protected singletons for the console.  There is only one so
    // try to enforce that to avoid a myriad of issues (split into in and out).
    static ref CONSOLE_IN: io::Result<ReentrantMutex<RefCell<ConsoleIn>>> = make_tty_in();
    static ref CONSOLE_OUT: io::Result<ReentrantMutex<RefCell<ConsoleOut>>> = make_tty_out();
}

/// Lock and return read side of the tty/console for the application.
///
/// This provides a Read object that is connected to /dev/tty (unix) or
/// the console (windows).  This will not read the piped standard input, but
/// rather read from the TTY or console device, since reading asyncronized
/// from piped input would rarely make sense. In other words, if you pipe
/// standard output from another process, it won't be reflected in the stream
/// returned by this function, as this represents the TTY/console device, and
/// not the piped standard input.
pub fn conin<'a>() -> io::Result<ConsoleInLock<'a>> {
    match &*CONSOLE_IN {
        Ok(conin) => Ok(ConsoleInLock {
            inner: conin.lock(),
        }),
        Err(err) => Err(io::Error::new(err.kind(), err)),
    }
}

/// Lock and return write side of the tty/console for the application.
///
/// This provides a Write object that is connected to /dev/tty (unix) or
/// the console (windows).  This will not write to standard output (if it is
/// not the tty/console), but rather write to the TTY or console device.
/// In other words, if you pipe standard output to another process things you
/// write to conout() will not go into the pipe but will go to the terminal.
pub fn conout<'a>() -> io::Result<ConsoleOutLock<'a>> {
    match &*CONSOLE_OUT {
        Ok(conout) => Ok(ConsoleOutLock {
            inner: conout.lock(),
        }),
        Err(err) => Err(io::Error::new(err.kind(), err)),
    }
}

/// RAII guard for entering raw mode, will restore previous mode when dropped.
pub struct RawModeGuard {
    old_raw: bool,
}

impl Drop for RawModeGuard {
    fn drop(&mut self) {
        if !self.old_raw {
            if let Ok(mut conout) = conout() {
                if conout.raw_mode_off().is_err() {} // Ignore error in drop.
            }
        }
    }
}

/// Console output trait.
pub trait ConsoleWrite: Write {
    /// Switch to original (non-raw) mode
    fn raw_mode_off(&mut self) -> io::Result<()>;
    /// Switch to raw mode
    fn raw_mode_on(&mut self) -> io::Result<()>;
    /// Switch to raw mode and return a RAII guard to switch to previous mode
    /// when scope ends.
    fn raw_mode_guard(&mut self) -> io::Result<RawModeGuard>;
    /// True if in raw mode.
    fn is_raw_mode(&self) -> bool;
}

/// Console input trait.
pub trait ConsoleRead: Read {
    /// Set whether the console is blocking or non-blocking.
    ///
    /// Default is blocking.  If non blocking then the get_* functions can
    /// return WouldBlock errors if no data is ready.  The poll functions
    /// will work whether in blocking or non blocking mode.
    fn set_blocking(&mut self, blocking: bool);

    /// Is this console blocking or non-blocking?
    fn is_blocking(&self) -> bool;

    /// Get the next input event from the tty and the bytes that define it.
    ///
    /// If the tty is non-blocking then can return a WouldBlock error.
    fn get_event_and_raw(&mut self) -> io::Result<(Event, Vec<u8>)>;

    /// Get the next input event from the tty.
    ///
    /// If the tty is non-blocking then can return a WouldBlock error.
    fn get_event(&mut self) -> io::Result<Event>;

    /// Get the next key event from the tty.
    ///
    /// This will skip over non-key events (they will be lost).
    /// If the tty is non-blocking then can return a WouldBlock error.
    fn get_key(&mut self) -> io::Result<Key>;

    /// Return when more data is avialable.
    ///
    /// Calls to a get_* function should return a value now.
    /// Assume this can be interupted.
    fn poll(&mut self);

    /// Return more data is ready or the timeout is reached.
    ///
    /// Assume this can be interupted.
    /// Returns true if the more data was ready, false if timed out.
    fn poll_timeout(&mut self, timeout: Duration) -> bool;
}

/// Represents the input side of the tty/console terminal.
///
/// This is a singleton that aquires a lock to access the console (similiar to
/// Stdin).  It should be used to access the tty/terminal to avoid conflicts
/// and other issues.
pub struct ConsoleIn {
    syscon: SysConsoleIn,
    leftover: Option<u8>,
    blocking: bool,
}

/// A locked console input device.
pub struct ConsoleInLock<'a> {
    inner: ReentrantMutexGuard<'a, RefCell<ConsoleIn>>,
}

/// Represents the output side of the tty/console terminal.
///
/// This is a singleton that aquires a lock to access the console (similiar to
/// Stdin).  It should be used to access the tty/terminal to avoid conflicts
/// and other issues.
pub struct ConsoleOut {
    syscon: SysConsoleOut,
    raw_mode: bool,
}

/// A locked console output device.
pub struct ConsoleOutLock<'a> {
    inner: ReentrantMutexGuard<'a, RefCell<ConsoleOut>>,
}

impl ConsoleRead for ConsoleIn {
    fn set_blocking(&mut self, blocking: bool) {
        self.blocking = blocking;
    }

    fn is_blocking(&self) -> bool {
        self.blocking
    }

    fn get_event_and_raw(&mut self) -> io::Result<(Event, Vec<u8>)> {
        let mut leftover = self.leftover.take();
        if let Some(er) = event_and_raw(self, &mut leftover) {
            er
        } else {
            Err(io::Error::new(
                io::ErrorKind::WouldBlock,
                "Event stream would block",
            ))
        }
    }

    fn get_event(&mut self) -> io::Result<Event> {
        match self.get_event_and_raw() {
            Ok((event, _raw)) => Ok(event),
            Err(err) => Err(err),
        }
    }

    fn get_key(&mut self) -> io::Result<Key> {
        loop {
            match self.get_event() {
                Ok(Event::Key(k)) => return Ok(k),
                Ok(_) => continue,
                Err(e) => return Err(e),
            }
        }
    }

    fn poll(&mut self) {
        self.syscon.poll();
    }

    fn poll_timeout(&mut self, timeout: Duration) -> bool {
        self.syscon.poll_timeout(timeout)
    }
}

impl Read for ConsoleIn {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        if self.blocking {
            self.syscon.read_block(buf)
        } else {
            self.syscon.read(buf)
        }
    }
}

impl<'a> ConsoleRead for ConsoleInLock<'a> {
    fn set_blocking(&mut self, blocking: bool) {
        self.inner.borrow_mut().blocking = blocking;
    }

    fn is_blocking(&self) -> bool {
        self.inner.borrow().blocking
    }

    fn get_event_and_raw(&mut self) -> io::Result<(Event, Vec<u8>)> {
        self.inner.borrow_mut().get_event_and_raw()
    }

    fn get_event(&mut self) -> io::Result<Event> {
        self.inner.borrow_mut().get_event()
    }

    fn get_key(&mut self) -> io::Result<Key> {
        self.inner.borrow_mut().get_key()
    }

    fn poll(&mut self) {
        self.inner.borrow_mut().poll();
    }

    fn poll_timeout(&mut self, timeout: Duration) -> bool {
        self.inner.borrow_mut().poll_timeout(timeout)
    }
}

impl<'a> Read for ConsoleInLock<'a> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.inner.borrow_mut().read(buf)
    }
}

impl ConsoleWrite for ConsoleOut {
    fn raw_mode_off(&mut self) -> io::Result<()> {
        if self.raw_mode {
            self.raw_mode = false;
            self.syscon.suspend_raw_mode()?;
        }
        Ok(())
    }

    fn raw_mode_on(&mut self) -> io::Result<()> {
        if !self.raw_mode {
            self.raw_mode = true;
            self.syscon.activate_raw_mode()?;
        }
        Ok(())
    }

    fn raw_mode_guard(&mut self) -> io::Result<RawModeGuard> {
        let old_raw = self.raw_mode;
        if !self.raw_mode {
            self.raw_mode = true;
            self.syscon.activate_raw_mode()?;
        }
        Ok(RawModeGuard { old_raw })
    }

    fn is_raw_mode(&self) -> bool {
        self.raw_mode
    }
}

impl Write for ConsoleOut {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.syscon.write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.syscon.flush()
    }
}

impl<'a> ConsoleWrite for ConsoleOutLock<'a> {
    fn raw_mode_off(&mut self) -> io::Result<()> {
        self.inner.borrow_mut().raw_mode_off()
    }

    fn raw_mode_on(&mut self) -> io::Result<()> {
        self.inner.borrow_mut().raw_mode_on()
    }

    fn raw_mode_guard(&mut self) -> io::Result<RawModeGuard> {
        self.inner.borrow_mut().raw_mode_guard()
    }

    fn is_raw_mode(&self) -> bool {
        self.inner.borrow().is_raw_mode()
    }
}

impl<'a> Write for ConsoleOutLock<'a> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.inner.borrow_mut().write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.inner.borrow_mut().flush()
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use std::io::Read;

    #[test]
    fn test_async_stdin() {
        let mut tty = conin().unwrap();
        tty.set_blocking(false);
        tty.bytes().next();
    }
}

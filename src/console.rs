//! Support access to the tty/console.

use std::io::{self, Read, Write};

use crate::event::{Event, Key};
use crate::input::event_and_raw;
use crate::sys::console::*;

/// An RAII guard that allows the console to be put in blocking or non-blocking
/// mode and the previos state will be restored when it goes out of scope.
pub struct ConsoleBlocker<'console, 'conref> {
    console: &'conref mut Console<'console>,
    old_blocking: bool,
}

impl<'console, 'conref> Drop for ConsoleBlocker<'console, 'conref> {
    fn drop(&mut self) {
        self.console.set_blocking(self.old_blocking);
    }
}

impl<'console, 'conref> Read for ConsoleBlocker<'console, 'conref> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.console.read(buf)
    }
}

impl<'console, 'conref> Write for ConsoleBlocker<'console, 'conref> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.console.write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.console.flush()
    }
}

impl<'a> Console<'a> {
    /// Set whether the tty is blocking or non-blocking.
    ///
    /// Default is blocking.  If non blocking then the get_* functions can
    /// return WouldBlock errors if no data is ready.  The poll functions
    /// will work whether in blocking or non blocking mode.
    pub fn set_blocking(&mut self, blocking: bool) {
        self.blocking = blocking;
    }

    /// Return an RAII guard for the console in non blocking mode.
    pub fn non_blocking<'b>(&'b mut self) -> ConsoleBlocker<'a, 'b> {
        let old_blocking = self.blocking;
        self.set_blocking(false);
        ConsoleBlocker {
            console: self,
            old_blocking,
        }
    }

    /// Return an RAII guard for the console in blocking mode.
    pub fn blocking<'b>(&'b mut self) -> ConsoleBlocker<'a, 'b> {
        let old_blocking = self.blocking;
        self.set_blocking(false);
        ConsoleBlocker {
            console: self,
            old_blocking,
        }
    }

    /// Get the next input event from the tty and the bytes that define it.
    ///
    /// If the tty is non-blocking then can return a WouldBlock error.
    pub fn get_event_and_raw(&mut self) -> io::Result<(Event, Vec<u8>)> {
        let mut leftover = self.leftover.take();
        let result = if let Some(er) = event_and_raw(self, &mut leftover) {
            er
        } else {
            Err(io::Error::new(
                io::ErrorKind::WouldBlock,
                "Event stream would block",
            ))
        };
        result
    }

    /// Get the next input event from the tty.
    ///
    /// If the tty is non-blocking then can return a WouldBlock error.
    pub fn get_event(&mut self) -> io::Result<Event> {
        match self.get_event_and_raw() {
            Ok((event, _raw)) => Ok(event),
            Err(err) => Err(err),
        }
    }

    /// Get the next key event from the tty.
    ///
    /// This will skip over non-key events (they will be lost).
    /// If the tty is non-blocking then can return a WouldBlock error.
    pub fn get_key(&mut self) -> io::Result<Key> {
        loop {
            match self.get_event() {
                Ok(Event::Key(k)) => return Ok(k),
                Ok(_) => continue,
                Err(e) => return Err(e),
            }
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use std::io::Read;

    #[test]
    fn test_async_stdin() {
        let mut tty = console().unwrap();
        tty.set_blocking(false);
        tty.bytes().next();
    }
}

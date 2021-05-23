//! Support access to the tty/console.

use std::io::{self, Read, Write};
use std::ops;
use std::time::Duration;

use crate::event::{Event, Key};
use crate::input::event_and_raw;
use crate::sys::console::*;

/// Lock and return the tty/console for the application.
///
/// This provides a Read/Write object that is connected to /dev/tty (unix) or
/// the console (windows).  This will not read the piped standard input, but
/// rather read from the TTY or console device, since reading asyncronized
/// from piped input would rarely make sense. In other words, if you pipe
/// standard output from another process, it won't be reflected in the stream
/// returned by this function, as this represents the TTY/console device, and
/// not the piped standard input.
pub fn console<'a>() -> io::Result<Console<'a>> {
    let mut syscon = sys_console()?;
    syscon.activate_raw_mode()?;
    Ok(Console {
        syscon,
        leftover: None,
        blocking: true,
    })
}
/*
pub struct Blocking<CON: ConMark> {
    console: CON,
    old_blocking: bool,
}

impl<CON: ConMark> From<CON> for Blocking<CON> {
    fn from(mut from: CON) -> Blocking<CON> {
        let old_blocking = from.is_blocking();
        from.set_blocking(true);
        Blocking { console: from, old_blocking }
    }
}

impl<CON: ConMark> Drop for Blocking<CON> {
    fn drop(&mut self) {
        self.console.set_blocking(self.old_blocking);
    }
}

impl<CON: ConMark> ops::Deref for Blocking<CON> {
    type Target = CON;

    fn deref(&self) -> &CON {
        &self.console
    }
}

impl<CON: ConMark> ops::DerefMut for Blocking<CON> {
    fn deref_mut(&mut self) -> &mut CON {
        &mut self.console
    }
}
*/
/// RAII guard for a reference to a Console for non-blocking.
/// Use this if you have a &mut Console and need it to be non-blocking while
/// the reference is in use.  Will restore the previos state when it goes out
/// of scope.
///
/// This can be obtained through the `From` implementations.
pub struct NonBlockingRef<'console, 'conref> {
    console: &'conref mut Console<'console>,
    old_blocking: bool,
}

impl<'console, 'conref> From<&'conref mut Console<'console>> for NonBlockingRef<'console, 'conref> {
    fn from(from: &'conref mut Console<'console>) -> NonBlockingRef<'console, 'conref> {
        let old_blocking = from.blocking;
        from.set_blocking(false);
        NonBlockingRef {
            console: from,
            old_blocking,
        }
    }
}

impl<'console, 'conref> Drop for NonBlockingRef<'console, 'conref> {
    fn drop(&mut self) {
        self.console.set_blocking(self.old_blocking);
    }
}

impl<'console, 'conref> ops::Deref for NonBlockingRef<'console, 'conref> {
    type Target = Console<'console>;

    fn deref(&self) -> &Console<'console> {
        &self.console
    }
}

impl<'console, 'conref> ops::DerefMut for NonBlockingRef<'console, 'conref> {
    fn deref_mut(&mut self) -> &mut Console<'console> {
        self.console
    }
}

/// Mark then console so it can be distinguished from other random Read + Write
/// things.
pub trait ConMark: Read + Write {
    /// Set whether the console is blocking or non-blocking.
    ///
    /// Default is blocking.  If non blocking then the get_* functions can
    /// return WouldBlock errors if no data is ready.  The poll functions
    /// will work whether in blocking or non blocking mode.
    fn set_blocking(&mut self, blocking: bool);

    /// Is this console blocking or non-blocking?
    fn is_blocking(&self) -> bool;
}

/// Represents a tty/console terminal.
///
/// This is a singleton that aquires a lock when grabbed via get_term.  It
/// should be used to access the tty/terminal to avoid conflicts and other
/// issues.
pub struct Console<'a> {
    syscon: SysConsole<'a>,
    leftover: Option<u8>,
    blocking: bool,
}

impl<'a> ConMark for Console<'a> {
    fn set_blocking(&mut self, blocking: bool) {
        self.blocking = blocking;
    }

    fn is_blocking(&self) -> bool {
        self.blocking
    }
}

impl<'a> Console<'a> {
    /// Get the next input event from the tty and the bytes that define it.
    ///
    /// If the tty is non-blocking then can return a WouldBlock error.
    pub fn get_event_and_raw(&mut self) -> io::Result<(Event, Vec<u8>)> {
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

    /// Return when more data is avialable.
    ///
    /// Calls to a get_* function should return a value now.
    /// Assume this can be interupted.
    pub fn poll(&mut self) {
        self.syscon.poll();
    }

    /// Return more data is ready or the timeout is reached.
    ///
    /// Assume this can be interupted.
    /// Returns true if the more data was ready, false if timed out.
    pub fn poll_timeout(&mut self, timeout: Duration) -> bool {
        self.syscon.poll_timeout(timeout)
    }
}

impl<'a> Read for Console<'a> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        if self.blocking {
            self.syscon.poll();
        }
        self.syscon.read(buf)
    }
}

impl<'a> Write for Console<'a> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.syscon.write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.syscon.flush()
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

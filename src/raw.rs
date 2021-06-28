//! Managing raw mode.
//!
//! Raw mode is a particular state a TTY can have. It signifies that:
//!
//! 1. No line buffering (the input is given byte-by-byte).
//! 2. The input is not written out, instead it has to be done manually by the programmer.
//! 3. The output is not canonicalized (for example, `\n` means "go one line down", not "line
//!    break").
//!
//! It is essential to design terminal programs.
//!
//! # Example
//!
//! ```rust,no_run
//! use sl_console::*;
//! use std::io::Write;
//!
//!     let mut conout = conout().into_raw_mode().unwrap();
//!
//!     write!(conout, "Hey there.").unwrap();
//! ```

use std::io::{self, Write};
use std::ops;

use crate::console::*;

/// A terminal restorer, which keeps the previous state of the terminal, and restores it, when
/// dropped.
///
/// Restoring will entirely bring back the old TTY state.
pub struct RawTerminal<W: ConsoleWrite> {
    prev_mode: bool,
    output: W,
}

impl<W: ConsoleWrite> Drop for RawTerminal<W> {
    fn drop(&mut self) {
        // Ignore error in drop...
        if self.output.set_raw_mode(self.prev_mode).is_err() {}
    }
}

impl<W: ConsoleWrite> ops::Deref for RawTerminal<W> {
    type Target = W;

    fn deref(&self) -> &W {
        &self.output
    }
}

impl<W: ConsoleWrite> ops::DerefMut for RawTerminal<W> {
    fn deref_mut(&mut self) -> &mut W {
        &mut self.output
    }
}

impl<W: ConsoleWrite> ConsoleWrite for RawTerminal<W> {
    fn set_raw_mode(&mut self, mode: bool) -> io::Result<bool> {
        self.output.set_raw_mode(mode)
    }

    fn is_raw_mode(&self) -> bool {
        self.output.is_raw_mode()
    }
}

impl<W: ConsoleWrite> Write for RawTerminal<W> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.output.write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.output.flush()
    }
}

#[cfg(unix)]
mod unix_impl {
    use super::*;
    use std::os::unix::io::{AsRawFd, RawFd};

    impl<W: ConsoleWrite + AsRawFd> AsRawFd for RawTerminal<W> {
        fn as_raw_fd(&self) -> RawFd {
            self.output.as_raw_fd()
        }
    }
}

#[cfg(windows)]
mod windows_impl {
    use super::*;
    use std::os::windows::io::{AsRawHandle, RawHandle};

    impl<W: ConsoleWrite + AsRawHandle> AsRawHandle for RawTerminal<W> {
        fn as_raw_handle(&self) -> RawHandle {
            self.output.as_raw_handle()
        }
    }
}

/// Types which can be converted into "raw mode".
///
/// # Why is this type defined on writers and not readers?
///
/// TTYs has their state controlled by the writer, not the reader. You use the writer to clear the
/// screen, move the cursor and so on, so naturally you use the writer to change the mode as well.
pub trait RawModeExt: ConsoleWrite + Sized {
    /// Switch to original (non-raw) mode
    ///
    /// This call needs to also lock the conin (conout will have been locked
    /// already).  If it can not lock conin it will return an error of kind
    /// WouldBlock.
    fn raw_mode_off(&mut self) -> io::Result<()>;

    /// Switch to raw mode.
    ///
    /// This call needs to also lock the conin (conout will have been locked
    /// already).  If it can not lock conin it will return an error of kind
    /// WouldBlock.
    fn raw_mode_on(&mut self) -> io::Result<()>;

    /// Switch to raw mode.
    ///
    /// Raw mode means that stdin won't be printed (it will instead have to be written manually by
    /// the program). Furthermore, the input isn't canonicalised or buffered (that is, you can
    /// read from stdin one byte of a time). The output is neither modified in any way.
    fn into_raw_mode(self) -> io::Result<RawTerminal<Self>>;
}

impl<W: ConsoleWrite> RawModeExt for W {
    fn raw_mode_off(&mut self) -> io::Result<()> {
        self.set_raw_mode(false)?;
        Ok(())
    }

    fn raw_mode_on(&mut self) -> io::Result<()> {
        self.set_raw_mode(true)?;
        Ok(())
    }

    fn into_raw_mode(mut self) -> io::Result<RawTerminal<W>> {
        let prev_mode = self.set_raw_mode(true)?;

        Ok(RawTerminal {
            prev_mode,
            output: self,
        })
    }
}

impl<W: ConsoleWrite> RawTerminal<W> {
    /// Temporarily switch to original mode
    pub fn suspend_raw_mode(&mut self) -> io::Result<()> {
        self.output.set_raw_mode(false)?;
        Ok(())
    }

    /// Temporarily switch to raw mode
    pub fn activate_raw_mode(&mut self) -> io::Result<()> {
        self.output.set_raw_mode(true)?;
        Ok(())
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use std::io::Write;

    #[test]
    fn test_into_raw_mode() {
        // Need this lock because tests are multi-threaded.
        let _conin = conin().lock();
        let mut out = conout().into_raw_mode().unwrap();

        out.write_all(b"this is a test, muahhahahah\r\n").unwrap();

        drop(out);
    }
}

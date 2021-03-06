//! Managing switching between main and alternate screen buffers.
//!
//! Note that this implementation uses xterm's new escape sequences for screen switching and thus
//! only works for xterm compatible terminals (which should be most terminals nowadays).
//!
//! # Example
//!
//! ```rust
//! use sl_console::conout;
//! use sl_console::screen::AlternateScreen;
//! use std::io::{Write, stdout};
//!
//!     {
//!         let mut screen = AlternateScreen::from(conout());
//!         write!(screen, "Writing to alternate screen!").unwrap();
//!         screen.flush().unwrap();
//!     }
//!     println!("Writing to main screen.");
//! ```

use std::fmt;
use std::io::{self, Write};
use std::ops;

use crate::console::ConsoleWrite;

/// Switch to the main screen buffer of the terminal.
pub struct ToMainScreen;

impl fmt::Display for ToMainScreen {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, csi!("?1049l"))
    }
}

/// Switch to the alternate screen buffer of the terminal.
pub struct ToAlternateScreen;

impl fmt::Display for ToAlternateScreen {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, csi!("?1049h"))
    }
}

/// A terminal restorer, which wraps a type implementing Write, and causes all writes to be written
/// to an alternate screen.
///
/// This is achieved by switching the terminal to the alternate screen on creation and
/// automatically switching it back to the original screen on drop.
pub struct AlternateScreen<W: ConsoleWrite> {
    /// The output target.
    output: W,
}

impl<W: ConsoleWrite> AlternateScreen<W> {
    /// Create an alternate screen wrapper struct for the provided output and switch the terminal
    /// to the alternate screen.
    pub fn from(mut output: W) -> Self {
        write!(output, "{}", ToAlternateScreen).expect("switch to alternate screen");
        AlternateScreen { output }
    }
}

impl<W: ConsoleWrite> Drop for AlternateScreen<W> {
    fn drop(&mut self) {
        write!(self, "{}", ToMainScreen).expect("switch to main screen");
    }
}

impl<W: ConsoleWrite> ops::Deref for AlternateScreen<W> {
    type Target = W;

    fn deref(&self) -> &W {
        &self.output
    }
}

impl<W: ConsoleWrite> ops::DerefMut for AlternateScreen<W> {
    fn deref_mut(&mut self) -> &mut W {
        &mut self.output
    }
}

impl<W: ConsoleWrite> Write for AlternateScreen<W> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.output.write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.output.flush()
    }
}

impl<W: ConsoleWrite> ConsoleWrite for AlternateScreen<W> {
    fn set_raw_mode(&mut self, mode: bool) -> io::Result<bool> {
        self.output.set_raw_mode(mode)
    }

    fn is_raw_mode(&self) -> bool {
        self.output.is_raw_mode()
    }
}

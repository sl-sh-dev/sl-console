//! Sl-console is a pure Rust, bindless library for low-level handling, manipulating
//! and reading information about terminals.  This is a fork of termion.
//!
//! Sl-console aims to be simple and yet expressive. It is bindless, meaning that it
//! is not a front-end to some other library (e.g., ncurses or termbox), but a
//! standalone library directly talking to the TTY.
//!
//! Supports Mac OS X, Linux, and Windows (or, in general, ANSI terminals).
//!
//! For more information refer to the [README](https://github.com/sl-sh-dev/sl-console).
#![warn(missing_docs)]

#[cfg(unix)]
#[path = "sys/unix/mod.rs"]
mod sys;

#[cfg(windows)]
#[path = "sys/windows/mod.rs"]
mod sys;

pub use console::{con_init, conin, conout, ConsoleRead, ConsoleWrite};
pub use input::ConsoleReadExt;
pub use raw::RawModeExt;
pub use sys::size::terminal_size;
#[cfg(unix)]
pub use sys::size::terminal_size_pixels;
pub use sys::tty::is_tty;

#[macro_use]
mod macros;
pub mod clear;
pub mod color;
pub mod console;
pub mod cursor;
pub mod event;
pub mod input;
pub mod raw;
pub mod screen;
pub mod scroll;
pub mod style;

#[cfg(test)]
mod test {
    use super::sys;

    #[test]
    fn test_size() {
        sys::size::terminal_size().unwrap();
    }
}

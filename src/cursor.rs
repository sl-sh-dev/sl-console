//! Cursor movement.

use crate::console::*;
use numtoa::NumToA;
use std::fmt;
use std::io::{self, Error, ErrorKind, Write};
use std::ops;
use std::time::{Duration, Instant};

/// The timeout of an escape code control sequence, in milliseconds.
const CONTROL_SEQUENCE_TIMEOUT: u64 = 100;

derive_csi_sequence!("Hide the cursor.", Hide, "?25l");
derive_csi_sequence!("Show the cursor.", Show, "?25h");

derive_csi_sequence!("Restore the cursor.", Restore, "u");
derive_csi_sequence!("Save the cursor.", Save, "s");

derive_csi_sequence!(
    "Change the cursor style to blinking block",
    BlinkingBlock,
    "\x31 q"
);
derive_csi_sequence!(
    "Change the cursor style to steady block",
    SteadyBlock,
    "\x32 q"
);
derive_csi_sequence!(
    "Change the cursor style to blinking underline",
    BlinkingUnderline,
    "\x33 q"
);
derive_csi_sequence!(
    "Change the cursor style to steady underline",
    SteadyUnderline,
    "\x34 q"
);
derive_csi_sequence!(
    "Change the cursor style to blinking bar",
    BlinkingBar,
    "\x35 q"
);
derive_csi_sequence!("Change the cursor style to steady bar", SteadyBar, "\x36 q");

/// Goto some position ((1,1)-based).
///
/// # Why one-based?
///
/// ANSI escapes are very poorly designed, and one of the many odd aspects is being one-based. This
/// can be quite strange at first, but it is not that big of an obstruction once you get used to
/// it.
///
/// # Example
///
/// ```rust
/// extern crate sl_console;
///
/// fn main() {
///     print!("{}{}Stuff", sl_console::clear::All, sl_console::cursor::Goto(5, 3));
/// }
/// ```
#[derive(Copy, Clone, PartialEq, Eq)]
pub struct Goto(pub u16, pub u16);

impl From<Goto> for String {
    fn from(this: Goto) -> String {
        let (mut x, mut y) = ([0u8; 20], [0u8; 20]);
        [
            "\x1B[",
            this.1.numtoa_str(10, &mut x),
            ";",
            this.0.numtoa_str(10, &mut y),
            "H",
        ]
        .concat()
    }
}

impl Default for Goto {
    fn default() -> Goto {
        Goto(1, 1)
    }
}

impl fmt::Display for Goto {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        debug_assert!(self != &Goto(0, 0), "Goto is one-based.");
        write!(f, "\x1B[{};{}H", self.1, self.0)
    }
}

/// Move cursor left.
#[derive(Copy, Clone, PartialEq, Eq)]
pub struct Left(pub u16);

impl From<Left> for String {
    fn from(this: Left) -> String {
        let mut buf = [0u8; 20];
        ["\x1B[", this.0.numtoa_str(10, &mut buf), "D"].concat()
    }
}

impl fmt::Display for Left {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "\x1B[{}D", self.0)
    }
}

/// Move cursor right.
#[derive(Copy, Clone, PartialEq, Eq)]
pub struct Right(pub u16);

impl From<Right> for String {
    fn from(this: Right) -> String {
        let mut buf = [0u8; 20];
        ["\x1B[", this.0.numtoa_str(10, &mut buf), "C"].concat()
    }
}

impl fmt::Display for Right {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "\x1B[{}C", self.0)
    }
}

/// Move cursor up.
#[derive(Copy, Clone, PartialEq, Eq)]
pub struct Up(pub u16);

impl From<Up> for String {
    fn from(this: Up) -> String {
        let mut buf = [0u8; 20];
        ["\x1B[", this.0.numtoa_str(10, &mut buf), "A"].concat()
    }
}

impl fmt::Display for Up {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "\x1B[{}A", self.0)
    }
}

/// Move cursor down.
#[derive(Copy, Clone, PartialEq, Eq)]
pub struct Down(pub u16);

impl From<Down> for String {
    fn from(this: Down) -> String {
        let mut buf = [0u8; 20];
        ["\x1B[", this.0.numtoa_str(10, &mut buf), "B"].concat()
    }
}

impl fmt::Display for Down {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "\x1B[{}B", self.0)
    }
}

/// Move the cursor to (x, y).
///
/// This a convience wrapper.
pub fn goto(x: u16, y: u16) -> io::Result<()> {
    let mut conout = conout_r()?.lock();
    write!(conout, "{}", Goto(x, y))?;
    conout.flush()?;
    Ok(())
}

/// Return the current cursor position.
pub fn cursor_pos() -> io::Result<(u16, u16)> {
    let delimiter = b'R';

    {
        let mut conout = conout_r()?.lock();
        // Where is the cursor?
        // Use `ESC [ 6 n`.
        write!(conout, "\x1B[6n")?;
        conout.flush()?;
    }

    let mut conin = conin_r()?.lock();
    let mut buf: [u8; 1] = [0];
    let mut read_chars = Vec::new();

    let timeout = Duration::from_millis(CONTROL_SEQUENCE_TIMEOUT);
    let now = Instant::now();
    while buf[0] != delimiter && now.elapsed() < timeout {
        match conin.read_timeout(&mut buf, Some(timeout - now.elapsed())) {
            Ok(1) => {
                read_chars.push(buf[0]);
            }
            Ok(0) => {
                return Err(io::Error::new(
                    io::ErrorKind::UnexpectedEof,
                    "Unexpected EOF.",
                ));
            }
            Ok(_) => {}
            Err(err) if err.kind() == io::ErrorKind::WouldBlock => {}
            Err(err) => return Err(err),
        }
    }

    if read_chars.pop().unwrap_or(b'\0') == delimiter && !read_chars.is_empty() {
        // The answer will look like `ESC [ Cy ; Cx R`.
        // The pop in the if removes and verifies the delimiter
        if let Ok(read_str) = String::from_utf8(read_chars) {
            if let Some(beg) = read_str.rfind('[') {
                let coords: String = read_str.chars().skip(beg + 1).collect();
                let mut nums = coords.split(';');
                if let (Some(cy), Some(cx)) = (nums.next(), nums.next()) {
                    if let (Ok(cy), Ok(cx)) = (cy.parse::<u16>(), cx.parse::<u16>()) {
                        return Ok((cx, cy));
                    }
                }
            }
        }
        return Err(Error::new(
            ErrorKind::Other,
            "Failed to parse coords from chars read from console.",
        ));
    }
    Err(Error::new(
        ErrorKind::Other,
        "Cursor position detection timed out.",
    ))
}

/// Hide the cursor for the lifetime of this struct.
/// It will hide the cursor on creation with from() and show it back on drop().
pub struct HideCursor<W: ConsoleWrite> {
    /// The output target.
    output: W,
}

impl<W: ConsoleWrite> HideCursor<W> {
    /// Create a hide cursor wrapper struct for the provided output and hides the cursor.
    pub fn from(mut output: W) -> Self {
        write!(output, "{}", Hide).expect("hide the cursor");
        HideCursor { output }
    }
}

impl<W: ConsoleWrite> Drop for HideCursor<W> {
    fn drop(&mut self) {
        write!(self, "{}", Show).expect("show the cursor");
    }
}

impl<W: ConsoleWrite> ops::Deref for HideCursor<W> {
    type Target = W;

    fn deref(&self) -> &W {
        &self.output
    }
}

impl<W: ConsoleWrite> ops::DerefMut for HideCursor<W> {
    fn deref_mut(&mut self) -> &mut W {
        &mut self.output
    }
}

impl<W: ConsoleWrite> Write for HideCursor<W> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.output.write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.output.flush()
    }
}

impl<W: ConsoleWrite> ConsoleWrite for HideCursor<W> {
    fn set_raw_mode(&mut self, mode: bool) -> io::Result<bool> {
        self.output.set_raw_mode(mode)
    }

    fn is_raw_mode(&self) -> bool {
        self.output.is_raw_mode()
    }
}

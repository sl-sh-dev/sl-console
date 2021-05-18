use crate::sys::attr::{get_terminal_attr, set_terminal_attr, virt_terminal_attr};
use std::io::{self, Read, Result, Stdin, Stdout, Write};
use std::os::windows::io::AsRawHandle;

/// Is this stream a TTY?
pub fn is_tty<T: AsRawHandle>(stream: &T) -> bool {
    // @MAYBE Jezza - 17 Dec. 2018: Is this the correct implementation?
    // I just check against this program's stdin or stdout handle, and if they're the same, then the given
    // handle must be a tty for something... I guess...
    let raw = stream.as_raw_handle();
    raw == io::stdin().as_raw_handle() || raw == io::stdout().as_raw_handle()
}

/// Get the TTY device.
///
/// This allows for getting stdio representing _only_ the TTY, and not other streams.
pub fn get_tty() -> io::Result<impl Read + Write> {
    let stdin = io::stdin();
    let stdout = io::stdout();
    Ok(TerminalHandle { stdin, stdout })
}

/// Make sure the windows console will handle terminal escape codes.
///
/// If you are writing a windows app (or a crossplatform app) and you are NOT
/// using raw mode then you need to call this to make sure the console will
/// handle escape codes.  This is a noop on other platforms so is safe to call.
pub fn set_virtual_terminal() -> io::Result<()> {
    let mut ios = get_terminal_attr()?;
    virt_terminal_attr(&mut ios);
    set_terminal_attr(&ios)?;
    Ok(())
}

struct TerminalHandle {
    stdin: Stdin,
    stdout: Stdout,
}

impl Read for TerminalHandle {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        self.stdin.read(buf)
    }
}

impl Write for TerminalHandle {
    fn write(&mut self, buf: &[u8]) -> Result<usize> {
        self.stdout.write(buf)
    }

    fn flush(&mut self) -> Result<()> {
        self.stdout.flush()
    }
}

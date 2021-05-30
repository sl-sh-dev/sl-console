use std::io;
use std::os::windows::io::AsRawHandle;

/// Is this stream a TTY?
pub fn is_tty<T: AsRawHandle>(stream: &T) -> bool {
    // @MAYBE Jezza - 17 Dec. 2018: Is this the correct implementation?
    // I just check against this program's stdin or stdout handle, and if they're the same, then the given
    // handle must be a tty for something... I guess...
    let raw = stream.as_raw_handle();
    raw == io::stdin().as_raw_handle() || raw == io::stdout().as_raw_handle()
}

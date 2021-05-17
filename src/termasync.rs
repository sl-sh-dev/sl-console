//! Support async reading of the tty/console.

#[cfg(unix)]
use std::fs::{File, OpenOptions};
use std::io::{self, Read};
#[cfg(unix)]
use std::os::unix::fs::OpenOptionsExt;
#[cfg(not(unix))]
use std::sync::mpsc;
#[cfg(not(unix))]
use std::thread;
//use libc::{self, c_int, suseconds_t, timeval};

#[cfg(not(unix))]
use crate::sys::tty::get_tty;

/// Construct an asynchronous handle to the TTY standard input, with a delimiter byte.
///
/// This has the same advantages as async_stdin(), but also allows specifying a delimiter byte. The
/// reader will stop reading after consuming the delimiter byte.
#[cfg(not(unix))]
pub fn async_stdin_until(delimiter: u8) -> io::Result<AsyncReader> {
    let tty = get_tty()?;
    let (send, recv) = mpsc::channel();

    thread::spawn(move || {
        for i in tty.bytes() {
            match i {
                Ok(byte) => {
                    let end_of_stream = byte == delimiter;
                    let send_error = send.send(Ok(byte)).is_err();

                    if end_of_stream || send_error {
                        return;
                    }
                }
                Err(_) => {
                    return;
                }
            }
        }
    });

    Ok(AsyncReader { recv })
}

/// Construct an asynchronous handle to the TTY standard input, with a delimiter byte.
///
/// This version use non-blocking IO not a thread so is the same as async_stdin.
#[cfg(unix)]
pub fn async_stdin_until(_delimiter: u8) -> io::Result<AsyncReader> {
    async_stdin()
}

/// Construct an asynchronous handle to the TTY standard input.
///
/// This allows you to read from standard input _without blocking_ the current thread.
/// Specifically, it works by firing up another thread to handle the event stream, which will then
/// be buffered in a mpsc queue, which will eventually be read by the current thread.
///
/// This will not read the piped standard input, but rather read from the TTY device, since reading
/// asyncronized from piped input would rarely make sense. In other words, if you pipe standard
/// output from another process, it won't be reflected in the stream returned by this function, as
/// this represents the TTY device, and not the piped standard input.
#[cfg(not(unix))]
pub fn async_stdin() -> io::Result<AsyncReader> {
    let tty = get_tty()?;
    let (send, recv) = mpsc::channel();
    thread::spawn(move || {
        for i in tty.bytes() {
            if send.send(i).is_err() {
                return;
            }
        }
    });

    Ok(AsyncReader { recv })
}

/// Construct an asynchronous handle to the TTY standard input.
///
/// This allows you to read from standard input _without blocking_ the current thread.
/// Specifically, it works by opening up the tty device non-blocking.
///
/// This will not read the piped standard input, but rather read from the TTY device, since reading
/// asyncronized from piped input would rarely make sense. In other words, if you pipe standard
/// output from another process, it won't be reflected in the stream returned by this function, as
/// this represents the TTY device, and not the piped standard input.
#[cfg(unix)]
pub fn async_stdin() -> io::Result<AsyncReader> {
    let tty = OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NONBLOCK)
        .open("/dev/tty")
        .unwrap();
    Ok(AsyncReader { tty })
}

/// An asynchronous reader.
///
/// This acts as any other stream, with the exception that reading from it won't block. Instead,
/// the buffer will only be partially updated based on how much the internal buffer holds.
pub struct AsyncReader {
    /// The underlying mpsc receiver.
    #[cfg(not(unix))]
    recv: mpsc::Receiver<io::Result<u8>>,
    #[cfg(unix)]
    tty: File,
}

impl Read for AsyncReader {
    /// Read from the byte stream.
    ///
    /// This will never block, but try to drain the event queue until empty. If the total number of
    /// bytes written is lower than the buffer's length, the event queue is empty or that the event
    /// stream halted.
    #[cfg(not(unix))]
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
                Err(err) if err == mpsc::TryRecvError::Empty && total == 0 => {
                    return Err(io::Error::new(io::ErrorKind::WouldBlock, ""))
                }
                Err(_) => break,
            }
        }

        Ok(total)
    }

    #[cfg(unix)]
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.tty.read(buf)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use std::io::Read;

    #[test]
    fn test_async_stdin() {
        let stdin = async_stdin().unwrap();
        stdin.bytes().next();
    }
}

//! Support async reading of the tty/console.

use std::cell::RefCell;
use std::io::{self, Read};
use std::rc::Rc;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use crate::sys::tty::get_tty;

/// Construct an asynchronous handle to the TTY standard input, with a delimiter byte.
///
/// This has the same advantages as async_stdin(), but also allows specifying a delimiter byte. The
/// reader will stop reading after consuming the delimiter byte.
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
    let recv = Rc::new(RefCell::new(recv));
    let next = Rc::new(RefCell::new(None));

    Ok(AsyncReader { recv, next })
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
    let recv = Rc::new(RefCell::new(recv));
    let next = Rc::new(RefCell::new(None));

    Ok(AsyncReader { recv, next })
}

/// An asynchronous reader.
///
/// This acts as any other stream, with the exception that reading from it won't block. Instead,
/// the buffer will only be partially updated based on how much the internal buffer holds.
pub struct AsyncReader {
    /// The underlying mpsc receiver.
    recv: Rc<RefCell<mpsc::Receiver<io::Result<u8>>>>,
    next: Rc<RefCell<Option<io::Result<u8>>>>,
}

/// A blocker for an asynchronous reader.
///
/// This is useful when you need to block waiting on new data withoug a spin
/// loop or sleeps.
pub struct AsyncBlocker {
    recv: Rc<RefCell<mpsc::Receiver<io::Result<u8>>>>,
    next: Rc<RefCell<Option<io::Result<u8>>>>,
}

impl AsyncBlocker {
    /// Block until more data is ready.
    ///
    /// Assume this can be interupted.
    pub fn block(&mut self) {
        if let Ok(v) = self.recv.borrow().recv() {
            self.next.borrow_mut().replace(v);
        }
    }

    /// Block until more data is ready with a timeout.
    ///
    /// Assume this can be interupted.
    /// Returns true if the block timed out vs more data was ready.
    pub fn block_timeout(&mut self, timeout: Duration) -> bool {
        if let Ok(v) = self.recv.borrow().recv_timeout(timeout) {
            self.next.borrow_mut().replace(v);
            false
        } else {
            true
        }
    }
}

impl AsyncReader {
    /// Return a blocker struct.
    ///
    /// This can be used to block or block with a timeout on the AsyncReader.
    pub fn blocker(&mut self) -> AsyncBlocker {
        AsyncBlocker {
            recv: self.recv.clone(),
            next: self.next.clone(),
        }
    }
}

impl Read for AsyncReader {
    /// Read from the byte stream.
    ///
    /// This will never block, but try to drain the event queue until empty. If the total number of
    /// bytes written is lower than the buffer's length, the event queue is empty or that the event
    /// stream halted.
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let mut total = 0;

        loop {
            if total >= buf.len() {
                break;
            }
            if let Some(v) = self.next.borrow_mut().take() {
                match v {
                    Ok(b) => {
                        buf[total] = b;
                        total += 1;
                    }
                    Err(e) => return Err(e),
                }
            }

            match self.recv.borrow().try_recv() {
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

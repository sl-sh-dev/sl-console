//! Support async reading of the tty/console.

use std::io::{self, Read};
use std::sync::{Mutex, MutexGuard};
use std::thread;
use std::time::Duration;

use crossbeam_channel::*;
use lazy_static::lazy_static;

use crate::sys::tty::get_tty;

type Internals = Receiver<io::Result<u8>>;

fn setup_tty() -> Internals {
    if let Ok(tty) = get_tty() {
        let (send, recv) = unbounded();
        thread::spawn(move || {
            for i in tty.bytes() {
                if send.send(i).is_err() {
                    return;
                }
            }
        });
        recv
    } else {
        panic!("No tty!");
    }
}

lazy_static! {
    static ref INTERNAL_TTY: Mutex<Internals> = Mutex::new(setup_tty());
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
pub fn async_stdin<'a>() -> io::Result<AsyncReader<'a>> {
    Ok(AsyncReader {
        recv: INTERNAL_TTY.lock().unwrap(),
    })
}

/// An asynchronous reader.
///
/// This acts as any other stream, with the exception that reading from it won't block. Instead,
/// the buffer will only be partially updated based on how much the internal buffer holds.
pub struct AsyncReader<'a> {
    /// The underlying receiver.
    recv: MutexGuard<'a, Receiver<io::Result<u8>>>,
}

/// A blocker for an asynchronous reader.
///
/// This is useful when you need to block waiting on new data withoug a spin
/// loop or sleeps.
pub struct AsyncBlocker<'a> {
    recv: &'a MutexGuard<'a, Receiver<io::Result<u8>>>,
}

impl<'a> AsyncBlocker<'a> {
    /// Block until more data is ready.
    ///
    /// Assume this can be interupted.
    pub fn block(&mut self) {
        let recv_d = self.recv;
        let mut sel = Select::new();
        sel.recv(&recv_d);
        sel.ready();
    }

    /// Block until more data is ready with a timeout.
    ///
    /// Assume this can be interupted.
    /// Returns true if the block timed out vs more data was ready.
    pub fn block_timeout(&mut self, timeout: Duration) -> bool {
        let recv_d = self.recv;
        let mut sel = Select::new();
        sel.recv(&recv_d);
        sel.ready_timeout(timeout).is_err()
    }
}

impl<'a> AsyncReader<'a> {
    /// Return a blocker struct.
    ///
    /// This can be used to block or block with a timeout on the AsyncReader.
    pub fn blocker(&mut self) -> AsyncBlocker<'_> {
        AsyncBlocker { recv: &self.recv }
    }
}

impl<'a> Read for AsyncReader<'a> {
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

            match self.recv.try_recv() {
                Ok(Ok(b)) => {
                    buf[total] = b;
                    total += 1;
                }
                Ok(Err(e)) => return Err(e),
                Err(err) if err == TryRecvError::Empty && total == 0 => {
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

//! Support async reading of the tty/console.

use std::io::{self, Read, Stdin, Stdout, Write};
use std::sync::{Mutex, MutexGuard};
use std::thread;
use std::time::Duration;

use crossbeam_channel::*;
use lazy_static::lazy_static;

use crate::sys::tty::{get_tty, set_virtual_terminal};

type Internals = Receiver<io::Result<u8>>;

fn setup_tty() -> Internals {
    //let stdin = io::stdin();
    set_virtual_terminal();
    //let stdout = io::stdout();
    //if let Ok(tty) = get_tty() {
    let (send, recv) = unbounded();
    thread::spawn(move || {
        for i in io::stdin().bytes() {
            if send.send(i).is_err() {
                return;
            }
        }
    });
    recv
    //} else {
    //    panic!("No tty!");
    //}
}

lazy_static! {
    // Provide a protected singleton for the tty.  There is only one so try to
    // enforce that to avoid a myriad of issues.
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
pub fn console<'a>() -> io::Result<Console<'a>> {
    Ok(Console {
        recv: INTERNAL_TTY.lock().unwrap(),
        leftover: None,
        blocking: true,
    })
}

/// An asynchronous reader.
///
/// This acts as any other stream, with the exception that reading from it won't block. Instead,
/// the buffer will only be partially updated based on how much the internal buffer holds.
pub struct Console<'a> {
    /// The underlying receiver.
    recv: MutexGuard<'a, Receiver<io::Result<u8>>>,
    pub(crate) leftover: Option<u8>,
    pub(crate) blocking: bool,
}

impl<'a> Console<'a> {
    /// Return when more data is avialable.
    ///
    /// Calls to a get_* function should return a value now.
    /// Assume this can be interupted.
    pub fn poll(&mut self) {
        let mut sel = Select::new();
        sel.recv(&self.recv);
        sel.ready();
    }

    /// Return more data is ready or the timeout is reached.
    ///
    /// Assume this can be interupted.
    /// Returns true if the more data was ready, false if timed out.
    pub fn poll_timeout(&mut self, timeout: Duration) -> bool {
        let mut sel = Select::new();
        sel.recv(&self.recv);
        sel.ready_timeout(timeout).is_ok()
    }
}

impl<'a> Read for Console<'a> {
    /// Read from the byte stream.
    ///
    /// This will never block, but try to drain the event queue until empty. If the total number of
    /// bytes written is lower than the buffer's length, the event queue is empty or that the event
    /// stream halted.
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        if self.blocking {
            self.poll();
        }
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

impl<'a> Write for Console<'a> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        //self.tty.write(buf)
        io::stdout().write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        //self.tty.flush()
        io::stdout().flush()
    }
}

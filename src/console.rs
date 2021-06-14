//! Support access to the console.
//!
//! The input and output are split and are accessed in a way very similiar to
//! stdin/stdout with a similiar interface.  The console will be attached to
//! /dev/tty on unix and CONIN$/CONOUT$ on Windows.  This means it bypasses
//! stdin/out, if they were not redirected then they should be the same but
//! even if redirected conin()/conout() will attach to the tty or console
//! directly.
//!
//! The con_init() function should be called once (it is safe to call multiple
//! times) and if it returns an error then no tty/console is available.  If
//! con_init() fails then calls to conin()/conout() will panic.  It is safe to
//! call conin_r()/conout_r() but you will have to deal with the error and
//! conin()/conout() will always work if con_init() was successful.

use std::cell::RefCell;
use std::io::{self, Read, Write};
use std::time::Duration;

use lazy_static::lazy_static;
use parking_lot::*;

use crate::event::{Event, Key};
use crate::input::event_and_raw;
use crate::sys::console::*;

fn make_tty_in() -> io::Result<ReentrantMutex<RefCell<ConsoleIn>>> {
    let syscon = open_syscon_in()?;
    Ok(ReentrantMutex::new(RefCell::new(ConsoleIn {
        syscon,
        leftover: None,
        blocking: true,
    })))
}

fn make_tty_out() -> io::Result<ReentrantMutex<RefCell<ConsoleOut>>> {
    let syscon = open_syscon_out()?;
    Ok(ReentrantMutex::new(RefCell::new(ConsoleOut {
        syscon,
        raw_mode: false,
    })))
}

lazy_static! {
    // Provide a protected singletons for the console.  There is only one so
    // try to enforce that to avoid a myriad of issues (split into in and out).
    static ref CONSOLE_IN: io::Result<ReentrantMutex<RefCell<ConsoleIn>>> = make_tty_in();
    static ref CONSOLE_OUT: io::Result<ReentrantMutex<RefCell<ConsoleOut>>> = make_tty_out();
}

/// Initialize the console lib.
///
/// This will make sure that conin()/conout() will not panic.  It is safe
/// to call multiple times and should always be called before conin()/conout()
/// for the first time.  Do NOT call conin()/conout() if it returns an error,
/// they will panic if the console is in an error state (note they should always
/// work if coninit() returns Ok).  It is safe to call conin_r()/conout_r()
/// even if coninit() is not used- they return a result so will not panic.
pub fn con_init() -> io::Result<()> {
    if let Err(err) = &*CONSOLE_IN {
        return Err(io::Error::new(err.kind(), err));
    }
    if let Err(err) = &*CONSOLE_OUT {
        return Err(io::Error::new(err.kind(), err));
    }
    Ok(())
}

/// Lock and return read side of the tty/console for the application.
///
/// This provides a Read object that is connected to /dev/tty (unix) or
/// the console (windows).  This will not read the piped standard input, but
/// rather read from the TTY or console device, since reading asyncronized
/// from piped input would rarely make sense. In other words, if you pipe
/// standard output from another process, it won't be reflected in the stream
/// returned by this function, as this represents the TTY/console device, and
/// not the piped standard input.  This version returns an Error if the console
/// was not setup properly and coninit() is optional with it.
pub fn conin_r() -> io::Result<Conin> {
    match &*CONSOLE_IN {
        Ok(conin) => Ok(Conin { inner: conin }),
        Err(err) => Err(io::Error::new(err.kind(), err)),
    }
}

/// Lock and return write side of the tty/console for the application.
///
/// This provides a Write object that is connected to /dev/tty (unix) or
/// the console (windows).  This will not write to standard output (if it is
/// not the tty/console), but rather write to the TTY or console device.
/// In other words, if you pipe standard output to another process things you
/// write to conout() will not go into the pipe but will go to the terminal.
/// This version returns an Error if the console was not setup properly and
/// coninit() is optional with it.
pub fn conout_r() -> io::Result<Conout> {
    match &*CONSOLE_OUT {
        Ok(conout) => Ok(Conout { inner: conout }),
        Err(err) => Err(io::Error::new(err.kind(), err)),
    }
}

/// Lock and return read side of the tty/console for the application.
///
/// This provides a Read object that is connected to /dev/tty (unix) or
/// the console (windows).  This will not read the piped standard input, but
/// rather read from the TTY or console device, since reading asyncronized
/// from piped input would rarely make sense. In other words, if you pipe
/// standard output from another process, it won't be reflected in the stream
/// returned by this function, as this represents the TTY/console device, and
/// not the piped standard input.  This will always return the the locked
/// input console, will panic if it does not exit.  Always call coninit() once
/// and do not call conin() if it returns an error.
pub fn conin() -> Conin {
    match &*CONSOLE_IN {
        Ok(conin) => Conin { inner: conin },
        Err(err) => {
            eprintln!("Called conin() when no input console exists!");
            eprintln!("Did you call coninit() first and check for an error?");
            panic!("conin() failed: {}", err);
        }
    }
}

/// Lock and return write side of the tty/console for the application.
///
/// This provides a Write object that is connected to /dev/tty (unix) or
/// the console (windows).  This will not write to standard output (if it is
/// not the tty/console), but rather write to the TTY or console device.
/// In other words, if you pipe standard output to another process things you
/// write to conout() will not go into the pipe but will go to the terminal.
/// This will always return the the locked output console, will panic if it
/// does not exit.  Always call coninit() once and do not call conout() if it
/// returns an error.
pub fn conout() -> Conout {
    match &*CONSOLE_OUT {
        Ok(conout) => Conout { inner: conout },
        Err(err) => {
            eprintln!("Called conout() when no output console exists!");
            eprintln!("Did you call coninit() first and check for an error?");
            panic!("conout() failed: {}", err);
        }
    }
}

/// RAII guard for entering raw mode, will restore previous mode when dropped.
///
/// The returned guard might fail to restore the raw mode when the
/// console is being used on multiple threads.  If it can not get the
/// locks on conin and conout on drop it will silently do nothing.  Use
/// raw_mode_on/off directly when this might be an issue.
pub struct RawModeGuard {
    old_raw: bool,
}

impl Drop for RawModeGuard {
    fn drop(&mut self) {
        // Ignore error in drop.
        // Also ignore if we can not get a lock on either conin or conout.
        if let Some(mut conout) = conout().try_lock() {
            if self.old_raw {
                if conout.raw_mode_on().is_err() {}
            } else if conout.raw_mode_off().is_err() {
            }
        }
    }
}

/// Console output trait.
pub trait ConsoleWrite: Write {
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

    /// Switch to raw mode and return a RAII guard to switch to previous mode
    /// when scope ends.
    ///
    /// This call needs to also lock the conin (conout will have been locked
    /// already).  If it can not lock conin it will return an error of kind
    /// WouldBlock.
    /// The returned guard might fail to restore the raw mode when the
    /// console is being used on multiple threads.  If it can not get the
    /// locks on conin and conout on drop it will silently do nothing.  Use
    /// raw_mode_on/off directly when this might be an issue.
    fn raw_mode_guard(&mut self) -> io::Result<RawModeGuard>;

    /// True if in raw mode.
    fn is_raw_mode(&self) -> bool;
}

/// Console input trait.
pub trait ConsoleRead: Read {
    /// Set whether the console is blocking or non-blocking.
    ///
    /// Default is blocking.  If non blocking then the get_* functions can
    /// return WouldBlock errors if no data is ready.  The poll functions
    /// will work whether in blocking or non blocking mode.
    fn set_blocking(&mut self, blocking: bool);

    /// Is this console blocking or non-blocking?
    fn is_blocking(&self) -> bool;

    /// Get the next input event from the tty and the bytes that define it.
    ///
    /// If the tty is non-blocking then can return a WouldBlock error.
    fn get_event_and_raw(&mut self) -> io::Result<(Event, Vec<u8>)>;

    /// Get the next input event from the tty.
    ///
    /// If the tty is non-blocking then can return a WouldBlock error.
    fn get_event(&mut self) -> io::Result<Event>;

    /// Get the next key event from the tty.
    ///
    /// This will skip over non-key events (they will be lost).
    /// If the tty is non-blocking then can return a WouldBlock error.
    fn get_key(&mut self) -> io::Result<Key>;

    /// Return when more data is avialable.
    ///
    /// Calls to a get_* function should return a value now.
    /// Assume this can be interupted.
    fn poll(&mut self);

    /// Return more data is ready or the timeout is reached.
    ///
    /// Assume this can be interupted.
    /// Returns true if the more data was ready, false if timed out.
    fn poll_timeout(&mut self, timeout: Duration) -> bool;
}

/// Represents the input side of the tty/console terminal.
///
/// This is a singleton that aquires a lock to access the console (similiar to
/// Stdin).  It should be used to access the tty/terminal to avoid conflicts
/// and other issues.
pub struct Conin {
    inner: &'static ReentrantMutex<RefCell<ConsoleIn>>,
}

impl Conin {
    /// Locks the input console and returns a guard.
    ///
    /// Lock is released when the guard is dropped.
    pub fn lock<'a>(&self) -> ConsoleInLock<'a> {
        ConsoleInLock {
            inner: self.inner.lock(),
        }
    }

    /// Tries to lock the input console and returns Some(guard) if it could or
    /// None if it could not.  If the lock is already held by another thread
    /// then it will return None.  Underlying lock is a ReentrantMutex from
    /// parking lot so will be fine to use on the same thread.
    ///
    /// Lock is released when the guard is dropped.
    pub fn try_lock<'a>(&self) -> Option<ConsoleInLock<'a>> {
        self.inner.try_lock().map(|inner| ConsoleInLock { inner })
    }
}

impl ConsoleRead for Conin {
    fn set_blocking(&mut self, blocking: bool) {
        self.lock().set_blocking(blocking);
    }

    fn is_blocking(&self) -> bool {
        self.lock().is_blocking()
    }

    fn get_event_and_raw(&mut self) -> io::Result<(Event, Vec<u8>)> {
        self.lock().get_event_and_raw()
    }

    fn get_event(&mut self) -> io::Result<Event> {
        self.lock().get_event()
    }

    fn get_key(&mut self) -> io::Result<Key> {
        self.lock().get_key()
    }

    fn poll(&mut self) {
        self.lock().poll();
    }

    fn poll_timeout(&mut self, timeout: Duration) -> bool {
        self.lock().poll_timeout(timeout)
    }
}

impl Read for Conin {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.lock().read(buf)
    }
}

/// Represents the output side of the tty/console terminal.
///
/// This is a singleton that aquires a lock to access the console (similiar to
/// Stdin).  It should be used to access the tty/terminal to avoid conflicts
/// and other issues.
pub struct Conout {
    inner: &'static ReentrantMutex<RefCell<ConsoleOut>>,
}

impl Conout {
    /// Locks the output console and returns a guard.
    ///
    /// Lock is released when the guard is dropped.
    pub fn lock<'a>(&self) -> ConsoleOutLock<'a> {
        ConsoleOutLock {
            inner: self.inner.lock(),
        }
    }

    /// Tries to lock the output console and returns Some(guard) if it could or
    /// None if it could not.  If the lock is already held by another thread
    /// then it will return None.  Underlying lock is a ReentrantMutex from
    /// parking lot so will be fine to use on the same thread.
    ///
    /// Lock is released when the guard is dropped.
    pub fn try_lock<'a>(&self) -> Option<ConsoleOutLock<'a>> {
        self.inner.try_lock().map(|inner| ConsoleOutLock { inner })
    }
}

impl ConsoleWrite for Conout {
    fn raw_mode_off(&mut self) -> io::Result<()> {
        self.lock().raw_mode_off()
    }

    fn raw_mode_on(&mut self) -> io::Result<()> {
        self.lock().raw_mode_on()
    }

    fn raw_mode_guard(&mut self) -> io::Result<RawModeGuard> {
        self.lock().raw_mode_guard()
    }

    fn is_raw_mode(&self) -> bool {
        self.lock().is_raw_mode()
    }
}

impl Write for Conout {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.lock().write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.lock().flush()
    }
}

/// Represents the input side of the tty/console terminal.
///
/// This is a singleton that aquires a lock to access the console (similiar to
/// Stdin).  It should be used to access the tty/terminal to avoid conflicts
/// and other issues.
pub struct ConsoleIn {
    syscon: SysConsoleIn,
    leftover: Option<u8>,
    blocking: bool,
}

/// A locked console input device.
pub struct ConsoleInLock<'a> {
    inner: ReentrantMutexGuard<'a, RefCell<ConsoleIn>>,
}

/// Represents the output side of the tty/console terminal.
///
/// This is a singleton that aquires a lock to access the console (similiar to
/// Stdin).  It should be used to access the tty/terminal to avoid conflicts
/// and other issues.
pub struct ConsoleOut {
    syscon: SysConsoleOut,
    raw_mode: bool,
}

/// A locked console output device.
pub struct ConsoleOutLock<'a> {
    inner: ReentrantMutexGuard<'a, RefCell<ConsoleOut>>,
}

impl ConsoleRead for ConsoleIn {
    fn set_blocking(&mut self, blocking: bool) {
        self.blocking = blocking;
    }

    fn is_blocking(&self) -> bool {
        self.blocking
    }

    fn get_event_and_raw(&mut self) -> io::Result<(Event, Vec<u8>)> {
        let mut leftover = self.leftover.take();
        if let Some(er) = event_and_raw(self, &mut leftover) {
            er
        } else {
            Err(io::Error::new(
                io::ErrorKind::WouldBlock,
                "Event stream would block",
            ))
        }
    }

    fn get_event(&mut self) -> io::Result<Event> {
        match self.get_event_and_raw() {
            Ok((event, _raw)) => Ok(event),
            Err(err) => Err(err),
        }
    }

    fn get_key(&mut self) -> io::Result<Key> {
        loop {
            match self.get_event() {
                Ok(Event::Key(k)) => return Ok(k),
                Ok(_) => continue,
                Err(e) => return Err(e),
            }
        }
    }

    fn poll(&mut self) {
        self.syscon.poll();
    }

    fn poll_timeout(&mut self, timeout: Duration) -> bool {
        self.syscon.poll_timeout(timeout)
    }
}

impl Read for ConsoleIn {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        if self.blocking {
            self.syscon.read_block(buf)
        } else {
            self.syscon.read(buf)
        }
    }
}

impl<'a> ConsoleRead for ConsoleInLock<'a> {
    fn set_blocking(&mut self, blocking: bool) {
        self.inner.borrow_mut().blocking = blocking;
    }

    fn is_blocking(&self) -> bool {
        self.inner.borrow().blocking
    }

    fn get_event_and_raw(&mut self) -> io::Result<(Event, Vec<u8>)> {
        self.inner.borrow_mut().get_event_and_raw()
    }

    fn get_event(&mut self) -> io::Result<Event> {
        self.inner.borrow_mut().get_event()
    }

    fn get_key(&mut self) -> io::Result<Key> {
        self.inner.borrow_mut().get_key()
    }

    fn poll(&mut self) {
        self.inner.borrow_mut().poll();
    }

    fn poll_timeout(&mut self, timeout: Duration) -> bool {
        self.inner.borrow_mut().poll_timeout(timeout)
    }
}

impl<'a> Read for ConsoleInLock<'a> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.inner.borrow_mut().read(buf)
    }
}

impl ConsoleWrite for ConsoleOut {
    fn raw_mode_off(&mut self) -> io::Result<()> {
        if self.raw_mode {
            if let Some(conin) = conin_r()?.try_lock() {
                self.syscon.suspend_raw_mode(&conin.inner.borrow().syscon)?;
                self.raw_mode = false;
                Ok(())
            } else {
                Err(io::Error::new(
                    io::ErrorKind::WouldBlock,
                    "Conin is already locked.",
                ))
            }
        } else {
            Ok(())
        }
    }

    fn raw_mode_on(&mut self) -> io::Result<()> {
        if !self.raw_mode {
            if let Some(conin) = conin_r()?.try_lock() {
                self.syscon
                    .activate_raw_mode(&conin.inner.borrow().syscon)?;
                self.raw_mode = true;
                Ok(())
            } else {
                Err(io::Error::new(
                    io::ErrorKind::WouldBlock,
                    "Conin is already locked.",
                ))
            }
        } else {
            Ok(())
        }
    }

    fn raw_mode_guard(&mut self) -> io::Result<RawModeGuard> {
        let old_raw = self.raw_mode;
        self.raw_mode_on()?;
        Ok(RawModeGuard { old_raw })
    }

    fn is_raw_mode(&self) -> bool {
        self.raw_mode
    }
}

impl Write for ConsoleOut {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.syscon.write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.syscon.flush()
    }
}

impl<'a> ConsoleWrite for ConsoleOutLock<'a> {
    fn raw_mode_off(&mut self) -> io::Result<()> {
        self.inner.borrow_mut().raw_mode_off()
    }

    fn raw_mode_on(&mut self) -> io::Result<()> {
        self.inner.borrow_mut().raw_mode_on()
    }

    fn raw_mode_guard(&mut self) -> io::Result<RawModeGuard> {
        self.inner.borrow_mut().raw_mode_guard()
    }

    fn is_raw_mode(&self) -> bool {
        self.inner.borrow().is_raw_mode()
    }
}

impl<'a> Write for ConsoleOutLock<'a> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.inner.borrow_mut().write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.inner.borrow_mut().flush()
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use std::io::Read;

    #[test]
    fn test_async_stdin() {
        let mut tty = conin_r().unwrap();
        tty.set_blocking(false);
        tty.bytes().next();
    }
}

#[cfg(unix)]
mod unix_impl {
    use super::*;
    use std::os::unix::io::{AsRawFd, RawFd};

    impl AsRawFd for Conin {
        fn as_raw_fd(&self) -> RawFd {
            self.lock().as_raw_fd()
        }
    }
    impl AsRawFd for Conout {
        fn as_raw_fd(&self) -> RawFd {
            self.lock().as_raw_fd()
        }
    }

    impl AsRawFd for ConsoleIn {
        fn as_raw_fd(&self) -> RawFd {
            self.syscon.as_raw_fd()
        }
    }
    impl AsRawFd for ConsoleOut {
        fn as_raw_fd(&self) -> RawFd {
            self.syscon.as_raw_fd()
        }
    }

    impl<'a> AsRawFd for ConsoleInLock<'a> {
        fn as_raw_fd(&self) -> RawFd {
            self.inner.borrow_mut().as_raw_fd()
        }
    }
    impl<'a> AsRawFd for ConsoleOutLock<'a> {
        fn as_raw_fd(&self) -> RawFd {
            self.inner.borrow_mut().as_raw_fd()
        }
    }
}

#[cfg(windows)]
mod windows_impl {
    use super::*;
    use std::os::windows::io::{AsRawHandle, RawHandle};

    impl AsRawHandle for Conin {
        fn as_raw_handle(&self) -> RawHandle {
            self.lock().as_raw_handle()
        }
    }
    impl AsRawHandle for Conout {
        fn as_raw_handle(&self) -> RawHandle {
            self.lock().as_raw_handle()
        }
    }

    impl AsRawHandle for ConsoleIn {
        fn as_raw_handle(&self) -> RawHandle {
            self.syscon.as_raw_handle()
        }
    }
    impl AsRawHandle for ConsoleOut {
        fn as_raw_handle(&self) -> RawHandle {
            self.syscon.as_raw_handle()
        }
    }

    impl<'a> AsRawHandle for ConsoleInLock<'a> {
        fn as_raw_handle(&self) -> RawHandle {
            self.inner.borrow_mut().as_raw_handle()
        }
    }
    impl<'a> AsRawHandle for ConsoleOutLock<'a> {
        fn as_raw_handle(&self) -> RawHandle {
            self.inner.borrow_mut().as_raw_handle()
        }
    }
}

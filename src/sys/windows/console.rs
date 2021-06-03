//! Support async reading of the tty/console.

use std::ffi::OsStr;
use std::fs::File;
use std::io::{self, Read, Write};
use std::iter::once;
use std::os::windows::ffi::OsStrExt;
use std::os::windows::io::AsRawHandle;
use std::os::windows::io::FromRawHandle;
use std::ptr::null_mut;
use std::thread;
use std::time::Duration;

use crossbeam_channel::*;
use winapi::ctypes::c_void;
use winapi::um::consoleapi::{GetConsoleMode, SetConsoleMode};
use winapi::um::fileapi::CreateFile2;
use winapi::um::wincon::{
    ENABLE_ECHO_INPUT, ENABLE_LINE_INPUT, ENABLE_PROCESSED_INPUT, ENABLE_VIRTUAL_TERMINAL_INPUT,
    ENABLE_VIRTUAL_TERMINAL_PROCESSING,
};

use crate::sys::attr::{handle_result, result};

const RAW_MODE_IN_MASK: u32 = ENABLE_LINE_INPUT | ENABLE_ECHO_INPUT | ENABLE_PROCESSED_INPUT;

/// Open and return the read side of a console.
pub fn open_syscon_in() -> io::Result<SysConsoleIn> {
    let console_in_name: Vec<u16> = OsStr::new("CONIN$").encode_wide().chain(once(0)).collect();
    let handle = handle_result(unsafe {
        CreateFile2(
            console_in_name.as_ptr(),
            winapi::um::winnt::GENERIC_READ | winapi::um::winnt::GENERIC_WRITE,
            winapi::um::winnt::FILE_SHARE_WRITE,
            winapi::um::fileapi::OPEN_EXISTING,
            null_mut(),
        )
    })?;

    let mut console_mode = 0;
    result(unsafe { GetConsoleMode(handle as *mut c_void, &mut console_mode) })?;
    //console_mode &= !RAW_MODE_MASK;
    console_mode |= ENABLE_VIRTUAL_TERMINAL_INPUT;
    let normal_mode = console_mode;
    result(unsafe { SetConsoleMode(handle as *mut c_void, console_mode) })?;
    let tty = unsafe { File::from_raw_handle(handle as *mut std::ffi::c_void) };

    let (send, recv) = unbounded();
    thread::spawn(move || {
        for i in tty.bytes() {
            if send.send(i).is_err() {
                return;
            }
        }
    });
    let handle = handle as usize;
    Ok(SysConsoleIn {
        recv,
        normal_mode,
        handle,
    })
}

/// Open and return the write side of a console.
pub fn open_syscon_out() -> io::Result<SysConsoleOut> {
    //let tty = OpenOptions::new().write(true).read(true).open("CONOUT$")?;
    let console_in_name: Vec<u16> = OsStr::new("CONOUT$").encode_wide().chain(once(0)).collect();
    let handle = handle_result(unsafe {
        CreateFile2(
            console_in_name.as_ptr(),
            winapi::um::winnt::GENERIC_READ | winapi::um::winnt::GENERIC_WRITE,
            winapi::um::winnt::FILE_SHARE_READ,
            winapi::um::fileapi::OPEN_EXISTING,
            null_mut(),
        )
    })?;

    let mut console_mode = 0;
    result(unsafe { GetConsoleMode(handle as *mut c_void, &mut console_mode) })?;
    console_mode |= ENABLE_VIRTUAL_TERMINAL_PROCESSING;
    result(unsafe { SetConsoleMode(handle as *mut c_void, console_mode) })?;
    let tty = unsafe { File::from_raw_handle(handle as *mut std::ffi::c_void) };

    Ok(SysConsoleOut {
        tty,
        normal_mode: console_mode,
    })
}

/// Represents system specific part of a tty/console output.
pub struct SysConsoleOut {
    tty: File,
    /// The "normal" console attribs for out.
    normal_mode: u32,
}

/// An asynchronous reader.
///
/// This acts as any other stream, with the exception that reading from it won't block. Instead,
/// the buffer will only be partially updated based on how much the internal buffer holds.
pub struct SysConsoleIn {
    /// The underlying receiver.
    recv: Receiver<io::Result<u8>>,
    /// The "normal" console attribs for in.
    normal_mode: u32,
    /// Handle to CONIN$
    handle: usize,
}

impl SysConsoleOut {
    /// Switch to original mode
    pub fn suspend_raw_mode(&self, conin: &SysConsoleIn) -> io::Result<()> {
        let handle = self.tty.as_raw_handle() as *mut c_void;
        result(unsafe { SetConsoleMode(handle, self.normal_mode) })?;
        let handle = conin.handle as *mut c_void;
        result(unsafe { SetConsoleMode(handle, conin.normal_mode) })?;
        Ok(())
    }

    /// Switch to raw mode
    pub fn activate_raw_mode(&mut self, conin: &SysConsoleIn) -> io::Result<()> {
        //let handle = self.tty.as_raw_handle() as *mut c_void;
        //result(unsafe { SetConsoleMode(handle, self.normal_mode) })?;
        let handle = conin.handle as *mut c_void;
        let raw_mode = conin.normal_mode & !RAW_MODE_IN_MASK;
        result(unsafe { SetConsoleMode(handle, raw_mode) })?;
        Ok(())
    }
}

impl SysConsoleIn {
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

    /// Read from the byte stream.
    ///
    /// This version blocks, the read from the Read trait does not.
    pub fn read_block(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let mut total = 0;

        if buf.is_empty() {
            return Ok(0);
        }
        let mut last_byte;
        match self.recv.recv() {
            Ok(Ok(b)) => {
                last_byte = b;
                buf[total] = b;
                total += 1;
            }
            Ok(Err(e)) => return Err(e),
            Err(err) => return Err(io::Error::new(io::ErrorKind::Other, err)),
        }
        loop {
            if total >= buf.len() {
                break;
            }

            match self.recv.try_recv() {
                Ok(Ok(b)) => {
                    last_byte = b;
                    buf[total] = b;
                    total += 1;
                }
                Ok(Err(e)) => return Err(e),
                Err(TryRecvError::Empty) if last_byte == b'\x1B' => {
                    // If last byte was an escape small pause for the next byte
                    // in case it is an escape code...
                    self.poll_timeout(Duration::from_millis(3));
                    last_byte = b'\0';
                }
                Err(_) => break,
            }
        }

        Ok(total)
    }
}

impl Read for SysConsoleIn {
    /// Read from the byte stream.
    ///
    /// This read is non-blocking.
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let mut total = 0;
        let mut last_byte = b'\0';

        loop {
            if total >= buf.len() {
                break;
            }

            match self.recv.try_recv() {
                Ok(Ok(b)) => {
                    last_byte = b;
                    buf[total] = b;
                    total += 1;
                }
                Ok(Err(e)) => return Err(e),
                Err(TryRecvError::Empty) if last_byte == b'\x1B' => {
                    // If last byte was an escape small pause for the next byte
                    // in case it is an escape code...
                    self.poll_timeout(Duration::from_millis(3));
                    last_byte = b'\0';
                }
                Err(TryRecvError::Empty) if total == 0 => {
                    return Err(io::Error::new(io::ErrorKind::WouldBlock, ""))
                }
                Err(_) => break,
            }
        }

        Ok(total)
    }
}

impl Write for SysConsoleOut {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.tty.write(buf)
        //io::stdout().write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.tty.flush()
        //io::stdout().flush()
    }
}

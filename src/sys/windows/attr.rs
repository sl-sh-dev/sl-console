use std::io;

use super::Termios;
use winapi::ctypes::c_void;
use winapi::shared::minwindef::BOOL;
use winapi::um::consoleapi::{GetConsoleMode, SetConsoleMode};
use winapi::um::handleapi::INVALID_HANDLE_VALUE;
use winapi::um::processenv::GetStdHandle;
use winapi::um::winbase::{STD_INPUT_HANDLE, STD_OUTPUT_HANDLE};
use winapi::um::wincon::{
    ENABLE_ECHO_INPUT, ENABLE_LINE_INPUT, ENABLE_PROCESSED_INPUT, ENABLE_VIRTUAL_TERMINAL_INPUT,
    ENABLE_VIRTUAL_TERMINAL_PROCESSING,
};
use winapi::um::winnt::HANDLE;

const RAW_MODE_MASK: u32 = ENABLE_LINE_INPUT | ENABLE_ECHO_INPUT | ENABLE_PROCESSED_INPUT;

/// Get the result of a call to WinAPI as an [`io::Result`].
#[inline]
pub(crate) fn result(return_value: BOOL) -> io::Result<()> {
    if return_value != 0 {
        Ok(())
    } else {
        Err(io::Error::last_os_error())
    }
}

/// Get the result of a call to WinAPI that returns a handle or `INVALID_HANDLE_VALUE`.
#[inline]
pub(crate) fn handle_result(return_value: HANDLE) -> io::Result<HANDLE> {
    if return_value != INVALID_HANDLE_VALUE {
        Ok(return_value)
    } else {
        Err(io::Error::last_os_error())
    }
}

pub fn get_terminal_attr() -> io::Result<Termios> {
    let handle = handle_result(unsafe { GetStdHandle(STD_INPUT_HANDLE) })?;
    let mut in_mode = 0;
    result(unsafe { GetConsoleMode(handle as *mut c_void, &mut in_mode) })?;

    let handle = handle_result(unsafe { GetStdHandle(STD_OUTPUT_HANDLE) })?;
    let mut out_mode = 0;
    result(unsafe { GetConsoleMode(handle as *mut c_void, &mut out_mode) })?;

    Ok(Termios(in_mode, out_mode))
}

pub fn set_terminal_attr(termios: &Termios) -> io::Result<()> {
    let handle = handle_result(unsafe { GetStdHandle(STD_INPUT_HANDLE) })?;
    result(unsafe { SetConsoleMode(handle as *mut c_void, termios.0) })?;

    let handle = handle_result(unsafe { GetStdHandle(STD_OUTPUT_HANDLE) })?;
    result(unsafe { SetConsoleMode(handle as *mut c_void, termios.1) })?;

    Ok(())
}

pub fn raw_terminal_attr(termios: &mut Termios) {
    termios.0 &= !RAW_MODE_MASK;
    termios.0 |= ENABLE_VIRTUAL_TERMINAL_INPUT;

    termios.1 |= ENABLE_VIRTUAL_TERMINAL_PROCESSING;
}

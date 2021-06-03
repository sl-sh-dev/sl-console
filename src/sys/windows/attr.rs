use std::io;

use winapi::shared::minwindef::BOOL;
use winapi::um::handleapi::INVALID_HANDLE_VALUE;
//use winapi::um::processenv::GetStdHandle;
//use winapi::um::winbase::{STD_INPUT_HANDLE, STD_OUTPUT_HANDLE};
use winapi::um::winnt::HANDLE;

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

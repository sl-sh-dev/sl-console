use std::ffi::OsStr;
use std::io;
use std::iter::once;
use std::mem::zeroed;
use std::os::windows::ffi::OsStrExt;
use std::ptr::null_mut;

use winapi::um::fileapi::CreateFile2;
use winapi::um::wincon::GetConsoleScreenBufferInfo;

use crate::sys::attr::{handle_result, result};

/// Get the size of the terminal.
pub fn terminal_size() -> io::Result<(u16, u16)> {
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
    let mut csbi = unsafe { zeroed() };
    result(unsafe { GetConsoleScreenBufferInfo(handle, &mut csbi) })?;
    let width = csbi.srWindow.Right - csbi.srWindow.Left;
    let height = csbi.srWindow.Bottom - csbi.srWindow.Top;
    // windows starts counting at 0, unix at 1, add one to replicated unix behaviour.
    Ok(((width + 1) as u16, (height + 1) as u16))
}

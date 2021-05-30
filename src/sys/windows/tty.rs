use std::os::windows::io::AsRawHandle;

use winapi::ctypes::c_void;
use winapi::um::consoleapi::GetConsoleMode;

/// Is this stream a TTY?
pub fn is_tty<T: AsRawHandle>(stream: &T) -> bool {
    let mut console_mode = 0;
    let handle = stream.as_raw_handle();
    // This should fail if not a handle to CONIN$ or CONOUT$...
    let rc = unsafe { GetConsoleMode(handle as *mut c_void, &mut console_mode) };
    rc != 0
}

use std::io;

use super::crossterm_winapi::{ConsoleMode, Handle};
use super::Termios;

// These are copied from the MSDocs.
// Yes, technically, not the best, but Windows won't change these for obvious reasons.
// We could link in winapi explicitly, as crossterm_winapi is already doing that, but
// I feel it just adds a bit too much cruft, when we can just do this.
//
// https://docs.microsoft.com/en-us/windows/console/setconsolemode#parameters
const ENABLE_PROCESSED_INPUT: u32 = 0x0001;
const ENABLE_LINE_INPUT: u32 = 0x0002;
const ENABLE_ECHO_INPUT: u32 = 0x0004;
const ENABLE_VIRTUAL_TERMINAL_INPUT: u32 = 0x0200;
const ENABLE_VIRTUAL_TERMINAL_PROCESSING: u32 = 0x0004;
const RAW_MODE_MASK: u32 = ENABLE_LINE_INPUT | ENABLE_ECHO_INPUT | ENABLE_PROCESSED_INPUT;

pub fn get_terminal_attr() -> io::Result<Termios> {
    let console_mode = ConsoleMode::from(Handle::current_in_handle()?);

    let in_mode = console_mode.mode()?;

    let console_mode = ConsoleMode::new()?;

    let out_mode = console_mode.mode()?;
    Ok(Termios(in_mode, out_mode))
}

pub fn set_terminal_attr(termios: &Termios) -> io::Result<()> {
    let console_mode = ConsoleMode::from(Handle::current_in_handle()?);

    console_mode.set_mode(termios.0)?;

    let console_mode = ConsoleMode::new()?;

    console_mode.set_mode(termios.1)?;

    Ok(())
}

pub fn raw_terminal_attr(termios: &mut Termios) {
    termios.0 &= !RAW_MODE_MASK;
    termios.0 |= ENABLE_VIRTUAL_TERMINAL_INPUT;

    termios.1 |= ENABLE_VIRTUAL_TERMINAL_PROCESSING;
}

pub fn virt_terminal_attr(termios: &mut Termios) {
    termios.0 |= ENABLE_VIRTUAL_TERMINAL_INPUT;
    termios.1 |= ENABLE_VIRTUAL_TERMINAL_PROCESSING;
}

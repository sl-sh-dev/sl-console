//! Mouse and key events.

use std::io::{Error, ErrorKind};
use std::{io, str};

/// An event reported by the terminal.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Event {
    /// A key press.
    Key(Key),
    /// A mouse button press, release or wheel use at specific coordinates.
    Mouse(MouseEvent),
    /// An event that cannot currently be evaluated.
    Unsupported(Vec<u8>),
}

/// A mouse related event.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum MouseEvent {
    /// A mouse button was pressed.
    ///
    /// The coordinates are one-based.
    Press(MouseButton, u16, u16),
    /// A mouse button was released.
    ///
    /// The coordinates are one-based.
    Release(u16, u16),
    /// A mouse button is held over the given coordinates.
    ///
    /// The coordinates are one-based.
    Hold(u16, u16),
}

/// A mouse button.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum MouseButton {
    /// The left mouse button.
    Left,
    /// The right mouse button.
    Right,
    /// The middle mouse button.
    Middle,
    /// Mouse wheel is going up.
    ///
    /// This event is typically only used with Mouse::Press.
    WheelUp,
    /// Mouse wheel is going down.
    ///
    /// This event is typically only used with Mouse::Press.
    WheelDown,
}

/// A key.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Key {
    /// Backspace.
    Backspace,
    /// Left arrow.
    Left,
    /// Right arrow.
    Right,
    /// Up arrow.
    Up,
    /// Down arrow.
    Down,
    /// Home key.
    Home,
    /// End key.
    End,
    /// Page Up key.
    PageUp,
    /// Page Down key.
    PageDown,
    /// Backward Tab key.
    BackTab,
    /// Delete key.
    Delete,
    /// Insert key.
    Insert,
    /// Function keys.
    ///
    /// Only function keys 1 through 12 are supported.
    F(u8),
    /// Normal character.
    Char(char),
    /// Alt modified character.
    Alt(char),
    /// Ctrl modified character.
    ///
    /// Note that certain keys may not be modifiable with `ctrl`, due to limitations of terminals.
    Ctrl(char),
    /// Null byte.
    Null,
    /// Esc key.
    Esc,
}

/// Parse an Event from `item` and possibly subsequent bytes through `iter`.
pub fn parse_event<I>(item: u8, iter: &mut I) -> Result<Event, Error>
where
    I: Iterator<Item = Result<u8, Error>>,
{
    let error = Error::new(ErrorKind::Other, "Could not parse an event");
    match item {
        b'\x1B' => {
            // This is an escape character, leading a control sequence.
            Ok(match iter.next() {
                Some(Ok(b'O')) => {
                    match iter.next() {
                        // F1-F4
                        Some(Ok(val @ b'P'..=b'S')) => Event::Key(Key::F(1 + val - b'P')),
                        _ => return Err(error),
                    }
                }
                Some(Ok(b'[')) => {
                    // This is a CSI sequence.
                    parse_csi(iter)?
                }
                Some(Ok(c)) => {
                    let ch = parse_utf8_char(c, iter)?;
                    Event::Key(Key::Alt(ch))
                }
                Some(Err(_)) | None => return Err(error),
            })
        }
        b'\n' | b'\r' => Ok(Event::Key(Key::Char('\n'))),
        b'\t' => Ok(Event::Key(Key::Char('\t'))),
        b'\x7F' => Ok(Event::Key(Key::Backspace)),
        c @ b'\x01'..=b'\x1A' => Ok(Event::Key(Key::Ctrl((c as u8 - 0x1 + b'a') as char))),
        c @ b'\x1C'..=b'\x1F' => Ok(Event::Key(Key::Ctrl((c as u8 - 0x1C + b'4') as char))),
        b'\0' => Ok(Event::Key(Key::Null)),
        c => Ok({
            let ch = parse_utf8_char(c, iter)?;
            Event::Key(Key::Char(ch))
        }),
    }
}

fn next_char<I>(iter: &mut I) -> Option<u8>
where
    I: Iterator<Item = Result<u8, Error>>,
{
    if let Some(Ok(next)) = iter.next() {
        return Some(next);
    }
    None
}

/// Parses a CSI sequence, just after reading ^[
///
/// Returns None if an unrecognized sequence is found.
// TODO make this return Result<Event, io::Error>
fn parse_csi<I>(iter: &mut I) -> Result<Event, io::Error>
where
    I: Iterator<Item = Result<u8, Error>>,
{
    Ok(match iter.next() {
        Some(Ok(b'[')) => match iter.next() {
            Some(Ok(val @ b'A'..=b'E')) => Event::Key(Key::F(1 + val - b'A')),
            _ => {
                return Err(Error::new(
                    ErrorKind::Other,
                    "Failed to parse csi code b'['",
                ))
            }
        },
        Some(Ok(b'D')) => Event::Key(Key::Left),
        Some(Ok(b'C')) => Event::Key(Key::Right),
        Some(Ok(b'A')) => Event::Key(Key::Up),
        Some(Ok(b'B')) => Event::Key(Key::Down),
        Some(Ok(b'H')) => Event::Key(Key::Home),
        Some(Ok(b'F')) => Event::Key(Key::End),
        Some(Ok(b'Z')) => Event::Key(Key::BackTab),
        Some(Ok(b'M')) => {
            // X10 emulation mouse encoding: ESC [ CB Cx Cy (6 characters only).
            if let (Some(cb), Some(cx), Some(cy)) =
                (next_char(iter), next_char(iter), next_char(iter))
            {
                let cb = cb as i8 - 32;
                let cx = cx.saturating_sub(32) as u16;
                let cy = cy.saturating_sub(32) as u16;
                Event::Mouse(match cb & 0b11 {
                    0 => {
                        if cb & 0x40 != 0 {
                            MouseEvent::Press(MouseButton::WheelUp, cx, cy)
                        } else {
                            MouseEvent::Press(MouseButton::Left, cx, cy)
                        }
                    }
                    1 => {
                        if cb & 0x40 != 0 {
                            MouseEvent::Press(MouseButton::WheelDown, cx, cy)
                        } else {
                            MouseEvent::Press(MouseButton::Middle, cx, cy)
                        }
                    }
                    2 => MouseEvent::Press(MouseButton::Right, cx, cy),
                    3 => MouseEvent::Release(cx, cy),
                    _ => {
                        return Err(Error::new(
                            ErrorKind::Other,
                            "Failed to parse csi code b'M'",
                        ))
                    }
                })
            } else {
                return Err(Error::new(
                    ErrorKind::Other,
                    "Failed to parse X10 emulation mouse encoding. Expected: ESC [ CB Cx Cy (6 characters only)."
                ));
            }
        }
        Some(Ok(b'<')) => {
            // xterm mouse encoding:
            // ESC [ < Cb ; Cx ; Cy (;) (M or m)
            let mut buf = Vec::new();
            if let Some(mut c) = next_char(iter) {
                while !matches!(c, b'm' | b'M') {
                    buf.push(c);
                    if let Some(new_c) = next_char(iter) {
                        c = new_c
                    }
                }
                if !buf.is_empty() {
                    let str_buf = String::from_utf8(buf).unwrap();
                    let nums = &mut str_buf.split(';');

                    let cb = nums.next().unwrap().parse::<u16>().unwrap();
                    let cx = nums.next().unwrap().parse::<u16>().unwrap();
                    let cy = nums.next().unwrap().parse::<u16>().unwrap();

                    let event = match cb {
                        0..=2 | 64..=65 => {
                            let button = match cb {
                                0 => MouseButton::Left,
                                1 => MouseButton::Middle,
                                2 => MouseButton::Right,
                                64 => MouseButton::WheelUp,
                                65 => MouseButton::WheelDown,
                                _ => unreachable!(),
                            };
                            match c {
                                b'M' => MouseEvent::Press(button, cx, cy),
                                b'm' => MouseEvent::Release(cx, cy),
                                _ => {
                                    return Err(Error::new(
                                        ErrorKind::Other,
                                        "Failed to parse csi code b'M' or b'm' after b'<'",
                                    ))
                                }
                            }
                        }
                        32 => MouseEvent::Hold(cx, cy),
                        3 => MouseEvent::Release(cx, cy),
                        _ => {
                            return Err(Error::new(
                                ErrorKind::Other,
                                "Failed to parse csi code as mouse event",
                            ))
                        }
                    };

                    return Ok(Event::Mouse(event));
                }
            }
            return Err(Error::new(
                ErrorKind::Other,
                "Failed to parse xterm mouse encoding. Expected: ESC [ < Cb ; Cx ; Cy (;) (M or m)",
            ));
        }
        Some(Ok(c @ b'0'..=b'9')) => {
            // Numbered escape code.
            let mut buf = vec![c];
            let mut c = iter.next().unwrap().unwrap();
            // The final byte of a CSI sequence can be in the range 64-126, so
            // let's keep reading anything else.
            while !(64..=126).contains(&c) {
                buf.push(c);
                c = iter.next().unwrap().unwrap();
            }

            match c {
                // rxvt mouse encoding:
                // ESC [ Cb ; Cx ; Cy ; M
                b'M' => {
                    let str_buf = String::from_utf8(buf).unwrap();

                    let nums: Vec<u16> = str_buf.split(';').map(|n| n.parse().unwrap()).collect();

                    let cb = nums[0];
                    let cx = nums[1];
                    let cy = nums[2];

                    let event = match cb {
                        32 => MouseEvent::Press(MouseButton::Left, cx, cy),
                        33 => MouseEvent::Press(MouseButton::Middle, cx, cy),
                        34 => MouseEvent::Press(MouseButton::Right, cx, cy),
                        35 => MouseEvent::Release(cx, cy),
                        64 => MouseEvent::Hold(cx, cy),
                        96 | 97 => MouseEvent::Press(MouseButton::WheelUp, cx, cy),
                        _ => {
                            return Err(Error::new(
                                ErrorKind::Other,
                                "Failed to parse csi code b'0'..=b'9' as mouse event",
                            ))
                        }
                    };

                    Event::Mouse(event)
                }
                // Special key code.
                b'~' => {
                    let str_buf = String::from_utf8(buf).unwrap();

                    // This CSI sequence can be a list of semicolon-separated
                    // numbers.
                    let nums: Vec<u8> = str_buf.split(';').map(|n| n.parse().unwrap()).collect();

                    if nums.is_empty() {
                        return Err(Error::new(
                            ErrorKind::Other,
                            "Failed to parse csi b'~', buffer is empty",
                        ));
                    }

                    // TODO: handle multiple values for key modifiers (ex: values
                    // [3, 2] means Shift+Delete)
                    if nums.len() > 1 {
                        return Err(Error::new(
                            ErrorKind::Other,
                            "Multiple values for csi_code b'~' key modifiers not supported",
                        ));
                    }

                    match nums[0] {
                        1 | 7 => Event::Key(Key::Home),
                        2 => Event::Key(Key::Insert),
                        3 => Event::Key(Key::Delete),
                        4 | 8 => Event::Key(Key::End),
                        5 => Event::Key(Key::PageUp),
                        6 => Event::Key(Key::PageDown),
                        v @ 11..=15 => Event::Key(Key::F(v - 10)),
                        v @ 17..=21 => Event::Key(Key::F(v - 11)),
                        v @ 23..=24 => Event::Key(Key::F(v - 12)),
                        _ => {
                            return Err(Error::new(
                                ErrorKind::Other,
                                "Failed to parse csi code b'~', unexpected value",
                            ))
                        }
                    }
                }
                _ => {
                    return Err(Error::new(
                        ErrorKind::Other,
                        "Failed to parse csi code b'~'",
                    ))
                }
            }
        }
        _ => {
            return Err(Error::new(
                ErrorKind::Other,
                "Failed to parse input as csi code, unexpected value",
            ))
        }
    })
}

/// Parse `c` as either a single byte ASCII char or a variable size UTF-8 char.
fn parse_utf8_char<I>(c: u8, iter: &mut I) -> Result<char, Error>
where
    I: Iterator<Item = Result<u8, Error>>,
{
    let error = Err(Error::new(
        ErrorKind::Other,
        "Input character is not valid UTF-8",
    ));
    if c.is_ascii() {
        Ok(c as char)
    } else {
        let bytes = &mut Vec::new();
        bytes.push(c);

        loop {
            match iter.next() {
                Some(Ok(next)) => {
                    bytes.push(next);
                    if let Ok(st) = str::from_utf8(bytes) {
                        return Ok(st.chars().next().unwrap());
                    }
                    if bytes.len() >= 4 {
                        return error;
                    }
                }
                _ => return error,
            }
        }
    }
}

#[cfg(test)]
#[test]
fn test_parse_utf8() {
    let st = "abcéŷ¤£€ù%323";
    let ref mut bytes = st.bytes().map(|x| Ok(x));
    let chars = st.chars();
    for c in chars {
        let b = bytes.next().unwrap().unwrap();
        assert!(c == parse_utf8_char(b, bytes).unwrap());
    }
}

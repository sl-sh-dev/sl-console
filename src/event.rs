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

#[derive(Debug, Copy, Clone, Hash, PartialEq, Eq)]
/// Struct representing a Key composed of a KeyCode and KeyMod
/// Note that certain KeyCode + KeyMod combinations are not
/// supported:
/// - KeyMod::AltCtrlShift and KeyCode::Char(<any>), e.g. KeyCode::Char('h')
/// will report Key { code: Char('h), mods: KeyMod::CtrlAlt despite the
/// shift key being pressed due to limitations in the terminal API
/// - KeyMod::AltCtrl and KeyCode::Char(0..=9) are not supported.
/// - KeyMod::CtrlShift and KeyCode::Char(0..=9) are not supported
/// - Any modifier keys and Backspace/Tab are not supported, save
/// KeyMod::Shift and KeyCode::Tab which is KeyCode::BackTab
/// - Shift+Insert is not supported
/// - Some terminals do not support modifier keys and certain
/// non alpha-numeric keys
pub struct Key {
    /// any key that could be pressed
    pub code: KeyCode,
    /// any key modifier ctrl + alt + shift (excluding capital letters w/ shift) that could be
    /// pressed.
    pub mods: Option<KeyMod>,
}

impl Key {
    /// Creates a new Key with no KeyMod
    ///
    /// Returns Key
    pub fn new(key: KeyCode) -> Self {
        Self {
            code: key,
            mods: None,
        }
    }

    /// Creates a new Key with provided KeyMod
    ///
    /// Returns Key
    pub fn new_mod(key: KeyCode, mods: KeyMod) -> Self {
        Self {
            code: key,
            mods: Some(mods),
        }
    }
}

/// A key.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum KeyCode {
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
    /// Null byte.
    Null,
    /// Esc key.
    Esc,
}

/// Key combinations for keys besides Alt(char) and Ctrl(char) in
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum KeyMod {
    /// Alt modifier key
    Alt,
    /// Ctrl modifier key
    /// Note that certain keys may not be modifiable with `ctrl`, due to limitations of terminals.
    Ctrl,
    /// Shift modifier key
    /// Note that capital letters do not note the `shift` modifier.
    Shift,
    /// Alt + Ctrl
    AltCtrl,
    /// Alt + Shift
    AltShift,
    /// Ctrl + Shift
    CtrlShift,
    /// Alt + Ctrl + Shift
    AltCtrlShift,
}

/// Parse an Event from `item` and possibly subsequent bytes through `iter`.
pub fn parse_event<I>(item: u8, iter: &mut I) -> io::Result<Event>
where
    I: Iterator<Item = io::Result<u8>>,
{
    fn inner_parse_event<I>(item: u8, iter: &mut I) -> io::Result<Event>
    where
        I: Iterator<Item = io::Result<u8>>,
    {
        match item {
            b'\x1B' | b'\x9B' => {
                // This is an escape character, leading a control sequence.
                Ok(match iter.next() {
                    Some(Ok(b'O')) => {
                        match iter.next() {
                            // F1-F4
                            Some(Ok(val @ b'P'..=b'S')) => {
                                Event::Key(Key::new(KeyCode::F(1 + val - b'P')))
                            }
                            Some(Ok(b'5')) => match iter.next() {
                                Some(Ok(val @ b'P'..=b'S')) => Event::Key(Key::new_mod(
                                    KeyCode::F(1 + val - b'P'),
                                    KeyMod::Ctrl,
                                )),
                                _ => {
                                    return Err(Error::new(
                                        ErrorKind::Other,
                                        "Unknown escape code after ESC O 5",
                                    ))
                                }
                            },
                            _ => {
                                return Err(Error::new(
                                    ErrorKind::Other,
                                    "Unknown escape code after ESC O",
                                ))
                            }
                        }
                    }
                    Some(Ok(b'[')) => {
                        // This is a CSI sequence.
                        parse_csi(iter)?
                    }
                    Some(Ok(c)) => {
                        let ch = parse_utf8_char(c, iter)?;
                        match c {
                            b'\x01'..=b'\x1A' => Event::Key(Key::new_mod(
                                KeyCode::Char((ch as u8 - 0x1 + b'a') as char),
                                KeyMod::AltCtrl,
                            )),
                            _ => {
                                Event::Key(Key::new_mod(parse_libtickit_key_codes(c), KeyMod::Alt))
                            }
                        }
                    }
                    Some(Err(_)) | None => {
                        return Err(Error::new(ErrorKind::Other, "Could not parse an event"))
                    }
                })
            }
            b'\n' | b'\r' => Ok(Event::Key(Key::new(KeyCode::Char('\n')))),
            b'\t' => Ok(Event::Key(Key::new(KeyCode::Char('\t')))),
            b'\x7F' => Ok(Event::Key(Key::new(KeyCode::Backspace))),
            c @ b'\x01'..=b'\x1A' => Ok(Event::Key(Key::new_mod(
                KeyCode::Char((c as u8 - 0x1 + b'a') as char),
                KeyMod::Ctrl,
            ))),
            c @ b'\x1C'..=b'\x1F' => Ok(Event::Key(Key::new_mod(
                KeyCode::Char((c as u8 - 0x1C + b'4') as char),
                KeyMod::Ctrl,
            ))),
            b'\0' => Ok(Event::Key(Key::new(KeyCode::Null))),
            c => Ok({
                let ch = parse_utf8_char(c, iter)?;
                Event::Key(Key::new(KeyCode::Char(ch)))
            }),
        }
    }
    let mut control_seq = vec![item];
    let result = {
        let mut iter = iter.inspect(|k| {
            if let Ok(k) = k {
                control_seq.push(*k);
            }
        });
        inner_parse_event(item, &mut iter)
    };

    match result {
        Ok(event) => Ok(event),
        Err(error) => {
            log::error!("Failed to parse event: {}", error);
            Ok(Event::Unsupported(control_seq))
        }
    }
}

fn next_char<I, T>(iter: &mut I) -> Option<T>
where
    I: Iterator<Item = Result<T, Error>>,
    T: Copy,
{
    if let Some(Ok(next)) = iter.next() {
        return Some(next);
    }
    None
}

/// Reference for parse_special_key_code, parse_other_special_key_code, and parse_key_mods
/// https://invisible-island.net/xterm/ctlseqs/ctlseqs.html
fn parse_special_key_code(code: u8) -> Option<KeyCode> {
    let code = match code {
        1 | 7 => KeyCode::Home,
        2 => KeyCode::Insert,
        3 => KeyCode::Delete,
        4 | 8 => KeyCode::End,
        5 => KeyCode::PageUp,
        6 => KeyCode::PageDown,
        v @ 11..=15 => KeyCode::F(v - 10),
        v @ 17..=21 => KeyCode::F(v - 11),
        v @ 23..=24 => KeyCode::F(v - 12),
        _ => return None,
    };
    Some(code)
}

fn parse_other_special_key_code(code: u8) -> Option<KeyCode> {
    let code = match code {
        b'D' => KeyCode::Left,
        b'C' => KeyCode::Right,
        b'A' => KeyCode::Up,
        b'B' => KeyCode::Down,
        b'H' => KeyCode::Home,
        b'F' => KeyCode::End,
        b'Z' => KeyCode::BackTab,
        b'P' => KeyCode::F(1),
        b'Q' => KeyCode::F(2),
        b'R' => KeyCode::F(3),
        b'S' => KeyCode::F(4),
        _ => return None,
    };
    Some(code)
}

fn parse_libtickit_key_codes(code: u8) -> KeyCode {
    match code {
        27 => KeyCode::Esc,
        127 => KeyCode::Backspace,
        code => KeyCode::Char(code as char),
    }
}

fn parse_key_mods(mods: u8) -> Option<KeyMod> {
    let mods = match mods {
        2 => KeyMod::Shift,
        3 => KeyMod::Alt,
        4 => KeyMod::AltShift,
        5 => KeyMod::Ctrl,
        6 => KeyMod::CtrlShift,
        7 => KeyMod::AltCtrl,
        8 => KeyMod::AltCtrlShift,
        _ => return None,
    };
    Some(mods)
}

/// Parses a CSI sequence, just after reading ^[
///
/// Returns Result<Event, io::Error>, Event may be unsupported.
fn parse_csi<I>(iter: &mut I) -> io::Result<Event>
where
    I: Iterator<Item = Result<u8, Error>>,
{
    Ok(match iter.next() {
        Some(Ok(b'[')) => match iter.next() {
            Some(Ok(val @ b'A'..=b'E')) => Event::Key(Key::new(KeyCode::F(1 + val - b'A'))),
            _ => return Err(Error::new(ErrorKind::Other, "Failed to parse csi code [")),
        },
        Some(Ok(b'D')) => Event::Key(Key::new(KeyCode::Left)),
        Some(Ok(b'C')) => Event::Key(Key::new(KeyCode::Right)),
        Some(Ok(b'A')) => Event::Key(Key::new(KeyCode::Up)),
        Some(Ok(b'B')) => Event::Key(Key::new(KeyCode::Down)),
        Some(Ok(b'H')) => Event::Key(Key::new(KeyCode::Home)),
        Some(Ok(b'F')) => Event::Key(Key::new(KeyCode::End)),
        Some(Ok(b'Z')) => Event::Key(Key::new(KeyCode::BackTab)),
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
                    _ => return Err(Error::new(ErrorKind::Other, "Failed to parse csi code M")),
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
                    if let Ok(str_buf) = String::from_utf8(buf) {
                        let nums = &mut str_buf.split(';');
                        if let (Some(cb), Some(cx), Some(cy)) =
                            (nums.next(), nums.next(), nums.next())
                        {
                            if let (Ok(cb), Ok(cx), Ok(cy)) =
                                (cb.parse::<u16>(), cx.parse::<u16>(), cy.parse::<u16>())
                            {
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
                                                    "Failed to parse csi code M or m after <",
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
                    }
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
            if let Some(mut c) = next_char(iter) {
                // The final byte of a CSI sequence can be in the range 64-126, so
                // let's keep reading anything else.
                while !(64..=126).contains(&c) {
                    buf.push(c);
                    if let Some(new_c) = next_char(iter) {
                        c = new_c
                    }
                }
                match c {
                    b'^' => {
                        // rxvt ctrl codes for mod + special keys:
                        // ESC [ x ^
                        if let Ok(str_buf) = String::from_utf8(buf) {
                            if let Ok(to_int) = str_buf.parse::<u8>() {
                                return if let Some(code) = parse_special_key_code(to_int) {
                                    Ok(Event::Key(Key::new_mod(code, KeyMod::Ctrl)))
                                } else {
                                    Err(Error::new(
                                        ErrorKind::Other,
                                        "Unrecognized rxvt key encoding.",
                                    ))
                                };
                            }
                        }
                        return Err(Error::new(
                            ErrorKind::Other,
                            "Failed to parse rxvt mod + special keys.",
                        ));
                    }
                    // rxvt mouse encoding:
                    // ESC [ Cb ; Cx ; Cy ; M
                    b'M' => {
                        if let Ok(str_buf) = String::from_utf8(buf) {
                            let nums = &mut str_buf.split(';');
                            if let (Some(cb), Some(cx), Some(cy)) =
                                (nums.next(), nums.next(), nums.next())
                            {
                                if let (Ok(cb), Ok(cx), Ok(cy)) =
                                    (cb.parse::<u16>(), cx.parse::<u16>(), cy.parse::<u16>())
                                {
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
                                                "Failed to parse csi code 0-9 as mouse event",
                                            ))
                                        }
                                    };
                                    return Ok(Event::Mouse(event));
                                }
                            }
                        }
                        return Err(Error::new(
                            ErrorKind::Other,
                            "Failed to parse rxvt mouse encoding. Expected: ESC [ Cb ; Cx ; Cy ; M",
                        ));
                    }
                    // Special key code.
                    b'~' => {
                        if let Ok(str_buf) = String::from_utf8(buf) {
                            // This CSI sequence can be a list of semicolon-separated
                            // numbers.
                            let mut nums: Vec<u8> = vec![];
                            for i in str_buf.split(';') {
                                if let Ok(c) = i.parse::<u8>() {
                                    nums.push(c);
                                }
                            }
                            let event = match nums.len() {
                                0 => {
                                    return Err(Error::new(
                                        ErrorKind::Other,
                                        "Failed to parse csi ~, buffer is empty",
                                    ))
                                }
                                1 => {
                                    if let Some(code) = parse_special_key_code(nums[0]) {
                                        Event::Key(Key::new(code))
                                    } else {
                                        Event::Unsupported(nums)
                                    }
                                }
                                2 => {
                                    if let Some(key_code) = parse_special_key_code(nums[0]) {
                                        if let Some(mods) = parse_key_mods(nums[1]) {
                                            Event::Key(Key::new_mod(key_code, mods))
                                        } else {
                                            Event::Unsupported(nums)
                                        }
                                    } else {
                                        Event::Unsupported(nums)
                                    }
                                }
                                _ => Event::Unsupported(nums),
                            };
                            return Ok(event);
                        }
                        return Err(Error::new(
                            ErrorKind::Other,
                            "Failed to parse csi code ~ from buffer",
                        ));
                    }
                    b'u' => {
                        // libtickit specification:
                        // http://www.leonerd.org.uk/hacks/fixterms/
                        if let Ok(str_buf) = String::from_utf8(buf) {
                            // This libtickit sequence can be a list of semicolon-separated
                            // numbers.
                            let mut nums: Vec<u8> = vec![];
                            for i in str_buf.split(';') {
                                if let Ok(c) = i.parse::<u8>() {
                                    nums.push(c);
                                }
                            }
                            let event =
                                match nums.len() {
                                    0 => return Err(Error::new(
                                        ErrorKind::Other,
                                        "Failed to parse libtickit escape code, buffer is empty",
                                    )),
                                    1 => Event::Unsupported(nums),
                                    2 => {
                                        let key_code = parse_libtickit_key_codes(nums[0]);
                                        if let Some(mods) = parse_key_mods(nums[1]) {
                                            Event::Key(Key::new_mod(key_code, mods))
                                        } else {
                                            Event::Unsupported(nums)
                                        }
                                    }
                                    _ => Event::Unsupported(nums),
                                };
                            return Ok(event);
                        } else {
                            return Err(Error::new(
                                ErrorKind::Other,
                                "Failed to parse libtickit escape code",
                            ));
                        }
                    }
                    val => {
                        if let Some(key_code) = parse_other_special_key_code(val) {
                            if let Ok(str_buf) = String::from_utf8(buf) {
                                let mut nums: Vec<u8> = vec![];
                                for i in str_buf.split(';') {
                                    if let Ok(c) = i.parse::<u8>() {
                                        nums.push(c);
                                    }
                                }
                                if nums.len() == 2 {
                                    if let Some(mods) = parse_key_mods(nums[1]) {
                                        return Ok(Event::Key(Key::new_mod(key_code, mods)));
                                    }
                                }
                                return Ok(Event::Unsupported(nums));
                            }
                        }
                        return Err(Error::new(ErrorKind::Other, "Failed to parse csi code"));
                    }
                };
            };
            return Err(Error::new(
                ErrorKind::Other,
                "Failed to parse numbered escape code",
            ));
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
fn parse_utf8_char<I>(c: u8, iter: &mut I) -> io::Result<char>
where
    I: Iterator<Item = io::Result<u8>>,
{
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
                        if let Some(c) = st.chars().next() {
                            return Ok(c);
                        }
                    }
                    if bytes.len() >= 4 {
                        return Err(Error::new(
                            ErrorKind::Other,
                            "Input character is not valid UTF-8",
                        ));
                    }
                }
                _ => {
                    return Err(Error::new(
                        ErrorKind::Other,
                        "Input character is not valid UTF-8",
                    ))
                }
            }
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use std::array::IntoIter;
    use std::collections::HashMap;
    use std::iter::FromIterator;

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

    fn test_parse_event_dynamic(item: u8, map: &mut HashMap<String, Event>) {
        for (key, val) in map.iter() {
            let mut iter = key.bytes().map(|x| Ok(x));
            assert_eq!(*val, parse_event(item, &mut iter).unwrap())
        }
    }

    fn test_parse_event(item: u8, map: &mut HashMap<&str, Event>) {
        for (key, val) in map.iter() {
            let mut iter = key.bytes().map(|x| Ok(x));
            assert_eq!(*val, parse_event(item, &mut iter).unwrap())
        }
    }

    #[test]
    fn test_parse_valid_csi_special_codes() {
        let mut map = HashMap::<_, _>::from_iter(IntoIter::new([
            ("[1~", Event::Key(Key::new(KeyCode::Home))),
            ("[7~", Event::Key(Key::new(KeyCode::Home))),
            ("[7^", Event::Key(Key::new_mod(KeyCode::Home, KeyMod::Ctrl))),
            ("[2~", Event::Key(Key::new(KeyCode::Insert))),
            ("[4~", Event::Key(Key::new(KeyCode::End))),
            ("[8~", Event::Key(Key::new(KeyCode::End))),
            ("[5~", Event::Key(Key::new(KeyCode::PageUp))),
            ("[6~", Event::Key(Key::new(KeyCode::PageDown))),
            ("[H", Event::Key(Key::new(KeyCode::Home))),
            ("[F", Event::Key(Key::new(KeyCode::End))),
            ("[D", Event::Key(Key::new(KeyCode::Left))),
            ("[Z", Event::Key(Key::new(KeyCode::BackTab))),
            (
                "[1;2F",
                Event::Key(Key::new_mod(KeyCode::End, KeyMod::Shift)),
            ),
            ("[1;3F", Event::Key(Key::new_mod(KeyCode::End, KeyMod::Alt))),
            (
                "[1;4F",
                Event::Key(Key::new_mod(KeyCode::End, KeyMod::AltShift)),
            ),
            (
                "[1;5F",
                Event::Key(Key::new_mod(KeyCode::End, KeyMod::Ctrl)),
            ),
            (
                "[1;6F",
                Event::Key(Key::new_mod(KeyCode::End, KeyMod::CtrlShift)),
            ),
            (
                "[1;7F",
                Event::Key(Key::new_mod(KeyCode::End, KeyMod::AltCtrl)),
            ),
            (
                "[1;8F",
                Event::Key(Key::new_mod(KeyCode::End, KeyMod::AltCtrlShift)),
            ),
            (
                "[1;2C",
                Event::Key(Key::new_mod(KeyCode::Right, KeyMod::Shift)),
            ),
            (
                "[1;3C",
                Event::Key(Key::new_mod(KeyCode::Right, KeyMod::Alt)),
            ),
            (
                "[1;4C",
                Event::Key(Key::new_mod(KeyCode::Right, KeyMod::AltShift)),
            ),
            (
                "[1;5C",
                Event::Key(Key::new_mod(KeyCode::Right, KeyMod::Ctrl)),
            ),
            (
                "[1;6C",
                Event::Key(Key::new_mod(KeyCode::Right, KeyMod::CtrlShift)),
            ),
            (
                "[1;7C",
                Event::Key(Key::new_mod(KeyCode::Right, KeyMod::AltCtrl)),
            ),
            (
                "[1;8C",
                Event::Key(Key::new_mod(KeyCode::Right, KeyMod::AltCtrlShift)),
            ),
            ("[C", Event::Key(Key::new(KeyCode::Right))),
            ("[A", Event::Key(Key::new(KeyCode::Up))),
            ("[B", Event::Key(Key::new(KeyCode::Down))),
            (
                "[11^",
                Event::Key(Key::new_mod(KeyCode::F(1), KeyMod::Ctrl)),
            ),
            ("[11~", Event::Key(Key::new(KeyCode::F(1)))),
            ("[12~", Event::Key(Key::new(KeyCode::F(2)))),
            ("[13~", Event::Key(Key::new(KeyCode::F(3)))),
            ("[14~", Event::Key(Key::new(KeyCode::F(4)))),
            ("[15~", Event::Key(Key::new(KeyCode::F(5)))),
            ("[17~", Event::Key(Key::new(KeyCode::F(6)))),
            ("[18~", Event::Key(Key::new(KeyCode::F(7)))),
            ("[19~", Event::Key(Key::new(KeyCode::F(8)))),
            ("[20~", Event::Key(Key::new(KeyCode::F(9)))),
            ("[21~", Event::Key(Key::new(KeyCode::F(10)))),
            ("[23~", Event::Key(Key::new(KeyCode::F(11)))),
            ("[24~", Event::Key(Key::new(KeyCode::F(12)))),
            (
                "[3;2~",
                Event::Key(Key::new_mod(KeyCode::Delete, KeyMod::Shift)),
            ),
            (
                "[3;3~",
                Event::Key(Key::new_mod(KeyCode::Delete, KeyMod::Alt)),
            ),
            (
                "[3;4~",
                Event::Key(Key::new_mod(KeyCode::Delete, KeyMod::AltShift)),
            ),
            (
                "[3;5~",
                Event::Key(Key::new_mod(KeyCode::Delete, KeyMod::Ctrl)),
            ),
            (
                "[3;6~",
                Event::Key(Key::new_mod(KeyCode::Delete, KeyMod::CtrlShift)),
            ),
            (
                "[3;7~",
                Event::Key(Key::new_mod(KeyCode::Delete, KeyMod::AltCtrl)),
            ),
            (
                "[3;8~",
                Event::Key(Key::new_mod(KeyCode::Delete, KeyMod::AltCtrlShift)),
            ),
            (
                "[5;2~",
                Event::Key(Key::new_mod(KeyCode::PageUp, KeyMod::Shift)),
            ),
            (
                "[6;2~",
                Event::Key(Key::new_mod(KeyCode::PageDown, KeyMod::Shift)),
            ),
            (
                "[15;2~",
                Event::Key(Key::new_mod(KeyCode::F(5), KeyMod::Shift)),
            ),
            (
                "[17;2~",
                Event::Key(Key::new_mod(KeyCode::F(6), KeyMod::Shift)),
            ),
            (
                "[18;2~",
                Event::Key(Key::new_mod(KeyCode::F(7), KeyMod::Shift)),
            ),
            (
                "[19;2~",
                Event::Key(Key::new_mod(KeyCode::F(8), KeyMod::Shift)),
            ),
            (
                "[20;2~",
                Event::Key(Key::new_mod(KeyCode::F(9), KeyMod::Shift)),
            ),
            (
                "[21;2~",
                Event::Key(Key::new_mod(KeyCode::F(10), KeyMod::Shift)),
            ),
            (
                "[23;2~",
                Event::Key(Key::new_mod(KeyCode::F(11), KeyMod::Shift)),
            ),
            (
                "[24;2~",
                Event::Key(Key::new_mod(KeyCode::F(12), KeyMod::Shift)),
            ),
        ]));

        let item = b'\x1B';
        test_parse_event(item, &mut map);
    }

    #[test]
    fn test_parse_x10_emulation_mouse_encoding() {
        let mut map = HashMap::<_, _>::from_iter(IntoIter::new([
            (
                "[M\x00\x00\x00",
                Event::Mouse(MouseEvent::Press(MouseButton::WheelUp, 0, 0)),
            ),
            (
                "[M\x40\x30\x32",
                Event::Mouse(MouseEvent::Press(MouseButton::Left, 16, 18)),
            ),
            (
                "[M\x01\x00\x00",
                Event::Mouse(MouseEvent::Press(MouseButton::WheelDown, 0, 0)),
            ),
            (
                "[M\x41\x29\x30",
                Event::Mouse(MouseEvent::Press(MouseButton::Middle, 9, 16)),
            ),
            (
                "[M\x02\x00\x30",
                Event::Mouse(MouseEvent::Press(MouseButton::Right, 0, 16)),
            ),
            ("[M\x03\x30\x7F", Event::Mouse(MouseEvent::Release(16, 95))),
        ]));

        let item = b'\x1B';
        test_parse_event(item, &mut map);
    }

    #[test]
    fn test_parse_rxvt_mouse_encoding() {
        let mut map = HashMap::<_, _>::from_iter(IntoIter::new([
            (
                "[32;65;8;M",
                Event::Mouse(MouseEvent::Press(MouseButton::Left, 65, 8)),
            ),
            (
                "[33;5;2;M",
                Event::Mouse(MouseEvent::Press(MouseButton::Middle, 5, 2)),
            ),
            (
                "[34;65;8;M",
                Event::Mouse(MouseEvent::Press(MouseButton::Right, 65, 8)),
            ),
            ("[35;65;8;M", Event::Mouse(MouseEvent::Release(65, 8))),
            ("[64;113;234;M", Event::Mouse(MouseEvent::Hold(113, 234))),
            (
                "[96;65;8;M",
                Event::Mouse(MouseEvent::Press(MouseButton::WheelUp, 65, 8)),
            ),
            (
                "[97;65;8;M",
                Event::Mouse(MouseEvent::Press(MouseButton::WheelUp, 65, 8)),
            ),
        ]));

        let item = b'\x1B';
        test_parse_event(item, &mut map);
    }

    #[test]
    fn test_parse_valid_csi_xterm_mouse() {
        let mut map = HashMap::<_, _>::from_iter(IntoIter::new([
            (
                "[<0;65;8;M",
                Event::Mouse(MouseEvent::Press(MouseButton::Left, 65, 8)),
            ),
            (
                "[<1;5;2;M",
                Event::Mouse(MouseEvent::Press(MouseButton::Middle, 5, 2)),
            ),
            (
                "[<2;65;8;M",
                Event::Mouse(MouseEvent::Press(MouseButton::Right, 65, 8)),
            ),
            (
                "[<64;65;8;M",
                Event::Mouse(MouseEvent::Press(MouseButton::WheelUp, 65, 8)),
            ),
            (
                "[<65;82;1;M",
                Event::Mouse(MouseEvent::Press(MouseButton::WheelDown, 82, 1)),
            ),
            ("[<3;65;8;m", Event::Mouse(MouseEvent::Release(65, 8))),
            ("[<32;113;234;m", Event::Mouse(MouseEvent::Hold(113, 234))),
        ]));

        let item = b'\x1B';
        test_parse_event(item, &mut map);
    }

    #[test]
    fn test_parse_ctrl_key_alphanumeric() {
        // a
        let mut map = HashMap::<_, _>::from_iter(IntoIter::new([(
            "",
            Event::Key(Key::new_mod(KeyCode::Char('a'), KeyMod::Ctrl)),
        )]));

        let item = b'\x01';
        test_parse_event(item, &mut map);

        // z
        let mut map = HashMap::<_, _>::from_iter(IntoIter::new([(
            "",
            Event::Key(Key::new_mod(KeyCode::Char('z'), KeyMod::Ctrl)),
        )]));

        let item = b'\x1A';
        test_parse_event(item, &mut map);

        // 4
        let mut map = HashMap::<_, _>::from_iter(IntoIter::new([(
            "",
            Event::Key(Key::new_mod(KeyCode::Char('4'), KeyMod::Ctrl)),
        )]));

        let item = b'\x1C';
        test_parse_event(item, &mut map);

        // 7
        let mut map = HashMap::<_, _>::from_iter(IntoIter::new([(
            "",
            Event::Key(Key::new_mod(KeyCode::Char('7'), KeyMod::Ctrl)),
        )]));

        let item = b'\x1F';
        test_parse_event(item, &mut map);

        // newline
        let mut map = HashMap::<_, _>::from_iter(IntoIter::new([(
            "",
            Event::Key(Key::new(KeyCode::Char('\n'))),
        )]));

        let item = b'\n';
        test_parse_event(item, &mut map);

        // carriage return
        let mut map = HashMap::<_, _>::from_iter(IntoIter::new([(
            "",
            Event::Key(Key::new(KeyCode::Char('\n'))),
        )]));

        let item = b'\r';
        test_parse_event(item, &mut map);

        // tab
        let mut map = HashMap::<_, _>::from_iter(IntoIter::new([(
            "",
            Event::Key(Key::new(KeyCode::Char('\t'))),
        )]));

        let item = b'\t';
        test_parse_event(item, &mut map);

        // backspace
        let mut map = HashMap::<_, _>::from_iter(IntoIter::new([(
            "",
            Event::Key(Key::new(KeyCode::Backspace)),
        )]));

        let item = b'\x7F';
        test_parse_event(item, &mut map);

        // null
        let mut map =
            HashMap::<_, _>::from_iter(IntoIter::new([("", Event::Key(Key::new(KeyCode::Null)))]));

        let item = b'\0';
        test_parse_event(item, &mut map);
    }

    #[test]
    fn test_parse_non_csi_escape_codes() {
        let mut map = HashMap::<_, _>::from_iter(IntoIter::new([
            ("OP", Event::Key(Key::new(KeyCode::F(1)))),
            ("OS", Event::Key(Key::new(KeyCode::F(4)))),
            (
                "O5P^",
                Event::Key(Key::new_mod(KeyCode::F(1), KeyMod::Ctrl)),
            ),
            (
                "O5Q^",
                Event::Key(Key::new_mod(KeyCode::F(2), KeyMod::Ctrl)),
            ),
            (
                "O5R^",
                Event::Key(Key::new_mod(KeyCode::F(3), KeyMod::Ctrl)),
            ),
            (
                "O5S^",
                Event::Key(Key::new_mod(KeyCode::F(4), KeyMod::Ctrl)),
            ),
            (
                "\u{1}",
                Event::Key(Key::new_mod(KeyCode::Char('a'), KeyMod::AltCtrl)),
            ),
            (
                "\u{1a}",
                Event::Key(Key::new_mod(KeyCode::Char('z'), KeyMod::AltCtrl)),
            ),
            (
                "\u{61}",
                Event::Key(Key::new_mod(KeyCode::Char('a'), KeyMod::Alt)),
            ),
            (
                "\u{7a}",
                Event::Key(Key::new_mod(KeyCode::Char('z'), KeyMod::Alt)),
            ),
        ]));
        let item = b'\x1B';
        test_parse_event(item, &mut map);
    }

    #[test]
    fn test_parse_invalid() {
        let item = b'\x1B';
        let mut iter = "[x".bytes().map(|x| Ok(x));
        assert_eq!(
            parse_event(item, &mut iter).unwrap(),
            Event::Unsupported(vec![b'\x1B', b'[', b'x']),
        )
    }

    #[test]
    fn test_parse_libtickit_ascii() {
        let csi_sequences = vec![b'\x1b', b'\x9b'];
        let mod_map = HashMap::<_, _>::from_iter(IntoIter::new([
            ("6", KeyMod::CtrlShift),
            ("8", KeyMod::AltCtrlShift),
        ]));
        let mut upper_letters = HashMap::new();
        for n in 65..91 {
            upper_letters.insert(format!("{}", n), KeyCode::Char((n as u8) as char));
        }

        for csi in csi_sequences.iter() {
            let item = csi;
            let mut map = HashMap::new();
            for (mod_str, mods) in mod_map.iter() {
                for (letter_str, code) in upper_letters.iter() {
                    let str = format!("[{};{}u", letter_str, mod_str);
                    map.insert(str, Event::Key(Key::new_mod(*code, *mods)));
                }
            }
            test_parse_event_dynamic(*item, &mut map);
        }
    }

    #[test]
    fn test_parse_libtickit_special() {
        let csi_sequences = vec![b'\x1b', b'\x9b'];
        let mod_map = HashMap::<_, _>::from_iter(IntoIter::new([
            ("2", KeyMod::Shift),
            ("3", KeyMod::Alt),
            ("4", KeyMod::AltShift),
            ("5", KeyMod::Ctrl),
            ("6", KeyMod::CtrlShift),
            ("7", KeyMod::AltCtrl),
            ("8", KeyMod::AltCtrlShift),
        ]));
        let mut special_key_codes = HashMap::new();
        special_key_codes.insert("27", KeyCode::Esc);
        special_key_codes.insert("127", KeyCode::Backspace);
        for csi in csi_sequences.iter() {
            let mut map = HashMap::new();
            let item = csi;
            for (mod_str, mods) in mod_map.iter() {
                for (letter_str, code) in special_key_codes.iter() {
                    let str = format!("[{};{}u", letter_str, mod_str);
                    map.insert(str, Event::Key(Key::new_mod(*code, *mods)));
                }
            }
            test_parse_event_dynamic(*item, &mut map);
        }
    }
}

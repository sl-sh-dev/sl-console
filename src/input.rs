//! User input

use std::io::{self, Read, Write};
use std::ops;
use std::time::Duration;

use crate::console::{ConsoleRead, ConsoleWrite};
use crate::event::{self, Event, Key, KeyCode};

/// An iterator over input events.
pub struct EventsAndRaw<R> {
    inner: R,
}

impl<R: ConsoleRead> Iterator for EventsAndRaw<R> {
    type Item = Result<(Event, Vec<u8>), io::Error>;

    fn next(&mut self) -> Option<Result<(Event, Vec<u8>), io::Error>> {
        self.inner.get_event_and_raw(None)
    }
}

/// An iterator over input keys.
pub struct Keys<R> {
    inner: R,
}

impl<R: ConsoleRead> Iterator for Keys<R> {
    type Item = Result<Key, io::Error>;

    fn next(&mut self) -> Option<Result<Key, io::Error>> {
        self.inner.get_key()
    }
}

/// An iterator over input events.
pub struct Events<R> {
    inner: R,
}

impl<R: ConsoleRead> Iterator for Events<R> {
    type Item = Result<Event, io::Error>;

    fn next(&mut self) -> Option<Result<Event, io::Error>> {
        self.inner.get_event()
    }
}

/// Get the next input event and the bytes that define it.
pub(crate) fn event_and_raw(
    source: &mut dyn Read,
    leftover: &mut Option<u8>,
) -> Option<Result<(Event, Vec<u8>), io::Error>> {
    if let Some(c) = leftover {
        // we have a leftover byte, use it
        let ch = *c;
        *leftover = None;
        return Some(parse_event(ch, &mut source.bytes()));
    }

    // Here we read two bytes at a time. We need to distinguish between single ESC key presses,
    // and escape sequences (which start with ESC or a x1B byte). The idea is that if this is
    // an escape sequence, we will read multiple bytes (the first byte being ESC) but if this
    // is a single ESC keypress, we will only read a single byte.
    let mut buf = [0u8; 2];
    let res = match source.read(&mut buf) {
        Ok(0) => return None,
        Ok(1) => match buf[0] {
            b'\x1B' => Ok((Event::Key(Key::new(KeyCode::Esc)), vec![b'\x1B'])),
            c => parse_event(c, &mut source.bytes()),
        },
        Ok(2) => {
            let option_iter = &mut Some(buf[1]).into_iter();
            let result = {
                let mut iter = option_iter.map(Ok).chain(source.bytes());
                parse_event(buf[0], &mut iter)
            };
            // If the option_iter wasn't consumed, keep the byte for later.
            *leftover = option_iter.next();
            result
        }
        Ok(_) => unreachable!(),
        Err(e) => Err(e),
    };

    Some(res)
}

fn parse_event<I>(item: u8, iter: &mut I) -> io::Result<(Event, Vec<u8>)>
where
    I: Iterator<Item = io::Result<u8>>,
{
    let mut buf = vec![item];
    let result = {
        let mut iter = iter.inspect(|byte| {
            if let Ok(byte) = *byte {
                buf.push(byte);
            }
        });
        event::parse_event(item, &mut iter)
    };
    result
        .or_else(|_| Ok(Event::Unsupported(buf.clone())))
        .map(|e| (e, buf))
}

/// Extension to `ConsoleRead` trait.
pub trait ConsoleReadExt {
    /// An iterator over input events and the raw bytes that make them.
    fn events_and_raw(self) -> EventsAndRaw<Self>
    where
        Self: Sized;

    /// An iterator over input events.
    fn events(self) -> Events<Self>
    where
        Self: Sized;

    /// An iterator over key inputs.
    fn keys(self) -> Keys<Self>
    where
        Self: Sized;

    /// Get the next input event from the console.
    /// This version will block until an event is ready.
    /// Returns None if the Console has no more data vs a read that would block.
    fn get_event(&mut self) -> Option<io::Result<Event>>;

    /// Get the next input event from the console.
    ///
    /// If no data is ready before timeout then will return a WouldBlock error.
    /// Returns None if the Console has no more data vs a read that would block.
    fn get_event_timeout(&mut self, timeout: Duration) -> Option<io::Result<Event>>;

    /// Get the next key event from the console.
    ///
    /// This will skip over non-key events (they will be lost).
    /// This version will block until an event is ready.
    /// Returns None if the Console has no more data.
    fn get_key(&mut self) -> Option<io::Result<Key>>;
}

impl<R: ConsoleRead> ConsoleReadExt for R {
    fn events_and_raw(self) -> EventsAndRaw<Self> {
        EventsAndRaw { inner: self }
    }

    fn events(self) -> Events<Self> {
        Events { inner: self }
    }

    fn keys(self) -> Keys<Self> {
        Keys { inner: self }
    }

    fn get_event(&mut self) -> Option<io::Result<Event>> {
        match self.get_event_and_raw(None) {
            Some(Ok((event, _raw))) => Some(Ok(event)),
            Some(Err(err)) => Some(Err(err)),
            None => None,
        }
    }

    fn get_event_timeout(&mut self, timeout: Duration) -> Option<io::Result<Event>> {
        match self.get_event_and_raw(Some(timeout)) {
            Some(Ok((event, _raw))) => Some(Ok(event)),
            Some(Err(err)) => Some(Err(err)),
            None => None,
        }
    }

    fn get_key(&mut self) -> Option<io::Result<Key>> {
        loop {
            match self.get_event() {
                Some(Ok(Event::Key(k))) => return Some(Ok(k)),
                Some(Ok(_)) => continue,
                Some(Err(e)) => return Some(Err(e)),
                None => return None,
            }
        }
    }
}

/// A sequence of escape codes to enable terminal mouse support.
const ENTER_MOUSE_SEQUENCE: &str = csi!("?1000h\x1b[?1002h\x1b[?1015h\x1b[?1006h");

/// A sequence of escape codes to disable terminal mouse support.
const EXIT_MOUSE_SEQUENCE: &str = csi!("?1006l\x1b[?1015l\x1b[?1002l\x1b[?1000l");

/// Extension trait for ConsoleWrite to turn mouse support on or off for the console.
pub trait ConsoleMouseExt {
    /// Turn mouse support on for the console.
    fn mouse_on(&mut self) -> io::Result<()>;

    /// Turn mouse support off for the console.
    fn mouse_off(&mut self) -> io::Result<()>;
}

impl<W: ConsoleWrite> ConsoleMouseExt for W {
    fn mouse_on(&mut self) -> io::Result<()> {
        self.write_all(ENTER_MOUSE_SEQUENCE.as_bytes())?;
        Ok(())
    }

    fn mouse_off(&mut self) -> io::Result<()> {
        self.write_all(EXIT_MOUSE_SEQUENCE.as_bytes())?;
        Ok(())
    }
}

/// A terminal with added mouse support.
///
/// This can be obtained through the `From` implementations.
/// You can use this if you want an RAII guard around terminal mouse support.
pub struct MouseTerminal<W: ConsoleWrite> {
    term: W,
}

impl<W: ConsoleWrite> From<W> for MouseTerminal<W> {
    fn from(mut from: W) -> MouseTerminal<W> {
        from.write_all(ENTER_MOUSE_SEQUENCE.as_bytes()).unwrap();

        MouseTerminal { term: from }
    }
}

impl<W: ConsoleWrite> Drop for MouseTerminal<W> {
    fn drop(&mut self) {
        self.term.write_all(EXIT_MOUSE_SEQUENCE.as_bytes()).unwrap();
    }
}

impl<W: ConsoleWrite> ops::Deref for MouseTerminal<W> {
    type Target = W;

    fn deref(&self) -> &W {
        &self.term
    }
}

impl<W: ConsoleWrite> ops::DerefMut for MouseTerminal<W> {
    fn deref_mut(&mut self) -> &mut W {
        &mut self.term
    }
}

impl<W: ConsoleWrite> Write for MouseTerminal<W> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.term.write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.term.flush()
    }
}

impl<W: ConsoleWrite> ConsoleWrite for MouseTerminal<W> {
    fn set_raw_mode(&mut self, mode: bool) -> io::Result<bool> {
        self.term.set_raw_mode(mode)
    }

    fn is_raw_mode(&self) -> bool {
        self.term.is_raw_mode()
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use event::{Event, Key, KeyCode, KeyMod, MouseButton, MouseEvent};
    use std::cell::RefCell;

    thread_local!(static LEFTOVER: RefCell<Option<u8>> = RefCell::new(None));

    impl ConsoleRead for &[u8] {
        fn get_event_and_raw(
            &mut self,
            _timeout: Option<Duration>,
        ) -> Option<io::Result<(Event, Vec<u8>)>> {
            LEFTOVER.with(|leftover| event_and_raw(self, &mut leftover.borrow_mut()))
        }

        fn poll(&mut self, _timeout: Option<Duration>) -> bool {
            self.len() > 0
        }

        fn read_timeout(
            &mut self,
            buf: &mut [u8],
            _timeout: Option<Duration>,
        ) -> io::Result<usize> {
            self.read(buf)
        }
    }

    #[test]
    fn test_keys() {
        let mut i = b"\x1Bayo\x7F\x1B[D".keys();

        assert_eq!(
            i.next().unwrap().unwrap(),
            Key::new_mod(KeyCode::Char('a'), KeyMod::Alt)
        );
        assert_eq!(i.next().unwrap().unwrap(), Key::new(KeyCode::Char('y')));
        assert_eq!(i.next().unwrap().unwrap(), Key::new(KeyCode::Char('o')));
        assert_eq!(i.next().unwrap().unwrap(), Key::new(KeyCode::Backspace));
        assert_eq!(i.next().unwrap().unwrap(), Key::new(KeyCode::Left));
        assert!(i.next().is_none());
    }

    #[test]
    fn test_events() {
        let mut i = b"\x1B[\x00bc\x7F\x1B[D\
                    \x1B[M\x00\x22\x24\x1B[<0;2;4;M\x1B[32;2;4M\x1B[<0;2;4;m\x1B[35;2;4Mb"
            .events();

        assert_eq!(
            i.next().unwrap().unwrap(),
            Event::Unsupported(vec![0x1B, b'[', 0x00])
        );
        assert_eq!(
            i.next().unwrap().unwrap(),
            Event::Key(Key::new(KeyCode::Char('b')))
        );
        assert_eq!(
            i.next().unwrap().unwrap(),
            Event::Key(Key::new(KeyCode::Char('c')))
        );
        assert_eq!(
            i.next().unwrap().unwrap(),
            Event::Key(Key::new(KeyCode::Backspace))
        );
        assert_eq!(
            i.next().unwrap().unwrap(),
            Event::Key(Key::new(KeyCode::Left))
        );
        assert_eq!(
            i.next().unwrap().unwrap(),
            Event::Mouse(MouseEvent::Press(MouseButton::WheelUp, 2, 4))
        );
        assert_eq!(
            i.next().unwrap().unwrap(),
            Event::Mouse(MouseEvent::Press(MouseButton::Left, 2, 4))
        );
        assert_eq!(
            i.next().unwrap().unwrap(),
            Event::Mouse(MouseEvent::Press(MouseButton::Left, 2, 4))
        );
        assert_eq!(
            i.next().unwrap().unwrap(),
            Event::Mouse(MouseEvent::Release(2, 4))
        );
        assert_eq!(
            i.next().unwrap().unwrap(),
            Event::Mouse(MouseEvent::Release(2, 4))
        );
        assert_eq!(
            i.next().unwrap().unwrap(),
            Event::Key(Key::new(KeyCode::Char('b')))
        );
        assert!(i.next().is_none());
    }

    #[test]
    fn test_events_and_raw() {
        let input = b"\x1B[\x00bc\x7F\x1B[D\
                    \x1B[M\x00\x22\x24\x1B[<0;2;4;M\x1B[32;2;4M\x1B[<0;2;4;m\x1B[35;2;4Mb";
        let mut output = Vec::<u8>::new();
        {
            let mut i = input
                .events_and_raw()
                .map(|res| res.unwrap())
                .inspect(|&(_, ref raw)| {
                    output.extend(raw);
                })
                .map(|(event, _)| event);

            assert_eq!(
                i.next().unwrap(),
                Event::Unsupported(vec![0x1B, b'[', 0x00])
            );
            assert_eq!(i.next().unwrap(), Event::Key(Key::new(KeyCode::Char('b'))));
            assert_eq!(i.next().unwrap(), Event::Key(Key::new(KeyCode::Char('c'))));
            assert_eq!(i.next().unwrap(), Event::Key(Key::new(KeyCode::Backspace)));
            assert_eq!(i.next().unwrap(), Event::Key(Key::new(KeyCode::Left)));
            assert_eq!(
                i.next().unwrap(),
                Event::Mouse(MouseEvent::Press(MouseButton::WheelUp, 2, 4))
            );
            assert_eq!(
                i.next().unwrap(),
                Event::Mouse(MouseEvent::Press(MouseButton::Left, 2, 4))
            );
            assert_eq!(
                i.next().unwrap(),
                Event::Mouse(MouseEvent::Press(MouseButton::Left, 2, 4))
            );
            assert_eq!(i.next().unwrap(), Event::Mouse(MouseEvent::Release(2, 4)));
            assert_eq!(i.next().unwrap(), Event::Mouse(MouseEvent::Release(2, 4)));
            assert_eq!(i.next().unwrap(), Event::Key(Key::new(KeyCode::Char('b'))));
            assert!(i.next().is_none());
        }

        assert_eq!(input.iter().map(|b| *b).collect::<Vec<u8>>(), output)
    }

    #[test]
    fn test_function_keys() {
        let mut st = b"\x1BOP\x1BOQ\x1BOR\x1BOS".keys();
        for i in 1..5 {
            assert_eq!(st.next().unwrap().unwrap(), Key::new(KeyCode::F(i)));
        }

        let mut st = b"\x1B[11~\x1B[12~\x1B[13~\x1B[14~\x1B[15~\
        \x1B[17~\x1B[18~\x1B[19~\x1B[20~\x1B[21~\x1B[23~\x1B[24~"
            .keys();
        for i in 1..13 {
            assert_eq!(st.next().unwrap().unwrap(), Key::new(KeyCode::F(i)));
        }
    }

    #[test]
    fn test_special_keys() {
        let mut st = b"\x1B[2~\x1B[H\x1B[7~\x1B[5~\x1B[3~\x1B[F\x1B[8~\x1B[6~".keys();
        assert_eq!(st.next().unwrap().unwrap(), Key::new(KeyCode::Insert));
        assert_eq!(st.next().unwrap().unwrap(), Key::new(KeyCode::Home));
        assert_eq!(st.next().unwrap().unwrap(), Key::new(KeyCode::Home));
        assert_eq!(st.next().unwrap().unwrap(), Key::new(KeyCode::PageUp));
        assert_eq!(st.next().unwrap().unwrap(), Key::new(KeyCode::Delete));
        assert_eq!(st.next().unwrap().unwrap(), Key::new(KeyCode::End));
        assert_eq!(st.next().unwrap().unwrap(), Key::new(KeyCode::End));
        assert_eq!(st.next().unwrap().unwrap(), Key::new(KeyCode::PageDown));
        assert!(st.next().is_none());
    }

    #[test]
    fn test_esc_key() {
        let mut st = b"\x1B".keys();
        assert_eq!(st.next().unwrap().unwrap(), Key::new(KeyCode::Esc));
        assert!(st.next().is_none());
    }
}

use sl_console::*;

use std::io::{self, Read, Write};

/// Read a line.
///
/// EOT and ETX will abort the prompt, returning `None`. Newline or carriage return will
/// complete the input.
fn read_line() -> io::Result<Option<String>> {
    let mut buf = Vec::with_capacity(30);

    for c in conin().bytes() {
        match c {
            Err(e) => return Err(e),
            Ok(0) | Ok(3) | Ok(4) => return Ok(None),
            Ok(0x7f) => {
                buf.pop();
            }
            Ok(b'\n') | Ok(b'\r') => break,
            Ok(c) => buf.push(c),
        }
    }

    let string =
        String::from_utf8(buf).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
    Ok(Some(string))
}

fn main() {
    con_init().unwrap();
    let mut conout = conout().into_raw_mode().unwrap();

    conout.write_all(b"password: ").unwrap();
    conout.flush().unwrap();

    let pass = read_line();

    if let Ok(Some(pass)) = pass {
        conout.write_all(pass.as_bytes()).unwrap();
        conout.write_all(b"\n").unwrap();
    } else {
        conout.write_all(b"Error\n").unwrap();
    }
}

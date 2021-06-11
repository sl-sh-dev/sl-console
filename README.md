This is a fork of termion (https://gitlab.redox-os.org/redox-os/termion).
It adds window support (for 10 with new console ansi code support) based on
Jezza's MR (https://gitlab.redox-os.org/redox-os/termion/-/merge_requests/151).
It also attempts to fix some of the issues with the async stuff by treating the
console/tty as a singleton and controlling access that way.  It also uses
non-blocking io and select on unix to support async and a thread on windows
(similiar to termion's async but with gated access and only one thread ever
started).

**sl-console** is a pure Rust, bindless library for low-level handling, manipulating
and reading information about terminals.

Sl-console aims to be simple and yet expressive. It is bindless, meaning that it
is not a front-end to some other library (e.g., ncurses or termbox), but a
standalone library directly talking to the TTY.

Sl-console is quite convenient, due to its complete coverage of essential TTY
features, providing one consistent API. Sl-console is rather low-level containing
only abstraction aligned with what actually happens behind the scenes.

Sl-console generates escapes and API calls for the user. This makes it a whole lot
cleaner to use escapes.

Supports Mac OS X, BSD, Linux and Windows (or, in general, ANSI terminals).

## A note on stability

This forked crate is not yet stable.

## Cargo.toml

```toml
[dependencies]
sl-console = { git = "https://github.com/sl-sh-dev/sl-console.git" }
```

## Features

- Raw mode.
- TrueColor.
- 256-color mode.
- Cursor movement.
- Text formatting.
- Console size.
- TTY-only stream.
- Control sequences.
- Termios control.
- Password input.
- Windows 10 support (when console suports ansi escape codes).
- Safe `isatty` wrapper.
- Special keys events (modifiers, special keys, etc.).
- Asynchronous key events.
- Mouse input.
- Detailed documentation on every item.

and much more.

## Examples

### Style and colors.

```rust
use sl_console::{color, style};

use std::io;

fn main() {
    println!("{}Red", color::Fg(color::Red));
    println!("{}Blue", color::Fg(color::Blue));
    println!("{}Blue'n'Bold{}", style::Bold, style::Reset);
    println!("{}Just plain italic", style::Italic);
}
```

### Moving the cursor

```rust
fn main() {
    print!("{}{}Stuff", sl_console::clear::All, sl_console::cursor::Goto(1, 1));
}

```

### Mouse

```rust
use sl_console::event::{Key, Event, MouseEvent};
use sl_console::input::{TermRead, MouseTerminal};
use sl_console::raw::IntoRawMode;
use std::io::{Write, stdout, stdin};

fn main() {
    let stdin = stdin();
    let mut stdout = MouseTerminal::from(stdout().into_raw_mode().unwrap());

    write!(stdout, "{}{}q to exit. Click, click, click!", sl_console::clear::All, sl_console::cursor::Goto(1, 1)).unwrap();
    stdout.flush().unwrap();

    for c in stdin.events() {
        let evt = c.unwrap();
        match evt {
            Event::Key(Key::Char('q')) => break,
            Event::Mouse(me) => {
                match me {
                    MouseEvent::Press(_, x, y) => {
                        write!(stdout, "{}x", sl_console::cursor::Goto(x, y)).unwrap();
                    },
                    _ => (),
                }
            }
            _ => {}
        }
        stdout.flush().unwrap();
    }
}
```

### Read a password

```rust
use sl_console::input::TermRead;
use std::io::{Write, stdout, stdin};

fn main() {
    let stdout = stdout();
    let mut stdout = stdout.lock();
    let stdin = stdin();
    let mut stdin = stdin.lock();

    stdout.write_all(b"password: ").unwrap();
    stdout.flush().unwrap();

    let pass = stdin.read_passwd(&mut stdout);

    if let Ok(Some(pass)) = pass {
        stdout.write_all(pass.as_bytes()).unwrap();
        stdout.write_all(b"\n").unwrap();
    } else {
        stdout.write_all(b"Error\n").unwrap();
    }
}
```

## Usage

See `examples/`, and the documentation, which can be rendered using `cargo doc`.

For a more complete example, see [a minesweeper implementation](https://github.com/redox-os/games-for-redox/blob/master/src/minesweeper/main.rs), that I made for Redox using termion.

<img src="image.png" width="200">

## License

MIT/X11.

This is a fork of termion (https://gitlab.redox-os.org/redox-os/termion).
It adds window support (for 10 with new console ansi code support) based on
Jezza's MR (https://gitlab.redox-os.org/redox-os/termion/-/merge_requests/151).
It also attempts to fix some of the issues with the async stuff by treating the
console/tty as a singleton and controlling access that way.  It also uses
non-blocking io and select on unix to support async and a thread on windows
(similiar to termion's async but with gated access and only one thread ever
started).

Note on redox support: it is removed in this fork.  Without someone to maintain
it it is not viable.  If someone wants to readd and maintain it (should not be
hard- start with termion sys/redox and look at changes in sys/unix) that would
be cool.

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

## Notes on changes from termion
- Adds a console abstraction.  Use con_init() to setup and conin()/conout() to
access the in/read and out/write sides.  These work similiar to stdin()/stdout()
and use reentrant mutexes for locking.
- Raw mode can be entered with conout().raw_mode_on() (or raw_mode_off() to
exit) or with conout().raw_mode_guard().  If using the RAII guard it is not
currently an output object, it will restore normal mode (or whatever mode was
active when it was created) when it is dropped.  Note this requires something
with the ConsoleWrite trait not just Write.  Raw mode may be refined.
- The input console can be accessed non-blocking and can be polled for
readiness (with or without a timeout).
- The console abstraction should have fixed the issues termion had with it's
async input.
- Supports Windows (at least 10 or higher with a console that support ansi
escape codes- any version released in the last several years).

## Notes on threading
Sl-console is thread safe.  Conin/conout are singletons that are protected by
a mutex (a ReentrantMutex from parking lot specifically- why parking lot is a
dependency).  The unix version will not start any threads on it's own, it uses
non-blocking io and select for non-blocking conin.  The Windows version will
start a thread when conin is initialized (usually when con_init() is called but
possibly when conin()/conin_r() is called if that happens first).  This thread
should last for the life of the program and is reading the input console
(CONIN$).  Entering raw mode requires locks on both conin and conout.  It will
not deadlock (uses try_lock() not lock()) so may return a WouldBlock error if
it can not get both locks.  In general you probably want to access the console
from a single thread to avoid issues but it is thread safe.  It just may be
hard to keep input and output straight and entering raw mode once multiple
threads are accessing the console might be tricky (entering raw mode first
would be best).

## A note on stability

This forked crate is not yet stable (too many changes that need to settle).

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
use sl_console::*;
use sl_console::{color, style};

use std::io::{stdout, Write};

fn main() {
    // If stdout is the tty/console this will work.
    // Check to make sure stdout is a tty (skipping this will leave escape
    // codes in your output if not a tty/console).
    println!("Direct to stdout:");
    if is_tty(&stdout()) {
        println!("{}Red", color::Fg(color::Red));
        println!("{}Blue", color::Fg(color::Blue));
        println!("{}Blue'n'Bold{}", style::Bold, style::Reset);
        println!("{}Just plain italic", style::Italic);
    } else {
        println!("stdout not a tty!");
    }

    // Alternatively use the console directly.
    // This skips stdout and opens then tty/console directly.
    println!("\nDirect to console:");
    // If con_init() returns an error then there is no tty/console to attach
    // to.  DO NOT call conin()/conout() in this case, they will panic if there
    // is not console.
    con_init().unwrap();
    // The lock below is optional, works like lock() for stdout() and saves
    // each access to conout from needing to lock.  Note the locks in
    // conin/conout are reentrant so it is safe to lock multiple times in the
    // same thread (it won't deadlock).
    let mut conout = conout().lock();
    write!(conout, "{}Red\n", color::Fg(color::Red)).unwrap();
    write!(conout, "{}Blue\n", color::Fg(color::Blue)).unwrap();
    write!(conout, "{}Blue'n'Bold{}\n", style::Bold, style::Reset).unwrap();
    write!(conout, "{}Just plain italic\n", style::Italic).unwrap();
}
```

### Moving the cursor

```rust
use sl_console::*;

use std::io::{stdout, Write};

fn main() {
    // If stdout is the tty/console this will work.
    // Check to make sure stdout is a tty (skipping this will leave escape
    // codes in your output if not a tty/console) and wont clear or goto (1, 1).
    if is_tty(&stdout()) {
        print!("{}{}Stdout Stuff", sl_console::clear::All, sl_console::cursor::Goto(1, 1));
    }
    stdout().flush().unwrap();
    std::thread::sleep(std::time::Duration::from_millis(5000));

    // Alternatively use the console directly.
    // This skips stdout and opens then tty/console directly.
    con_init().unwrap();
    let mut conout = conout();
    write!(conout, "{}{}Console Stuff", sl_console::clear::All, sl_console::cursor::Goto(1, 1)).unwrap();
}
```

### Mouse

```rust
use sl_console::*;
use sl_console::event::{Key, Event, MouseEvent};
use sl_console::input::{TermRead, MouseTerminal};
use std::io::Write;

fn main() {
    con_init().unwrap();
    let mut conout = conout();
    let _raw = conout.raw_mode_guard().unwrap();
    let mut conout = MouseTerminal::from(conout);

    write!(conout, "{}{}q to exit. Click, click, click!", sl_console::clear::All, sl_console::cursor::Goto(1, 1)).unwrap();
    conout.flush().unwrap();

    for c in conin().events() {
        let evt = c.unwrap();
        match evt {
            Event::Key(Key::Char('q')) => break,
            Event::Mouse(me) => {
                match me {
                    MouseEvent::Press(_, x, y) => {
                        write!(conout, "{}x", sl_console::cursor::Goto(x, y)).unwrap();
                    },
                    _ => (),
                }
            }
            _ => {}
        }
        conout.flush().unwrap();
    }
}
```

### Read a password

```rust
use sl_console::*;
use sl_console::input::TermRead;
use std::io::Write;

fn main() {
    con_init().unwrap();
    let mut conout = conout().lock();
    let mut conin = conin().lock();
    // Raw mode so entered text is not echoed back.
    let _raw = conout.raw_mode_guard().unwrap();

    conout.write_all(b"password: ").unwrap();
    conout.flush().unwrap();

    let pass = conin.read_line();

    if let Ok(Some(pass)) = pass {
        conout.write_all(pass.as_bytes()).unwrap();
        conout.write_all(b"\n").unwrap();
    } else {
        conout.write_all(b"Error\n").unwrap();
    }
}
```

## Usage

See `examples/`, and the documentation, which can be rendered using `cargo doc`.

For a more complete example, see [a minesweeper implementation](https://github.com/sl-sh-dev/sl-console/blob/master/examples/minesweeper.rs).  This was ported from [redox games minesweeper](https://gitlab.redox-os.org/redox-os/games/-/tree/master/src/minesweeper).

<img src="image.png" width="200">

## License

MIT/X11.

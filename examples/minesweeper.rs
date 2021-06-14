//! This example is a simple implementation of minesweeper.
//! Original source from https://gitlab.redox-os.org/redox-os/games/-/tree/master/src/minesweeper
//! Adapted to sl-console and tweaked for playability (mouse, mines don't wrap).

/*
MIT License

Copyright (c) 2017 Redox OS

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.
*/

use sl_console::color::*;
use sl_console::event::*;
use sl_console::input::*;
use sl_console::*;

use std::env;
use std::io::{self, BufWriter, Write};
use std::process;

/// A LGC based, non-cryptographic, pseudo-random number generator with full cycle length (2^64 - 1).
///
/// To avoid hyperplanes, we apply a bijective function on the output.
/// Taken from here https://gitlab.redox-os.org/redox-os/libextra/-/blob/master/src/rand.rs
/// and stripped down.
pub struct Randomizer {
    state: u64,
}

impl Randomizer {
    /// Create a new randomizer from a seed.
    pub fn new(seed: u64) -> Randomizer {
        Randomizer {
            state: seed.wrapping_add(0xDEADBEEFDEADBEEF),
        }
    }

    /// Read a byte from the randomizer.
    pub fn read_u8(&mut self) -> u8 {
        self.state = self.state.wrapping_mul(6364136223846793005).wrapping_add(1);
        (self.state.wrapping_mul(1152921504735157271).rotate_right(2) ^ 0xFAB00105C0DE) as u8
    }
}

/// A cell in the grid.
#[derive(Copy, Clone, PartialEq, Eq, Debug, Hash)]
struct Cell {
    /// Does it contain a mine?
    mine: bool,
    /// Is it revealed?
    ///
    /// That is, is it showed or chosen previously by the player?
    revealed: bool,
    /// Is this cell observed?
    ///
    /// That is, is the state of this cell determined, or is it pending for randomization.
    observed: bool,
    /// Does this flag contain a flag?
    flagged: bool,
}

const BG_REVEALED: Bg<Rgb> = Bg(Rgb(128, 128, 128));
const FG_REVEALED: Fg<Rgb> = Fg(Rgb(0, 0, 0));
const BG_CONCEALED: Bg<Rgb> = Bg(Rgb(0, 0, 0));
/// The string printed for flagged cells.
const FLAGGED: &'static str = "X";
/// The string printed for mines in the game over revealing.
const MINE: &'static str = "*";
/// The string printed for concealed cells.
const CONCEALED: &'static str = " "; //▒";

/// The game over screen.
const GAME_OVER: &'static str = "╔═════════════════╗\n\r\
                                 ║───┬Game over────║\n\r\
                                 ║ r ┆ replay      ║\n\r\
                                 ║ q ┆ quit        ║\n\r\
                                 ╚═══╧═════════════╝";

/// The upper and lower boundary char.
const HORZ_BOUNDARY: &'static str = "─";
/// The left and right boundary char.
const VERT_BOUNDARY: &'static str = "│";

/// The top-left corner
const TOP_LEFT_CORNER: &'static str = "┌";
/// The top-right corner
const TOP_RIGHT_CORNER: &'static str = "┐";
/// The bottom-left corner
const BOTTOM_LEFT_CORNER: &'static str = "└";
/// The bottom-right corner
const BOTTOM_RIGHT_CORNER: &'static str = "┘";

/// The help page.
const HELP: &'static str = r#"
minesweeper ~ a simple minesweeper implementation.

rules:
    Select a cell to reveal, printing the number of adjacent cells holding a mine.
    If no adjacent cells hold a mine, the cell is called free. Free cell will recursively
    reveal their neighboring cells. If a mine is revealed, you loose.

flags:
    -r | --height N ~ set the height of the grid.
    -c | --width N  ~ set the width of the grid.
    -s | --seed N   ~ use N as the random seed instead of current time.  Note
                      that due to how mines are placed one would have to 
                      select squares in the same order for consistency so this
                      is currently not that useful.
    -h | --help     ~ this help page.
    -b              ~ beginner mode.
    -i              ~ intermediate mode.
    -a              ~ advanced mode.
    -g              ~ god mode.

controls:
    ---selection--------------------
    space | left mouse ~ reveal the current cell.
    ---movement---------------------
    h | a ~ move left.
    j | s ~ move down.
    k | w ~ move up.
    l | d ~ move right.
    ---flags------------------------
    f | right mouse ~ toggle flag.
    ---control----------------------
    q     ~ quit game.
    r     ~ restart game.

author:
    ticki.
"#;

/// The game state.
struct Game<R: ConsoleRead, W: Write> {
    /// Width of the grid.
    width: u16,
    /// Height of the grid.
    height: u16,
    /// The grid.
    ///
    /// The cells are enumerated like you would read a book. Left to right, until you reach the
    /// line ending.
    grid: Box<[Cell]>,
    /// The difficulty of the game.
    ///
    /// The lower, the easier.
    difficulty: u8,
    /// The x coordinate.
    x: u16,
    /// The y coordinate.
    y: u16,
    /// The randomizer.
    rand: Randomizer,
    /// Points.
    ///
    /// That is, revealed fields.
    points: u16,
    /// Console output.
    conout: W,
    /// Console input.
    conin: R,
    /// Is this the first click of a new game?
    first_click: bool,
}

/// Initialize the game.
fn init(difficulty: u8, w: u16, h: u16, rand_seed: u64) {
    // Get and lock the console out.
    let mut conout = conout().lock();
    // Use the mouse.  Drop for Game will turn it back off in the terminal.
    conout.mouse_on().expect("Failed to turn on mouse support!");
    // Let's go to raw mode.  Not using the guard, Game will turn off raw mode
    // when dropped.
    conout
        .raw_mode_on()
        .expect("Unable to put console in raw mode!");
    // Wrap the locked conout in a BufWriter, this will make a noticable
    // performance difference.  Can still access conout for one off calls
    // that need ConsoleWrite with conout().  Since we are single threaded and
    // the lock is reentrant this is fine.
    let mut conout = BufWriter::new(conout);
    // Grab the locked conin, in theory this will be faster but it is waiting
    // on input so is probably pointless.  We could not save conin in Game and
    // just use conin().get_key() or conin().get_event() as well.
    let conin = conin().lock();
    write!(conout, "{}", clear::All).unwrap();

    // Set the initial game state.
    let mut game = Game {
        x: 0,
        y: 0,
        rand: Randomizer::new(rand_seed),
        width: w,
        height: h,
        grid: vec![
            Cell {
                mine: false,
                revealed: false,
                observed: false,
                flagged: false,
            };
            w as usize * h as usize + 1
        ]
        .into_boxed_slice(),
        points: 0,
        conin,
        conout,
        difficulty,
        first_click: true,
    };

    // Reset that game.
    game.reset();

    // Start the event loop.
    while game.start() {}
}

impl<R: ConsoleRead, W: Write> Drop for Game<R, W> {
    fn drop(&mut self) {
        // When done, restore the defaults to avoid messing with the terminal.
        if write!(
            self.conout,
            "{}{}{}",
            clear::All,
            style::Reset,
            cursor::Goto(1, 1)
        )
        .is_err()
        {}
        // Done with raw mode.
        if conout().raw_mode_off().is_err() {};
        // Done with mouse.
        if conout().mouse_off().is_err() {};
    }
}

impl<R: ConsoleRead, W: Write> Game<R, W> {
    /// Get the grid position of a given coordinate.
    fn pos(&self, x: u16, y: u16) -> usize {
        if x == u16::MAX || y == u16::MAX {
            self.width as usize * self.height as usize
        } else {
            y as usize * self.width as usize + x as usize
        }
    }

    /// Read cell, randomizing it if it is unobserved.
    fn read_cell(&mut self, c: usize) {
        if !self.grid[c].observed {
            self.grid[c].mine = self.rand.read_u8() % self.difficulty == 0;
            self.grid[c].observed = true;
        }
    }

    /// Get the cell at (x, y).
    fn get(&mut self, x: u16, y: u16) -> Cell {
        let pos = self.pos(x, y);

        self.read_cell(pos);
        self.grid[pos]
    }

    /// Get a mutable reference to the cell at (x, y).
    fn get_mut(&mut self, x: u16, y: u16) -> &mut Cell {
        let pos = self.pos(x, y);

        self.read_cell(pos);
        &mut self.grid[pos]
    }

    fn click(&mut self) -> bool {
        // Check if it was a mine.
        let (x, y) = (self.x, self.y);

        if self.first_click {
            // This is the player's first turn; clear all cells of
            // mines around the cursor.
            for &(x, y) in self.adjacent(x, y).iter() {
                self.get_mut(x, y).mine = false;
            }
            self.get_mut(x, y).mine = false;
            self.first_click = false;
        }

        if self.get(x, y).mine {
            self.reveal_all();
            // Make the background colour of the mine we just
            // landed on red, and the foreground black.
            write!(
                self.conout,
                "{}{}{}{}{}",
                cursor::Goto(x + 2, y + 2),
                Bg(Red),
                Fg(Black),
                MINE,
                style::Reset
            )
            .unwrap();
            return self.game_over();
        }

        if !self.get(x, y).revealed {
            self.points += 1;
        }

        // Reveal the cell.
        self.reveal(x, y);

        self.print_points();
        self.conout.flush().unwrap();
        true
    }

    /// Start the game loop.
    ///
    /// This will listen to events and do the appropriate actions.
    fn start(&mut self) -> bool {
        loop {
            // Read an event and ignore an error.
            let event = if let Ok(e) = self.conin.get_event() {
                e
            } else {
                continue;
            };
            use sl_console::event::Key::*;
            match event {
                Event::Key(ch) => match ch {
                    Char('h') | Char('a') | Left => self.x = self.left(self.x),
                    Char('j') | Char('s') | Down => self.y = self.down(self.y),
                    Char('k') | Char('w') | Up => self.y = self.up(self.y),
                    Char('l') | Char('d') | Right => self.x = self.right(self.x),
                    Char(' ') => {
                        if !self.click() {
                            return false;
                        }
                    }
                    Char('f') => {
                        let (x, y) = (self.x, self.y);
                        self.toggle_flag(x, y);
                    }
                    Char('r') => {
                        self.reset();
                        return true;
                    }
                    Char('q') => return false,
                    _ => {}
                },
                Event::Mouse(me) => match me {
                    MouseEvent::Press(MouseButton::Left, a, b) => {
                        if a > 1 && a < (self.width + 2) && b > 1 && b < (self.height + 2) {
                            self.x = a - 2;
                            self.y = b - 2;
                            cursor::goto(self.x + 2, self.y + 2).unwrap();
                            if !self.click() {
                                return false;
                            }
                        }
                    }
                    MouseEvent::Press(MouseButton::Right, a, b) => {
                        if a > 1 && a < (self.width + 2) && b > 1 && b < (self.height + 2) {
                            self.x = a - 2;
                            self.y = b - 2;
                            cursor::goto(self.x + 2, self.y + 2).unwrap();
                            self.toggle_flag(self.x, self.y);
                        }
                    }
                    _ => {}
                },
                _ => {}
            }
            // Make sure the cursor is placed on the current position.
            cursor::goto(self.x + 2, self.y + 2).unwrap();
        }
    }

    /// Set a flag on cell.
    fn set_flag(&mut self, x: u16, y: u16) {
        if !self.get(x, y).revealed {
            write!(
                self.conout,
                "{}{}{}",
                Fg(Rgb(0, 255, 0)),
                FLAGGED,
                Fg(Reset)
            )
            .unwrap();
            self.conout.flush().unwrap();
            self.get_mut(x, y).flagged = true;
        }
    }
    /// Remove a flag on cell.
    fn remove_flag(&mut self, x: u16, y: u16) {
        write!(self.conout, "{}{}{}", BG_CONCEALED, CONCEALED, Bg(Reset)).unwrap();
        self.conout.flush().unwrap();
        self.get_mut(x, y).flagged = false;
    }
    /// Place a flag on cell if unflagged, or remove it if present.
    fn toggle_flag(&mut self, x: u16, y: u16) {
        if !self.get(x, y).flagged {
            self.set_flag(x, y);
        } else {
            self.remove_flag(x, y);
        }
    }

    /// Reset the game.
    ///
    /// This will display the starting grid, and fill the old grid with random mines.
    fn reset(&mut self) {
        // Reset the cursor.
        write!(self.conout, "{}", cursor::Goto(1, 1)).unwrap();

        // Write the upper part of the frame.
        self.conout.write(TOP_LEFT_CORNER.as_bytes()).unwrap();
        for _ in 0..self.width {
            self.conout.write(HORZ_BOUNDARY.as_bytes()).unwrap();
        }
        self.conout.write(TOP_RIGHT_CORNER.as_bytes()).unwrap();
        self.conout.write(b"\n\r").unwrap();

        // Conceal all the cells.
        for _ in 0..self.height {
            // The left part of the frame
            self.conout.write(VERT_BOUNDARY.as_bytes()).unwrap();

            for _ in 0..self.width {
                write!(self.conout, "{}{}{}", BG_CONCEALED, CONCEALED, Bg(Reset)).unwrap();
            }

            // The right part of the frame.
            self.conout.write(VERT_BOUNDARY.as_bytes()).unwrap();
            self.conout.write(b"\n\r").unwrap();
        }

        // Write the lower part of the frame.
        self.conout.write(BOTTOM_LEFT_CORNER.as_bytes()).unwrap();
        for _ in 0..self.width {
            self.conout.write(HORZ_BOUNDARY.as_bytes()).unwrap();
        }
        self.conout.write(BOTTOM_RIGHT_CORNER.as_bytes()).unwrap();

        write!(self.conout, "{}", cursor::Goto(self.x + 2, self.y + 2)).unwrap();
        self.conout.flush().unwrap();

        // Reset the grid.
        for i in 0..self.grid.len() {
            // Fill it with random, concealed fields.
            self.grid[i] = Cell {
                mine: false,
                revealed: false,
                observed: false,
                flagged: false,
            };

            self.points = 0;
        }
        self.grid[self.grid.len() - 1].observed = true;
        self.grid[self.grid.len() - 1].revealed = true;
        self.first_click = true;
    }

    /// Get the value of a cell.
    ///
    /// The value represent the sum of adjacent cells containing mines. A cell of value, 0, is
    /// called "free".
    fn val(&mut self, x: u16, y: u16) -> u8 {
        // To avoid nightly version, we manually sum the adjacent mines.
        let mut res = 0;
        for &(x, y) in self.adjacent(x, y).iter() {
            res += self.get(x, y).mine as u8;
        }
        res
    }

    /// Reveal the cell, _c_.
    ///
    /// This will recursively reveal free cells, until non-free cell is reached, terminating the
    /// current recursion descendant.
    fn reveal(&mut self, x: u16, y: u16) {
        let v = self.val(x, y);

        self.get_mut(x, y).revealed = true;

        write!(self.conout, "{}", cursor::Goto(x + 2, y + 2)).unwrap();

        if v == 0 {
            // If the cell is free, simply put a space on the position.
            write!(
                self.conout,
                "{}{} {}{}",
                FG_REVEALED,
                BG_REVEALED,
                Bg(Reset),
                Fg(Reset)
            )
            .unwrap();

            // Recursively reveal adjacent cells until a non-free cel is reached.
            for &(x, y) in self.adjacent(x, y).iter() {
                if !self.get(x, y).revealed && !self.get(x, y).mine {
                    self.reveal(x, y);
                }
            }
        } else {
            // Aww. The cell was not free. Print the value instead.
            write!(
                self.conout,
                "{}{}{}{}{}",
                FG_REVEALED,
                BG_REVEALED,
                (b'0' + v) as char,
                Bg(Reset),
                Fg(Reset)
            )
            .unwrap();
        }
    }

    /// Print the point count.
    fn print_points(&mut self) {
        write!(self.conout, "{}", cursor::Goto(3, self.height + 2)).unwrap();
        self.conout
            .write(self.points.to_string().as_bytes())
            .unwrap();
    }

    /// Reveal all the fields, printing where the mines were.
    fn reveal_all(&mut self) {
        write!(self.conout, "{}", cursor::Goto(1, 1)).unwrap();

        for y in 0..self.height {
            for x in 0..self.width {
                write!(self.conout, "{}", cursor::Goto(x + 2, y + 2)).unwrap();
                if self.get(x, y).mine {
                    write!(
                        self.conout,
                        "{}{}{}{}{}",
                        FG_REVEALED,
                        BG_REVEALED,
                        MINE,
                        Bg(Reset),
                        Fg(Reset)
                    )
                    .unwrap();
                }
            }
        }
    }

    /// Game over!
    fn game_over(&mut self) -> bool {
        let termsize = sl_console::terminal_size().ok();
        let termheight = termsize.map(|(_, h)| h).or_else(|| Some(6)).unwrap();
        //Goto bottom left corner
        write!(self.conout, "{}", cursor::Goto(1, termheight - 5)).unwrap();

        self.conout.write(GAME_OVER.as_bytes()).unwrap();
        self.conout.flush().unwrap();

        loop {
            // Repeatedly read a single key.
            let key = self.conin.get_key();
            match key {
                Ok(Key::Char('r')) => {
                    // Replay!
                    self.reset();
                    return true;
                }
                Ok(Key::Char('q')) => return false,
                _ => {}
            }
        }
    }

    /// Calculate the adjacent cells.
    fn adjacent(&self, x: u16, y: u16) -> [(u16, u16); 8] {
        let left = if x > 0 && x < u16::MAX {
            x - 1
        } else {
            u16::MAX
        };
        let right = if x < (self.width - 1) {
            x + 1
        } else {
            u16::MAX
        };
        let up = if y > 0 && y < u16::MAX {
            y - 1
        } else {
            u16::MAX
        };
        let down = if y < (self.height - 1) {
            y + 1
        } else {
            u16::MAX
        };

        [
            // Left-up
            (left, up),
            // Up
            (x, up),
            // Right-up
            (right, up),
            // Left
            (left, y),
            // Right
            (right, y),
            // Left-down
            (left, down),
            // Down
            (x, down),
            // Right-down
            (right, down),
        ]
    }

    /// Calculate the y coordinate of the cell "above" a given y coordinate.
    ///
    /// This wraps when _y = 0_.
    fn up(&self, y: u16) -> u16 {
        if y == 0 {
            // Upper bound reached. Wrap around.
            self.height - 1
        } else {
            y - 1
        }
    }
    /// Calculate the y coordinate of the cell "below" a given y coordinate.
    ///
    /// This wraps when _y = h - 1_.
    fn down(&self, y: u16) -> u16 {
        if y + 1 == self.height {
            // Lower bound reached. Wrap around.
            0
        } else {
            y + 1
        }
    }
    /// Calculate the x coordinate of the cell "left to" a given x coordinate.
    ///
    /// This wraps when _x = 0_.
    fn left(&self, x: u16) -> u16 {
        if x == 0 {
            // Lower bound reached. Wrap around.
            self.width - 1
        } else {
            x - 1
        }
    }
    /// Calculate the x coordinate of the cell "left to" a given x coordinate.
    ///
    /// This wraps when _x = w - 1_.
    fn right(&self, x: u16) -> u16 {
        if x + 1 == self.width {
            // Upper bound reached. Wrap around.
            0
        } else {
            x + 1
        }
    }
}

fn main() {
    let mut args = env::args().skip(1);
    let mut width = None;
    let mut height = None;
    let mut diff = 6;

    // Init the console.
    con_init().expect("Unable to initialize the console!");
    let stderr = io::stderr();
    let mut stderr = stderr.lock();

    // Get a default random seed.
    let mut seed = std::time::SystemTime::now()
        .elapsed()
        .expect("Unable to get system time for random seed!")
        .as_secs();

    loop {
        // Read the arguments.
        // Does not use a for loop because each argument may have second parameter.

        let arg = if let Some(x) = args.next() {
            x
        } else {
            break;
        };

        match arg.as_str() {
            "-r" | "--height" => {
                if height.is_none() {
                    height = Some(
                        args.next()
                            .unwrap_or_else(|| {
                                stderr.write(b"no height given.\n").unwrap();
                                stderr.flush().unwrap();
                                process::exit(1);
                            })
                            .parse()
                            .unwrap_or_else(|_| {
                                stderr.write(b"invalid integer given.\n").unwrap();
                                stderr.flush().unwrap();
                                process::exit(1);
                            }),
                    );
                } else {
                    stderr.write(b"you may only input one height.\n").unwrap();
                    stderr.flush().unwrap();
                    process::exit(1);
                }
            }
            "-c" | "--width" => {
                if width.is_none() {
                    width = Some(
                        args.next()
                            .unwrap_or_else(|| {
                                stderr.write(b"no width given.\n").unwrap();
                                stderr.flush().unwrap();
                                process::exit(1);
                            })
                            .parse()
                            .unwrap_or_else(|_| {
                                stderr.write(b"invalid integer given.\n").unwrap();
                                stderr.flush().unwrap();
                                process::exit(1);
                            }),
                    );
                } else {
                    stderr.write(b"you may only input one width.\n").unwrap();
                    stderr.flush().unwrap();
                    process::exit(1);
                }
            }
            "-h" | "--help" => {
                // Print the help page.
                conout().write(HELP.as_bytes()).unwrap();
                conout().flush().unwrap();
                process::exit(0);
            }
            "-s" | "--seed" => {
                seed = args
                    .next()
                    .unwrap_or_else(|| {
                        stderr.write(b"no seed given.\n").unwrap();
                        stderr.flush().unwrap();
                        process::exit(1);
                    })
                    .parse()
                    .unwrap_or_else(|_| {
                        stderr.write(b"invalid integer for seed given.\n").unwrap();
                        stderr.flush().unwrap();
                        process::exit(1);
                    });
            }
            "-g" => diff = 2,
            "-a" => diff = 4,
            "-i" => diff = 6,
            "-b" => diff = 10,
            _ => {
                stderr.write(b"Unknown argument.\n").unwrap();
                stderr.flush().unwrap();
                process::exit(1);
            }
        }
    }

    let termsize = sl_console::terminal_size().ok();
    let termwidth = termsize.map(|(w, _)| w - 2);
    let termheight = termsize.map(|(_, h)| h - 2);
    // Initialize the game!
    init(
        diff,
        width.or(termwidth).unwrap_or(70),
        height.or(termheight).unwrap_or(40),
        seed,
    );
}

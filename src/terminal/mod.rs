//! The terminal mode: the screen a device draws with escape sequences, as a terminal shows it, and
//! the bytes the keys typed become for the device. The screen is vt100's; RSC adds what it leaves
//! out: the answers to the device's questions (where the cursor is, how big the screen is), and the
//! character set of lines and corners that menus pick with ESC ( 0.

mod answers;
mod keys;
mod line_drawing;
#[cfg(test)]
mod tests;

pub use keys::{Key, Modifiers, key, pasted, typed};
pub use line_drawing::LINE_DRAWING;

use std::num::NonZeroU16;
use answers::Answers;
use line_drawing::LineDrawing;

/// The rows of history a terminal keeps at most, whatever the profile says.
pub const LONGEST_HISTORY: usize = 10_000;

/// The modes the device set that change what a key sends.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Modes {
    /// The arrows send ESC O A rather than ESC [ A (DECCKM), as vi and less ask.
    pub application_cursor: bool,
    /// What is pasted comes between ESC [ 200 ~ and ESC [ 201 ~, as shells ask.
    pub bracketed_paste: bool,
}

/// A terminal: the screen of a device and the rows that scrolled off it.
pub struct Terminal {
    parser: vt100::Parser<Answers>,
    drawing: LineDrawing,
    /// The start of a character cut at the end of the last read, given to vt100 with its end.
    cut: Vec<u8>,
}

/// How many bytes of `bytes` end with a whole character: all of them, but the start of a UTF-8
/// character whose end has not come yet.
///
/// vte 0.15.0, under vt100, loses a character when one comes cut between two reads: it reads up to
/// three bytes more to end it, prints only the cut one, and counts as read every whole character
/// among them ("arrê|tées" became "arrêées", on the test board, 18/11/2026). Given whole characters
/// only, vte never takes that path.
fn whole(bytes: &[u8]) -> usize {
    // The last character starts at most three bytes before the end.
    for back in 1..=bytes.len().min(4) {
        let byte = bytes[bytes.len() - back];
        if byte & 0xc0 == 0x80 {
            // A byte in the middle of a character: its start is further back.
            continue;
        }
        let length = match byte {
            0xc0..=0xdf => 2,
            0xe0..=0xef => 3,
            0xf0..=0xf7 => 4,
            // A byte of its own, or not a start: nothing to wait for.
            _ => return bytes.len(),
        };
        return if back < length { bytes.len() - back } else { bytes.len() };
    }
    bytes.len()
}

/// A size for vt100, which takes no zero.
fn non_zero(n: u16) -> NonZeroU16 {
    NonZeroU16::new(n).unwrap_or(NonZeroU16::MIN)
}

impl Terminal {
    /// A blank screen of `rows` by `cols`, keeping `history` rows that scroll off, up to
    /// `LONGEST_HISTORY`.
    pub fn new(rows: u16, cols: u16, history: usize) -> Self {
        Self {
            parser: vt100::Parser::new_with_callbacks(non_zero(rows), non_zero(cols), history.min(LONGEST_HISTORY), Answers::default()),
            drawing: LineDrawing::default(),
            cut: Vec::new(),
        }
    }

    /// What the device sent, as it comes: a sequence or a character may be cut anywhere. Gives back
    /// the answers to its questions, to be sent to it.
    pub fn receive(&mut self, bytes: &[u8]) -> Vec<u8> {
        let mut bytes = bytes;
        let joined;
        if !self.cut.is_empty() {
            let mut cut = std::mem::take(&mut self.cut);
            cut.extend_from_slice(bytes);
            joined = cut;
            bytes = &joined;
        }
        let (now, later) = bytes.split_at(whole(bytes));
        self.cut = later.to_vec();
        let mut filtered = Vec::with_capacity(now.len());
        self.drawing.filter(now, &mut filtered);
        self.parser.process(&filtered);
        std::mem::take(&mut self.parser.callbacks_mut().bytes)
    }

    /// The screen at a new size, in characters; the device is not told, a serial link cannot.
    pub fn resize(&mut self, rows: u16, cols: u16) {
        if self.size() != (rows.max(1), cols.max(1)) {
            self.parser.set_size(non_zero(rows), non_zero(cols));
        }
    }

    /// Rows, then columns.
    pub fn size(&self) -> (u16, u16) {
        let (rows, cols) = self.parser.screen().size();
        (rows.get(), cols.get())
    }

    pub fn screen(&self) -> &vt100::Screen {
        self.parser.screen()
    }

    pub fn modes(&self) -> Modes {
        let screen = self.parser.screen();
        Modes { application_cursor: screen.application_cursor(), bracketed_paste: screen.bracketed_paste() }
    }

    /// How many rows of history are above what is shown; 0 when the screen itself is shown.
    pub fn scrolled_back(&self) -> usize {
        self.parser.screen().scrollback()
    }

    /// Shows `rows` rows of history above the screen, as many as there are at most.
    pub fn scroll_back(&mut self, rows: usize) {
        self.parser.screen_mut().set_scrollback(rows);
    }

    /// The rows of history kept.
    pub fn history(&self) -> usize {
        self.parser.screen().scrollback_len()
    }

    /// The screen and its history emptied; the modes the device set stay.
    pub fn clear(&mut self) {
        self.parser.process(b"\x1b[H\x1b[2J");
        self.parser.screen_mut().clear_scrollback();
    }

    /// The text shown from one cell to another, in either order: rows, then columns, the last one
    /// left out.
    pub fn text_between(&self, a: (u16, u16), b: (u16, u16)) -> String {
        let ((row, col), (end_row, end_col)) = if a <= b { (a, b) } else { (b, a) };
        self.parser.screen().contents_between(row, col, end_row, end_col)
    }

    /// The word around a cell, as its first column and the column after it: letters, digits and
    /// what paths and addresses are made of.
    pub fn word_at(&self, row: u16, col: u16) -> Option<(u16, u16)> {
        let screen = self.parser.screen();
        let (_, cols) = self.size();
        let in_word = |col: u16| screen.cell(row, col).is_some_and(|cell| cell.contents().chars().next().is_some_and(|c| c.is_alphanumeric() || "_-./:@~".contains(c)));
        if !in_word(col) {
            return None;
        }
        let start = (0..col).rev().find(|&before| !in_word(before)).map_or(0, |before| before + 1);
        let end = (col..cols).find(|&after| !in_word(after)).unwrap_or(cols);
        Some((start, end))
    }
}

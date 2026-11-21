//! The answers to the device's questions, which vt100 leaves out: where the cursor is, how big
//! the screen is.

use std::fmt::Write as _;

/// What the device asked and RSC answers, gathered while its bytes are read.
#[derive(Default)]
pub(super) struct Answers {
    pub(super) bytes: Vec<u8>,
}

impl vt100::Callbacks for Answers {
    fn unhandled_csi(&mut self, screen: &mut vt100::Screen, i1: Option<u8>, _i2: Option<u8>, params: &[&[u16]], c: char) {
        let first = params.first().and_then(|param| param.first()).copied();
        match (i1, c, first) {
            // Where the cursor is: how `resize` and line editors learn the size of the screen.
            (None, 'n', Some(6)) => {
                let (row, col) = screen.cursor_position();
                let _ = write!(Text(&mut self.bytes), "\x1b[{};{}R", row + 1, col + 1);
            }
            // Whether the terminal is well.
            (None, 'n', Some(5)) => self.bytes.extend_from_slice(b"\x1b[0n"),
            // What the terminal is: a VT100 with its advanced video.
            (None, 'c', None | Some(0)) => self.bytes.extend_from_slice(b"\x1b[?1;2c"),
            // How big the screen is, in characters.
            (None, 't', Some(18)) => {
                let (rows, cols) = screen.size();
                let _ = write!(Text(&mut self.bytes), "\x1b[8;{rows};{cols}t");
            }
            _ => {}
        }
    }
}

/// Bytes written as text.
struct Text<'a>(&'a mut Vec<u8>);

impl std::fmt::Write for Text<'_> {
    fn write_str(&mut self, text: &str) -> std::fmt::Result {
        self.0.extend_from_slice(text.as_bytes());
        Ok(())
    }
}

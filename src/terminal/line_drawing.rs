//! The character set of lines and corners that menus pick with ESC ( 0, which vt100 leaves out:
//! its letters drawn as lines and corners before vt100 reads them.

/// Where the filter of the line-drawing set stands in what the device sends.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum Within {
    #[default]
    Text,
    /// After ESC.
    Escape,
    /// After ESC ( or ESC ): the set given to G0 or G1 comes next.
    Designate { g1: bool },
    /// After ESC and an intermediate byte other than ( and ).
    Intermediate,
    /// Inside ESC [ ... until its final byte.
    Csi,
    /// Inside ESC ] ... (or ESC P, ESC X, ESC ^, ESC _) until BEL or ESC \; whether an ESC came last.
    ControlString { escape: bool },
}

/// The character sets vt100 leaves out: the device gives G0 or G1 the set of lines and corners
/// (ESC ( 0, ESC ) 0) and shifts between them (SI, SO). Their letters from ` to ~ are then drawn as
/// the characters of that set, before vt100 reads them. What escape sequences hold is left as it is.
#[derive(Clone, Copy, Debug, Default)]
pub(super) struct LineDrawing {
    within: Within,
    /// Whether G0 and G1 hold the line-drawing set, and whether G1 is in use.
    g0: bool,
    g1: bool,
    shifted: bool,
}

/// The characters the line-drawing set draws for its letters ` to ~, as xterm draws them: lines and
/// corners from j to x.
pub const LINE_DRAWING: [char; 31] =
    ['◆', '▒', '␉', '␌', '␍', '␊', '°', '±', '␤', '␋', '┘', '┐', '┌', '└', '┼', '⎺', '⎻', '─', '⎼', '⎽', '├', '┤', '┴', '┬', '│', '≤', '≥', 'π', '≠', '£', '·'];

/// A letter of the line-drawing set, as the character it draws.
pub(super) fn drawn(byte: u8) -> Option<char> {
    byte.checked_sub(b'`').and_then(|index| LINE_DRAWING.get(usize::from(index))).copied()
}

impl LineDrawing {
    /// The bytes as vt100 is to read them: the letters of the line-drawing set drawn. Sequences cut
    /// between two reads are followed across them.
    pub(super) fn filter(&mut self, bytes: &[u8], out: &mut Vec<u8>) {
        for &byte in bytes {
            match self.within {
                Within::Text => match byte {
                    0x1b => self.within = Within::Escape,
                    // SO and SI: G1, then G0 again.
                    0x0e => self.shifted = true,
                    0x0f => self.shifted = false,
                    _ => {
                        let set = if self.shifted { self.g1 } else { self.g0 };
                        if let Some(c) = drawn(byte).filter(|_| set) {
                            let mut buffer = [0; 4];
                            out.extend_from_slice(c.encode_utf8(&mut buffer).as_bytes());
                            continue;
                        }
                    }
                },
                Within::Escape => {
                    self.within = match byte {
                        b'(' => Within::Designate { g1: false },
                        b')' => Within::Designate { g1: true },
                        b'[' => Within::Csi,
                        b']' | b'P' | b'X' | b'^' | b'_' => Within::ControlString { escape: false },
                        0x1b => Within::Escape,
                        0x20..=0x2f => Within::Intermediate,
                        _ => Within::Text,
                    };
                }
                Within::Designate { g1 } => {
                    let lines = byte == b'0';
                    if g1 {
                        self.g1 = lines;
                    } else {
                        self.g0 = lines;
                    }
                    self.within = Within::Text;
                }
                Within::Intermediate => {
                    if !(0x20..=0x2f).contains(&byte) {
                        self.within = if byte == 0x1b { Within::Escape } else { Within::Text };
                    }
                }
                Within::Csi => match byte {
                    0x40..=0x7e => self.within = Within::Text,
                    0x1b => self.within = Within::Escape,
                    // CAN and SUB end a sequence.
                    0x18 | 0x1a => self.within = Within::Text,
                    _ => {}
                },
                Within::ControlString { escape } => match byte {
                    0x07 => self.within = Within::Text,
                    b'\\' if escape => self.within = Within::Text,
                    0x18 | 0x1a => self.within = Within::Text,
                    _ => self.within = Within::ControlString { escape: byte == 0x1b },
                },
            }
            out.push(byte);
        }
    }
}

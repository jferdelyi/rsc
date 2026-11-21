//! The line to send: the bytes it becomes, and the lines sent before it.

use crate::settings::LineEnding;

/// Why a line with escapes cannot be sent.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EscapeError {
    /// A backslash followed by something that is not an escape: the character after it.
    Unknown(char),
    /// `\x` without its two hexadecimal digits.
    Hex,
    /// A backslash that ends the line.
    Dangling,
}

/// The bytes of `text` followed by its line ending. With `escapes`, `\n`, `\r`, `\t`, `\0`, `\e`,
/// `\\` and `\xNN` are read as the byte they name: `\x03` is Ctrl+C.
pub fn payload(text: &str, ending: LineEnding, escapes: bool) -> Result<Vec<u8>, EscapeError> {
    let mut bytes = if escapes { unescape(text)? } else { text.as_bytes().to_vec() };
    bytes.extend_from_slice(match ending {
        LineEnding::None => b"",
        LineEnding::Lf => b"\n",
        LineEnding::Cr => b"\r",
        LineEnding::CrLf => b"\r\n",
    });
    Ok(bytes)
}

fn unescape(text: &str) -> Result<Vec<u8>, EscapeError> {
    let mut bytes = Vec::with_capacity(text.len());
    let mut chars = text.chars();
    while let Some(c) = chars.next() {
        if c != '\\' {
            bytes.extend_from_slice(c.encode_utf8(&mut [0; 4]).as_bytes());
            continue;
        }
        bytes.push(match chars.next() {
            Some('n') => b'\n',
            Some('r') => b'\r',
            Some('t') => b'\t',
            Some('0') => 0,
            Some('e') => 0x1b,
            Some('\\') => b'\\',
            Some('x') => {
                let digits: String = chars.by_ref().take(2).collect();
                if digits.len() != 2 {
                    return Err(EscapeError::Hex);
                }
                u8::from_str_radix(&digits, 16).map_err(|_| EscapeError::Hex)?
            }
            Some(other) => return Err(EscapeError::Unknown(other)),
            None => return Err(EscapeError::Dangling),
        });
    }
    Ok(bytes)
}

/// The bytes written in hexadecimal: "48 65 6C", "0x48,0x65", "48656C" alike. The piece that is not
/// hexadecimal, or not whole bytes, when there is one.
pub fn parse_hex(text: &str) -> Result<Vec<u8>, String> {
    let mut bytes = Vec::new();
    for piece in text.split(|c: char| c.is_whitespace() || c == ',' || c == ';').filter(|piece| !piece.is_empty()) {
        let digits = piece.strip_prefix("0x").or_else(|| piece.strip_prefix("0X")).unwrap_or(piece);
        if digits.is_empty() || digits.len() % 2 != 0 || !digits.chars().all(|c| c.is_ascii_hexdigit()) {
            return Err(piece.to_owned());
        }
        for pair in digits.as_bytes().chunks(2) {
            // Two ASCII hexadecimal digits: always a byte.
            bytes.push(u8::from_str_radix(std::str::from_utf8(pair).unwrap_or("00"), 16).unwrap_or(0));
        }
    }
    Ok(bytes)
}

/// The lines sent, the latest last, walked with the up and down arrows.
#[derive(Default)]
pub struct History {
    lines: Vec<String>,
    /// The line shown, when walking.
    at: Option<usize>,
    /// What was being typed when the walk began.
    draft: String,
}

impl History {
    const KEPT: usize = 100;

    /// Remembers a line that was sent. The same line twice in a row counts once.
    pub fn push(&mut self, line: &str) {
        self.at = None;
        self.draft.clear();
        if line.is_empty() || self.lines.last().is_some_and(|last| last == line) {
            return;
        }
        self.lines.push(line.to_owned());
        if self.lines.len() > Self::KEPT {
            self.lines.remove(0);
        }
    }

    /// The line sent before the one shown, `typed` being what the field holds now.
    pub fn older(&mut self, typed: &str) -> Option<&str> {
        let at = match self.at {
            None => {
                typed.clone_into(&mut self.draft);
                self.lines.len().checked_sub(1)?
            }
            Some(at) => at.checked_sub(1)?,
        };
        self.at = Some(at);
        Some(&self.lines[at])
    }

    /// The line sent after the one shown; past the latest, what was being typed.
    pub fn newer(&mut self) -> Option<&str> {
        let at = self.at? + 1;
        if at < self.lines.len() {
            self.at = Some(at);
            Some(&self.lines[at])
        } else {
            self.at = None;
            Some(&self.draft)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_line_ending_follows_the_text() {
        assert_eq!(payload("AT", LineEnding::None, false).unwrap(), b"AT");
        assert_eq!(payload("AT", LineEnding::Lf, false).unwrap(), b"AT\n");
        assert_eq!(payload("AT", LineEnding::Cr, false).unwrap(), b"AT\r");
        assert_eq!(payload("AT", LineEnding::CrLf, false).unwrap(), b"AT\r\n");
        assert_eq!(payload("", LineEnding::Lf, false).unwrap(), b"\n", "an empty line still sends its ending");
    }

    #[test]
    fn text_goes_out_as_utf8() {
        assert_eq!(payload("été", LineEnding::None, false).unwrap(), "été".as_bytes());
    }

    #[test]
    fn a_backslash_is_a_backslash_unless_escapes_are_read() {
        assert_eq!(payload(r"a\nb", LineEnding::None, false).unwrap(), br"a\nb");
        assert_eq!(payload(r"a\nb", LineEnding::None, true).unwrap(), b"a\nb");
    }

    #[test]
    fn escapes_name_their_byte() {
        assert_eq!(payload(r"\x03", LineEnding::None, true).unwrap(), [0x03]);
        assert_eq!(payload(r"\r\n\t\0\e\\", LineEnding::None, true).unwrap(), [b'\r', b'\n', b'\t', 0, 0x1b, b'\\']);
        assert_eq!(payload(r"\xFFé\x41", LineEnding::Lf, true).unwrap(), [0xff, 0xc3, 0xa9, b'A', b'\n']);
    }

    #[test]
    fn an_escape_that_is_not_one_is_refused() {
        assert_eq!(payload(r"a\qb", LineEnding::Lf, true), Err(EscapeError::Unknown('q')));
        assert_eq!(payload(r"\x4", LineEnding::Lf, true), Err(EscapeError::Hex));
        assert_eq!(payload(r"\xZZ", LineEnding::Lf, true), Err(EscapeError::Hex));
        assert_eq!(payload("fin\\", LineEnding::Lf, true), Err(EscapeError::Dangling));
    }

    #[test]
    fn hexadecimal_is_read_with_or_without_separators() {
        assert_eq!(parse_hex("48 65 6C 6c 6F"), Ok(b"Hello".to_vec()));
        assert_eq!(parse_hex("0x48,0x69  0X0d;0a"), Ok(b"Hi\r\n".to_vec()));
        assert_eq!(parse_hex("DEADbeef"), Ok(vec![0xde, 0xad, 0xbe, 0xef]));
        assert_eq!(parse_hex("  "), Ok(Vec::new()));
    }

    #[test]
    fn a_piece_that_is_not_whole_bytes_is_named() {
        assert_eq!(parse_hex("48 6"), Err("6".into()));
        assert_eq!(parse_hex("48 0xZZ"), Err("0xZZ".into()));
        assert_eq!(parse_hex("0x"), Err("0x".into()));
        assert_eq!(parse_hex("hello"), Err("hello".into()));
    }

    #[test]
    fn the_arrows_walk_the_lines_sent_and_come_back_to_the_draft() {
        let mut history = History::default();
        assert_eq!(history.older("brouillon"), None, "nothing sent yet");
        history.push("status");
        history.push("read temp");
        history.push("read temp");
        history.push("");
        assert_eq!(history.older("rea"), Some("read temp"));
        assert_eq!(history.older("read temp"), Some("status"));
        assert_eq!(history.older("status"), None, "the oldest stays");
        assert_eq!(history.newer(), Some("read temp"));
        assert_eq!(history.newer(), Some("rea"), "what was being typed comes back");
        assert_eq!(history.newer(), None);
    }

    #[test]
    fn only_the_latest_lines_are_kept() {
        let mut history = History::default();
        for n in 0..History::KEPT + 5 {
            history.push(&format!("line {n}"));
        }
        assert_eq!(history.lines.len(), History::KEPT);
        assert_eq!(history.lines[0], "line 5");
    }
}

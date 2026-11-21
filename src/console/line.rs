//! A line of the console and what it shows: its kind, its colours, its text or its bytes in
//! hexadecimal, its time, how wide it is and how it folds.

use super::{TIME_WIDTH, Time};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    /// From the device.
    Received,
    /// Sent to the device, shown after "> ".
    Sent,
    /// From the tool itself, shown after "# ".
    Notice,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Color {
    Default,
    /// One of the 16 colours of a terminal: 0 to 7, then their bright forms.
    Ansi(u8),
}

/// The colour of a line's text from `start`, a character index, to the next span.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Span {
    pub start: usize,
    pub color: Color,
}

/// How lines are shown: with their time or not, as text or as the bytes they were.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Shown {
    pub timestamps: bool,
    pub hex: bool,
}

/// How a line folds when it is wider than the console (see `Wrap`): its width in characters, the
/// characters before its body (its time, "> "), and the characters the body is made of, three for a
/// byte in hexadecimal, so that a byte is never cut in two.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Fold {
    pub width: usize,
    pub body: usize,
    pub unit: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Line {
    pub kind: Kind,
    pub time: Time,
    text: String,
    /// The length of `text` in characters.
    pub(super) chars: usize,
    spans: Vec<Span>,
    /// The bytes the line came from, its end and escape sequences included; what was sent, for a
    /// line sent.
    pub(super) raw: Vec<u8>,
}

impl Line {
    pub(super) fn new(kind: Kind, time: Time, text: &str, raw: Vec<u8>) -> Self {
        Self { kind, time, text: text.to_owned(), chars: text.chars().count(), spans: Vec::new(), raw }
    }

    pub fn text(&self) -> &str {
        &self.text
    }

    pub fn spans(&self) -> &[Span] {
        &self.spans
    }

    pub fn raw(&self) -> &[u8] {
        &self.raw
    }

    /// Whether the line shows its bytes: a notice is never shown in hexadecimal.
    pub fn in_hex(&self, shown: Shown) -> bool {
        shown.hex && self.kind != Kind::Notice
    }

    /// What comes before the body in hexadecimal: "> " for a line sent.
    fn hex_prefix(&self) -> &'static str {
        if self.kind == Kind::Sent { "> " } else { "" }
    }

    /// The characters the line takes on screen.
    pub fn width(&self, shown: Shown) -> usize {
        let time = if shown.timestamps { TIME_WIDTH } else { 0 };
        let body = if self.in_hex(shown) { self.hex_prefix().len() + hex_width(self.raw.len()) } else { self.chars };
        time + body
    }

    pub fn fold(&self, shown: Shown) -> Fold {
        let time = if shown.timestamps { TIME_WIDTH } else { 0 };
        if self.in_hex(shown) {
            Fold { width: self.width(shown), body: time + self.hex_prefix().len(), unit: 3 }
        } else {
            Fold { width: self.width(shown), body: time, unit: 1 }
        }
    }

    /// The line as shown, and as copied or saved: its time first when asked for.
    pub fn display(&self, shown: Shown) -> String {
        let body = if self.in_hex(shown) { format!("{}{}", self.hex_prefix(), hex(&self.raw)) } else { self.text.clone() };
        if shown.timestamps { format!("{} {body}", self.time.format("%H:%M:%S%.3f")) } else { body }
    }

    pub(super) fn push(&mut self, c: char, color: Color) {
        if self.spans.last().map_or(Color::Default, |span| span.color) != color {
            self.spans.push(Span { start: self.chars, color });
        }
        self.text.push(c);
        self.chars += 1;
    }

    pub(super) fn pop(&mut self) {
        if self.text.pop().is_some() {
            self.chars -= 1;
            self.spans.retain(|span| span.start < self.chars);
        }
    }
}

/// Bytes in hexadecimal, a space between two: "48 65 0D 0A".
pub fn hex(bytes: &[u8]) -> String {
    const DIGITS: &[u8; 16] = b"0123456789ABCDEF";
    let mut text = String::with_capacity(bytes.len() * 3);
    for (i, &byte) in bytes.iter().enumerate() {
        if i > 0 {
            text.push(' ');
        }
        text.push(char::from(DIGITS[usize::from(byte >> 4)]));
        text.push(char::from(DIGITS[usize::from(byte & 0x0f)]));
    }
    text
}

fn hex_width(bytes: usize) -> usize {
    (bytes * 3).saturating_sub(1)
}

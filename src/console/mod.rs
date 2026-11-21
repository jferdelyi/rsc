//! The console's text: what the device sent, cut into lines as it arrives, with the lines sent to it
//! and the tool's own notices in between. Each line keeps the bytes it came from, for the
//! hexadecimal view. Nothing here draws: the window reads the lines.

mod decode;
mod line;
#[cfg(test)]
mod tests;
mod wrap;

use std::collections::VecDeque;

use chrono::NaiveDateTime;

use decode::Escape;

pub use line::{Color, Fold, Kind, Line, Shown, Span, hex};
pub use wrap::Wrap;

/// When a line began, in local time.
pub type Time = NaiveDateTime;

/// The width of a line's time as shown, "10:15:22.104 ".
pub const TIME_WIDTH: usize = 13;

/// A line without an end is cut there: a wrong baud rate sends garbage with no newline.
const LONGEST_LINE: usize = 4096;

/// A place in the console: a line by its number since the tool started, and a character of that
/// line as shown. Lines dropped from the top keep their number.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Position {
    pub line: u64,
    pub col: usize,
}

pub struct Console {
    lines: VecDeque<Line>,
    /// The number of the oldest line kept.
    first: u64,
    limit: usize,
    /// Changes with the text: what follows it knows when to look again.
    revision: u64,
    /// Whether the last line is one from the device that has not ended.
    open: bool,
    /// A carriage return ended the last line: the line feed that follows it belongs to it.
    after_cr: bool,
    /// The start of a character cut by the read.
    undecoded: Vec<u8>,
    /// Bytes received before the line they belong to has begun: an escape sequence, mostly.
    raw_pending: Vec<u8>,
    escape: Escape,
    color: Color,
    bold: bool,
    /// The lines that ended since `take_ended`, when someone follows them.
    ended: Option<Vec<Line>>,
}

impl Console {
    pub fn new(limit: usize) -> Self {
        Self {
            lines: VecDeque::new(),
            first: 0,
            limit: limit.max(1),
            revision: 0,
            open: false,
            after_cr: false,
            undecoded: Vec::new(),
            raw_pending: Vec::new(),
            escape: Escape::None,
            color: Color::Default,
            bold: false,
            ended: None,
        }
    }

    pub fn lines(&self) -> &VecDeque<Line> {
        &self.lines
    }

    /// The number of the oldest line kept.
    pub fn first(&self) -> u64 {
        self.first
    }

    pub fn revision(&self) -> u64 {
        self.revision
    }

    pub fn is_empty(&self) -> bool {
        self.lines.is_empty()
    }

    pub fn set_limit(&mut self, limit: usize) {
        self.limit = limit.max(1);
        self.trim();
    }

    /// A line sent to the device, shown on a line of its own: as typed, and as the bytes that went.
    pub fn sent(&mut self, text: &str, bytes: &[u8], now: Time) {
        self.insert(Line::new(Kind::Sent, now, &format!("> {text}"), bytes.to_vec()));
    }

    /// A word from the tool itself: connected, port lost.
    pub fn notice(&mut self, text: &str, now: Time) {
        self.insert(Line::new(Kind::Notice, now, &format!("# {text}"), Vec::new()));
    }

    pub fn clear(&mut self) {
        self.first += self.lines.len() as u64;
        self.lines.clear();
        self.open = false;
        self.after_cr = false;
        self.undecoded.clear();
        self.raw_pending.clear();
        self.escape = Escape::None;
        self.revision += 1;
    }

    /// Starts or stops keeping the lines as they end, for who uses them elsewhere (the journal, the
    /// curves). Asked again while it keeps them, it loses none.
    pub fn follow(&mut self, on: bool) {
        match (on, &self.ended) {
            (true, Some(_)) => {}
            (true, None) => self.ended = Some(Vec::new()),
            (false, _) => self.ended = None,
        }
    }

    /// The lines that ended since the last call, in order.
    pub fn take_ended(&mut self) -> Vec<Line> {
        self.ended.as_mut().map(std::mem::take).unwrap_or_default()
    }

    /// The line the device has begun and not ended, if any.
    pub fn open_line(&self) -> Option<&Line> {
        if self.open { self.lines.back() } else { None }
    }

    /// The whole console as shown, a line feed after each line.
    pub fn text(&self, shown: Shown) -> String {
        let mut text = String::new();
        for line in &self.lines {
            text.push_str(&line.display(shown));
            text.push('\n');
        }
        text
    }

    /// What is shown between two places, in either order: the end of a line counts as a line feed.
    pub fn text_between(&self, a: Position, b: Position, shown: Shown) -> String {
        let (from, to) = if a <= b { (a, b) } else { (b, a) };
        let mut text = String::new();
        for number in from.line.max(self.first)..=to.line {
            let Some(line) = self.line(number) else {
                break;
            };
            let display = line.display(shown);
            let start = if number == from.line { from.col } else { 0 };
            let end = if number == to.line { to.col } else { usize::MAX };
            text.extend(display.chars().skip(start).take(end.saturating_sub(start)));
            if number != to.line {
                text.push('\n');
            }
        }
        text
    }

    /// The word around a place, as two places: letters, digits and underscores that touch.
    pub fn word_at(&self, at: Position, shown: Shown) -> Option<(Position, Position)> {
        let display: Vec<char> = self.line(at.line)?.display(shown).chars().collect();
        let in_word = |c: &char| c.is_alphanumeric() || *c == '_';
        if !display.get(at.col).is_some_and(in_word) {
            return None;
        }
        let start = display[..at.col].iter().rposition(|c| !in_word(c)).map_or(0, |before| before + 1);
        let end = display[at.col..].iter().position(|c| !in_word(c)).map_or(display.len(), |after| at.col + after);
        Some((Position { line: at.line, col: start }, Position { line: at.line, col: end }))
    }

    /// The line of that number, if it is still kept.
    pub fn line(&self, number: u64) -> Option<&Line> {
        self.lines.get(usize::try_from(number.checked_sub(self.first)?).ok()?)
    }

    /// The place after the last character shown.
    pub fn end(&self, shown: Shown) -> Position {
        match self.lines.back() {
            Some(last) => Position { line: self.first + self.lines.len() as u64 - 1, col: last.width(shown) },
            None => Position { line: self.first, col: 0 },
        }
    }

    fn insert(&mut self, line: Line) {
        // What the device had begun stays above, as it was.
        self.close();
        self.lines.push_back(line);
        self.hand_last();
        self.changed();
    }

    fn changed(&mut self) {
        self.revision += 1;
        self.trim();
    }

    fn trim(&mut self) {
        while self.lines.len() > self.limit {
            self.lines.pop_front();
            self.first += 1;
        }
    }

    /// Ends the line the device was writing, if any.
    fn close(&mut self) {
        if std::mem::take(&mut self.open) {
            self.hand_last();
        }
    }

    /// Hands the last line, which just ended, to who follows.
    fn hand_last(&mut self) {
        if let (Some(ended), Some(line)) = (&mut self.ended, self.lines.back()) {
            ended.push(line.clone());
        }
    }
}

#[cfg(test)]
pub(crate) fn at(seconds: u32, millis: u32) -> Time {
    chrono::NaiveDate::from_ymd_opt(2026, 11, 17).unwrap().and_hms_milli_opt(10, 15, seconds, millis).unwrap()
}

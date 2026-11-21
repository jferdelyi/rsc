//! The bytes received, made lines: characters put back together across reads, line ends, tabs,
//! ANSI colours kept as spans, other escape sequences taken out, a line too long cut.

use super::{Color, Console, Kind, LONGEST_LINE, Line, Time};

/// Where an escape sequence stands.
pub(super) enum Escape {
    None,
    /// After ESC.
    Started,
    /// After ESC and the characters that announce a longer sequence.
    Intermediate,
    /// After ESC [, its parameters so far.
    Csi(String),
    /// After ESC ], until BEL or ESC \: its length, and whether an ESC came last.
    Osc(usize, bool),
}

impl Console {
    /// What the device sent, as it comes: a line, a character or an escape sequence may be cut
    /// anywhere, its end comes with a later call.
    pub fn receive(&mut self, bytes: &[u8], now: Time) {
        if bytes.is_empty() {
            return;
        }
        let mut pending = std::mem::take(&mut self.undecoded);
        pending.extend_from_slice(bytes);
        let mut rest = &pending[..];
        while !rest.is_empty() {
            let (valid, error) = match std::str::from_utf8(rest) {
                Ok(text) => (text, None),
                Err(error) => (std::str::from_utf8(&rest[..error.valid_up_to()]).unwrap_or_default(), Some(error)),
            };
            for (at, c) in valid.char_indices() {
                self.put(c, &valid.as_bytes()[at..at + c.len_utf8()], now);
            }
            rest = &rest[valid.len()..];
            match error.map(|error| error.error_len()) {
                None => {}
                // Not UTF-8: a wrong baud rate, or a device that speaks bytes.
                Some(Some(len)) => {
                    self.put(char::REPLACEMENT_CHARACTER, &rest[..len], now);
                    rest = &rest[len..];
                }
                // A character cut by the read.
                Some(None) => {
                    self.undecoded = rest.to_vec();
                    rest = &[];
                }
            }
        }
        self.changed();
    }

    /// The colour of what the device writes now: bold shows as the bright form of a colour.
    fn ink(&self) -> Color {
        match (self.bold, self.color) {
            (true, Color::Ansi(index @ 0..=7)) => Color::Ansi(index + 8),
            (_, color) => color,
        }
    }

    /// Bytes of the line being written, or of the next one when none is.
    fn keep_raw(&mut self, raw: &[u8]) {
        match self.lines.back_mut() {
            Some(line) if self.open => line.raw.extend_from_slice(raw),
            _ => self.raw_pending.extend_from_slice(raw),
        }
    }

    /// Bytes of the line that just ended: the line feed of a CR LF.
    fn keep_raw_in_last(&mut self, raw: &[u8]) {
        match self.lines.back_mut() {
            Some(line) if line.kind == Kind::Received => line.raw.extend_from_slice(raw),
            _ => self.raw_pending.extend_from_slice(raw),
        }
    }

    /// One character from the device, and the bytes it came from.
    fn put(&mut self, c: char, raw: &[u8], now: Time) {
        // A line end's bytes belong to the line it ends: `plain` keeps them.
        if !matches!(c, '\n' | '\r') {
            self.keep_raw(raw);
        }
        match std::mem::replace(&mut self.escape, Escape::None) {
            Escape::None => self.plain(c, raw, now),
            Escape::Started | Escape::Intermediate => match c {
                '\u{1b}' => self.escape = Escape::Started,
                '[' => self.escape = Escape::Csi(String::new()),
                ']' => self.escape = Escape::Osc(0, false),
                // A character set, "ESC ( B": its last character is still to come.
                '\u{20}'..='\u{2f}' => self.escape = Escape::Intermediate,
                c if c.is_control() => self.plain(c, raw, now),
                // The last character of a short sequence: nothing to show.
                _ => {}
            },
            Escape::Csi(mut parameters) => match c {
                '\u{1b}' => self.escape = Escape::Started,
                'm' => self.colors(&parameters),
                // Any other final character: a cursor move, an erase. Nothing to show in a line.
                '\u{40}'..='\u{7e}' => {}
                // Not a sequence after all: a line end must not be swallowed.
                c if c.is_control() => self.plain(c, raw, now),
                c if parameters.len() < 32 => {
                    parameters.push(c);
                    self.escape = Escape::Csi(parameters);
                }
                _ => {}
            },
            Escape::Osc(length, after_escape) => match c {
                '\u{7}' => {}
                '\\' if after_escape => {}
                '\n' | '\r' => self.plain(c, raw, now),
                // Garbage that looks like its start must not swallow what follows for ever.
                _ if length >= 256 => {}
                c => self.escape = Escape::Osc(length + 1, c == '\u{1b}'),
            },
        }
    }

    fn plain(&mut self, c: char, raw: &[u8], now: Time) {
        match c {
            '\u{1b}' => self.escape = Escape::Started,
            // CR LF is one line end, and so is each of them alone.
            '\n' if std::mem::take(&mut self.after_cr) => self.keep_raw_in_last(raw),
            '\n' => self.end_line(raw, now),
            // "CR CR LF", a line end converted twice by the device, is one line end as well.
            '\r' if self.after_cr => self.keep_raw_in_last(raw),
            '\r' => {
                self.end_line(raw, now);
                self.after_cr = true;
            }
            c => {
                self.after_cr = false;
                let color = self.ink();
                match c {
                    '\t' => {
                        let line = self.current(now);
                        for _ in 0..8 - line.chars % 8 {
                            line.push(' ', color);
                        }
                    }
                    '\u{8}' if self.open => {
                        if let Some(line) = self.lines.back_mut() {
                            line.pop();
                        }
                    }
                    c if c.is_control() => {}
                    c => {
                        let line = self.current(now);
                        line.push(c, color);
                        if line.chars >= LONGEST_LINE {
                            self.close();
                        }
                    }
                }
            }
        }
    }

    /// The line the device is writing, begun now if there is none, with the bytes that came before.
    fn current(&mut self, now: Time) -> &mut Line {
        if !self.open {
            let raw = std::mem::take(&mut self.raw_pending);
            self.lines.push_back(Line::new(Kind::Received, now, "", raw));
            self.open = true;
        }
        self.lines.back_mut().expect("a line was just opened")
    }

    fn end_line(&mut self, raw: &[u8], now: Time) {
        // An empty line is a line too.
        self.current(now).raw.extend_from_slice(raw);
        self.close();
    }

    /// The colours of a "select graphic rendition" sequence: the 16 colours of the text, bold as
    /// their bright form. Backgrounds and the rest are left out.
    fn colors(&mut self, parameters: &str) {
        let mut codes = parameters.split(';').map(|code| code.parse::<u16>().unwrap_or(0));
        while let Some(code) = codes.next() {
            match code {
                0 => (self.color, self.bold) = (Color::Default, false),
                1 => self.bold = true,
                22 => self.bold = false,
                30..=37 => self.color = Color::Ansi((code - 30) as u8),
                39 => self.color = Color::Default,
                90..=97 => self.color = Color::Ansi((code - 90 + 8) as u8),
                // 256 colours and true colours: their parameters are not codes of their own.
                38 | 48 => match codes.next() {
                    Some(5) => {
                        let index = codes.next().unwrap_or(0);
                        if code == 38 && index < 16 {
                            self.color = Color::Ansi(index as u8);
                        }
                    }
                    Some(2) => {
                        codes.nth(2);
                    }
                    _ => {}
                },
                _ => {}
            }
        }
    }
}

//! The bytes the keys typed become for the device, as xterm sends them: the keys that are not
//! text, with Shift, Alt and Ctrl; text typed and pasted.

use super::Modes;

/// A key that is not a character of text.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Key {
    Enter,
    Backspace,
    Tab,
    Escape,
    Up,
    Down,
    Right,
    Left,
    Home,
    End,
    Insert,
    Delete,
    PageUp,
    PageDown,
    /// F1 to F12.
    F(u8),
    /// A character with Ctrl or Alt held: Ctrl+C, Alt+F.
    Char(char),
}

/// The keys held with another.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Modifiers {
    pub shift: bool,
    pub alt: bool,
    pub ctrl: bool,
}

impl Modifiers {
    /// How xterm numbers them in a sequence: 1 for none.
    fn number(self) -> u8 {
        1 + u8::from(self.shift) + 2 * u8::from(self.alt) + 4 * u8::from(self.ctrl)
    }
}

/// The control character Ctrl gives with a character, if any: Ctrl+C is 0x03.
fn control(c: char) -> Option<u8> {
    match c.to_ascii_lowercase() {
        c @ 'a'..='z' => Some(c as u8 - b'a' + 1),
        '@' | ' ' | '2' => Some(0x00),
        '[' | '3' => Some(0x1b),
        '\\' | '4' => Some(0x1c),
        ']' | '5' => Some(0x1d),
        '^' | '6' => Some(0x1e),
        '_' | '7' | '-' => Some(0x1f),
        '?' | '8' => Some(0x7f),
        _ => None,
    }
}

/// What a key sends to the device, as xterm sends it; nothing for a key it has no bytes for.
pub fn key(key: Key, modifiers: Modifiers, modes: Modes) -> Option<Vec<u8>> {
    let alt = |bytes: &[u8]| if modifiers.alt { [b"\x1b".as_slice(), bytes].concat() } else { bytes.to_vec() };
    let number = modifiers.number();
    // A cursor key: ESC [ A, ESC O A in the application mode, ESC [ 1 ; 5 A with Ctrl.
    let cursor = |end: u8| {
        if number > 1 {
            format!("\x1b[1;{number}{}", end as char).into_bytes()
        } else if modes.application_cursor {
            vec![0x1b, b'O', end]
        } else {
            vec![0x1b, b'[', end]
        }
    };
    // A key of the editing pad: ESC [ 3 ~, ESC [ 3 ; 5 ~ with Ctrl.
    let tilde = |code: u8| if number > 1 { format!("\x1b[{code};{number}~").into_bytes() } else { format!("\x1b[{code}~").into_bytes() };
    Some(match key {
        Key::Enter => alt(b"\r"),
        Key::Backspace if modifiers.ctrl => alt(b"\x08"),
        Key::Backspace => alt(b"\x7f"),
        Key::Tab if modifiers.shift => b"\x1b[Z".to_vec(),
        Key::Tab => alt(b"\t"),
        Key::Escape => alt(b"\x1b"),
        Key::Up => cursor(b'A'),
        Key::Down => cursor(b'B'),
        Key::Right => cursor(b'C'),
        Key::Left => cursor(b'D'),
        Key::Home => cursor(b'H'),
        Key::End => cursor(b'F'),
        Key::Insert => tilde(2),
        Key::Delete => tilde(3),
        Key::PageUp => tilde(5),
        Key::PageDown => tilde(6),
        Key::F(n @ 1..=4) => {
            let end = b'P' + (n - 1);
            if number > 1 { format!("\x1b[1;{number}{}", end as char).into_bytes() } else { vec![0x1b, b'O', end] }
        }
        Key::F(n @ 5..=12) => tilde([15, 17, 18, 19, 20, 21, 23, 24][usize::from(n - 5)]),
        Key::F(_) => return None,
        Key::Char(c) if modifiers.ctrl => alt(&[control(c)?]),
        Key::Char(c) => alt(c.to_string().as_bytes()),
    })
}

/// Text typed: its bytes, after ESC when Alt is held.
pub fn typed(text: &str, alt: bool) -> Vec<u8> {
    let mut bytes = if alt { vec![0x1b] } else { Vec::new() };
    bytes.extend_from_slice(text.as_bytes());
    bytes
}

/// Text pasted: each line end as the Enter key sends it, and between ESC [ 200 ~ and ESC [ 201 ~
/// when the device asked for it, its end marker taken out of the text so that the text cannot end
/// the paste early.
pub fn pasted(text: &str, modes: Modes) -> Vec<u8> {
    let text = text.replace("\r\n", "\r").replace('\n', "\r");
    if modes.bracketed_paste {
        let text = text.replace("\x1b[201~", "");
        [b"\x1b[200~".as_slice(), text.as_bytes(), b"\x1b[201~"].concat()
    } else {
        text.into_bytes()
    }
}

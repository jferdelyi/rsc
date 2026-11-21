//! The tests of the terminal mode: text cut between two reads, the screen and its history, the
//! answers, the set of lines and corners, the keys, the modes, the selection, the size.

use super::*;
use super::line_drawing::drawn;

/// The rows of the screen as text, blanks at their end left out.
fn rows(terminal: &Terminal) -> Vec<String> {
    let (_, cols) = terminal.size();
    terminal.screen().rows(0, cols).map(|row| row.trim_end().to_owned()).collect()
}

fn plain() -> Modifiers {
    Modifiers::default()
}

/// What the test board wrote on the real link (18/11/2026), its "t" lost on the screen: the
/// sentence cut between two reads at every place, accents included, loses nothing.
#[test]
fn a_text_cut_between_two_reads_anywhere_loses_nothing() {
    let sent = "Mesures arrêtées après 42 lignes\r\n".as_bytes();
    for cut in 0..=sent.len() {
        let mut terminal = Terminal::new(4, 80, 10);
        terminal.receive(&sent[..cut]);
        terminal.receive(&sent[cut..]);
        assert_eq!(rows(&terminal)[0], "Mesures arrêtées après 42 lignes", "cut after {cut} bytes");
    }
}

/// Characters of two, three and four bytes, cut anywhere into three reads, or a byte a read.
#[test]
fn characters_of_any_length_come_whole_however_they_are_cut() {
    let text = "é─😀a€b";
    let sent = text.as_bytes();
    for first in 0..=sent.len() {
        for second in first..=sent.len() {
            let mut terminal = Terminal::new(2, 20, 0);
            terminal.receive(&sent[..first]);
            terminal.receive(&sent[first..second]);
            terminal.receive(&sent[second..]);
            assert_eq!(rows(&terminal)[0], text, "cut after {first} and {second} bytes");
        }
    }
    let mut terminal = Terminal::new(2, 20, 0);
    for byte in sent {
        terminal.receive(std::slice::from_ref(byte));
    }
    assert_eq!(rows(&terminal)[0], text, "a byte a read");
    // A start that never ends is not kept back for ever: the next byte comes, and the screen is
    // the one of the same bytes in a single read.
    let mut at_once = Terminal::new(2, 20, 0);
    at_once.receive(b"a\xc3b");
    let mut terminal = Terminal::new(2, 20, 0);
    terminal.receive(b"a\xc3");
    terminal.receive(b"b");
    assert_eq!(rows(&terminal)[0], rows(&at_once)[0]);
    assert!(rows(&terminal)[0].ends_with('b'), "{:?}", rows(&terminal)[0]);
    assert_eq!((whole(b"ab"), whole(b"a\xc3"), whole(b"a\xe2\x94"), whole(b"a\xe2\x94\x80"), whole(b"\x80\x80")), (2, 1, 1, 4, 2));
}

#[test]
fn what_the_device_draws_is_on_the_screen() {
    let mut terminal = Terminal::new(4, 20, 100);
    terminal.receive(b"first\r\nsecond\x1b[1;3Hx\x1b[2;1H\x1b[K\x1b[31mred");
    assert_eq!(rows(&terminal), ["fixst", "red", "", ""]);
    assert_eq!(terminal.screen().cell(1, 0).map(vt100::Cell::fgcolor), Some(vt100::Color::Idx(1)));
}

#[test]
fn the_rows_that_scroll_off_are_kept_and_shown_on_demand() {
    let mut terminal = Terminal::new(3, 10, 100);
    for n in 0..6 {
        terminal.receive(format!("line {n}\r\n").as_bytes());
    }
    assert_eq!(terminal.history(), 4);
    terminal.scroll_back(2);
    assert_eq!((terminal.scrolled_back(), rows(&terminal)[0].as_str()), (2, "line 2"));
    terminal.scroll_back(usize::MAX);
    assert_eq!(terminal.scrolled_back(), 4, "no further back than the history");
    terminal.clear();
    assert_eq!((terminal.history(), terminal.scrolled_back(), rows(&terminal)), (0, 0, vec![String::new(); 3]));
}

#[test]
fn the_history_is_kept_to_its_longest() {
    let terminal = Terminal::new(3, 10, 1_000_000);
    assert_eq!(terminal.screen().scrollback_capacity(), LONGEST_HISTORY);
}

#[test]
fn the_device_learns_where_the_cursor_is_and_how_big_the_screen_is() {
    let mut terminal = Terminal::new(24, 80, 0);
    assert_eq!(terminal.receive(b"\x1b[5;10H\x1b[6n"), b"\x1b[5;10R");
    assert_eq!(terminal.receive(b"\x1b[5n"), b"\x1b[0n");
    assert_eq!(terminal.receive(b"\x1b[c"), b"\x1b[?1;2c");
    assert_eq!(terminal.receive(b"\x1b[0c"), b"\x1b[?1;2c");
    assert_eq!(terminal.receive(b"\x1b[18t"), b"\x1b[8;24;80t");
    terminal.resize(30, 100);
    assert_eq!(terminal.receive(b"\x1b[18t"), b"\x1b[8;30;100t");
    // `resize`'s way: the cursor sent far away, then asked for.
    assert_eq!(terminal.receive(b"\x1b[999;999H\x1b[6n"), b"\x1b[30;100R");
    // Cut between two reads, asked once.
    assert!(terminal.receive(b"\x1b[").is_empty());
    assert_eq!(terminal.receive(b"6n"), b"\x1b[30;100R");
    assert!(terminal.receive(b"\x1b[>c\x1b[?6n\x1b[99n").is_empty(), "nothing to other questions");
}

#[test]
fn the_line_drawing_set_draws_lines_and_corners() {
    let mut terminal = Terminal::new(3, 20, 0);
    terminal.receive(b"\x1b(0lqqk\x1b(B lqk\r\n\x1b(0x  x\x1b(B\r\n\x1b(0mqqj");
    assert_eq!(rows(&terminal), ["┌──┐ lqk", "│  │", "└──┘"]);
}

#[test]
fn the_line_drawing_set_holds_across_reads_and_spares_the_sequences() {
    let mut terminal = Terminal::new(2, 20, 0);
    // The designation cut in two, a colour and a cursor shape whose last letters are not drawn.
    terminal.receive(b"\x1b(");
    terminal.receive(b"0q\x1b[1mq\x1b[2 qq\x1b]0;title q\x07q\x1b(Bq");
    assert_eq!(rows(&terminal)[0], "────q");
    assert!(terminal.screen().cell(0, 1).is_some_and(vt100::Cell::bold), "the colour sequence read as such");
    // G1 holds the set, SO and SI shift to it and back.
    terminal.receive(b"\r\n\x1b)0\x0eqx\x0fqx");
    assert_eq!(rows(&terminal)[1], "─│qx");
}

#[test]
fn each_line_drawing_letter_has_its_character() {
    let letters: Vec<char> = (b'`'..=b'~').filter_map(drawn).collect();
    assert_eq!(letters.len(), 31);
    assert!(drawn(b'_').is_none() && drawn(b'A').is_none());
}

#[test]
fn the_keys_send_what_xterm_sends() {
    let normal = Modes::default();
    let application = Modes { application_cursor: true, ..Modes::default() };
    let ctrl = Modifiers { ctrl: true, ..Modifiers::default() };
    let alt = Modifiers { alt: true, ..Modifiers::default() };
    let shift = Modifiers { shift: true, ..Modifiers::default() };
    let cases: [(Key, Modifiers, Modes, &[u8]); 24] = [
        (Key::Enter, plain(), normal, b"\r"),
        (Key::Backspace, plain(), normal, b"\x7f"),
        (Key::Backspace, ctrl, normal, b"\x08"),
        (Key::Tab, plain(), normal, b"\t"),
        (Key::Tab, shift, normal, b"\x1b[Z"),
        (Key::Escape, plain(), normal, b"\x1b"),
        (Key::Up, plain(), normal, b"\x1b[A"),
        (Key::Up, plain(), application, b"\x1bOA"),
        (Key::Left, ctrl, normal, b"\x1b[1;5D"),
        (Key::Right, ctrl, application, b"\x1b[1;5C"),
        (Key::Home, plain(), normal, b"\x1b[H"),
        (Key::End, plain(), application, b"\x1bOF"),
        (Key::Insert, plain(), normal, b"\x1b[2~"),
        (Key::Delete, plain(), normal, b"\x1b[3~"),
        (Key::Delete, shift, normal, b"\x1b[3;2~"),
        (Key::PageUp, plain(), normal, b"\x1b[5~"),
        (Key::PageDown, plain(), normal, b"\x1b[6~"),
        (Key::F(1), plain(), normal, b"\x1bOP"),
        (Key::F(4), ctrl, normal, b"\x1b[1;5S"),
        (Key::F(5), plain(), normal, b"\x1b[15~"),
        (Key::F(12), plain(), normal, b"\x1b[24~"),
        (Key::Char('c'), ctrl, normal, b"\x03"),
        (Key::Char('f'), alt, normal, b"\x1bf"),
        (Key::Char('['), ctrl, normal, b"\x1b"),
    ];
    for (key_pressed, modifiers, modes, bytes) in cases {
        assert_eq!(key(key_pressed, modifiers, modes).as_deref(), Some(bytes), "{key_pressed:?} {modifiers:?} {modes:?}");
    }
    assert_eq!(key(Key::Char('é'), ctrl, normal), None, "Ctrl gives nothing to a character with no control");
    assert_eq!(key(Key::F(13), plain(), normal), None);
    assert_eq!(key(Key::Char(' '), ctrl, normal).as_deref(), Some(b"\x00".as_slice()));
}

#[test]
fn text_typed_and_pasted_goes_as_a_terminal_sends_it() {
    assert_eq!(typed("ls é", false), "ls é".as_bytes());
    assert_eq!(typed("b", true), b"\x1bb");
    let bracketed = Modes { bracketed_paste: true, ..Modes::default() };
    assert_eq!(pasted("one\r\ntwo\nthree", Modes::default()), b"one\rtwo\rthree");
    assert_eq!(pasted("echo hi\n", bracketed), b"\x1b[200~echo hi\r\x1b[201~");
    assert_eq!(pasted("a\x1b[201~rm -rf\n", bracketed), b"\x1b[200~arm -rf\r\x1b[201~", "the end marker cannot be pasted");
}

#[test]
fn the_modes_the_device_sets_are_seen() {
    let mut terminal = Terminal::new(4, 10, 0);
    assert_eq!(terminal.modes(), Modes::default());
    terminal.receive(b"\x1b[?1h\x1b[?2004h");
    assert_eq!(terminal.modes(), Modes { application_cursor: true, bracketed_paste: true });
    terminal.receive(b"\x1b[?1l\x1b[?2004l");
    assert_eq!(terminal.modes(), Modes::default());
}

#[test]
fn a_selection_gives_its_text_and_a_double_click_its_word() {
    let mut terminal = Terminal::new(3, 30, 0);
    terminal.receive(b"ls /dev/ttyUSB0 -l\r\nok done");
    assert_eq!(terminal.text_between((1, 4), (0, 3)), "/dev/ttyUSB0 -l\nok d");
    assert_eq!(terminal.word_at(0, 8), Some((3, 15)), "a path is one word");
    assert_eq!(terminal.word_at(0, 2), None, "a blank is no word");
}

#[test]
fn the_screen_takes_a_new_size() {
    let mut terminal = Terminal::new(24, 80, 10);
    terminal.resize(10, 40);
    assert_eq!(terminal.size(), (10, 40));
    terminal.resize(0, 0);
    assert_eq!(terminal.size(), (1, 1), "never empty");
}

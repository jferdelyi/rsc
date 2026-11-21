//! A selection's text, in either direction, as shown, from lines dropped; a word taken.

use super::*;

#[test]
fn a_selection_gives_its_text_in_either_direction() {
    let console = received(&[b"first line\nsecond line\nthird line\n"]);
    let (a, b) = (Position { line: 0, col: 6 }, Position { line: 2, col: 5 });
    assert_eq!(console.text_between(a, b, TEXT), "line\nsecond line\nthird");
    assert_eq!(console.text_between(b, a, TEXT), "line\nsecond line\nthird");
    assert_eq!(console.text_between(Position { line: 1, col: 0 }, Position { line: 1, col: 6 }, TEXT), "second");
    assert_eq!(console.text_between(a, a, TEXT), "");
    assert_eq!(console.text_between(Position { line: 0, col: 0 }, console.end(TEXT), TEXT), "first line\nsecond line\nthird line");
}

#[test]
fn a_selection_counts_what_is_shown() {
    let console = received(&[b"abc\n"]);
    assert_eq!(console.text_between(Position { line: 0, col: 0 }, console.end(TIMED), TIMED), "10:15:00.000 abc");
    assert_eq!(console.text_between(Position { line: 0, col: TIME_WIDTH }, console.end(TIMED), TIMED), "abc");
    assert_eq!(console.text_between(Position { line: 0, col: 3 }, console.end(HEX), HEX), "62 63 0A");
}

#[test]
fn a_selection_that_begins_in_dropped_lines_gives_what_is_left() {
    let mut console = Console::new(2);
    console.receive(b"0\n1\n2\n3\n", at(0, 0));
    assert_eq!(console.text_between(Position { line: 0, col: 0 }, Position { line: 3, col: 1 }, TEXT), "2\n3");
}

#[test]
fn a_double_click_takes_the_word() {
    let console = received(&[b"mac 24:62:ab heap_free=214 kB\n"]);
    let word = |col| console.word_at(Position { line: 0, col }, TEXT).map(|(a, b)| console.text_between(a, b, TEXT));
    assert_eq!(word(1).as_deref(), Some("mac"));
    assert_eq!(word(4).as_deref(), Some("24"));
    assert_eq!(word(15).as_deref(), Some("heap_free"));
    assert_eq!(word(28).as_deref(), Some("kB"));
    assert_eq!(word(3), None, "a space is not a word");
    assert_eq!(word(99), None);
}

//! The tests of the console on screen: the following, the characters of a row, the colours, the
//! minimap, the selection.

use super::*;
use super::paint::{char_slice, ink_runs, pieces, row_job, selected_cells};
use crate::console::{Color, TIME_WIDTH, at};

const LOOK: Look = Look { font_size: 13.0, shown: Shown { timestamps: false, hex: false }, ansi_colors: true, minimap: true };

#[test]
fn the_console_stops_following_when_the_view_goes_up_and_follows_again_at_the_bottom() {
    // Following: lines that come keep it at the bottom; going up stops it.
    assert!(follow_next(true, false, true, true));
    assert!(!follow_next(true, true, true, false), "the view went up");
    assert!(follow_next(true, true, true, true), "a taller window, still at the bottom");
    // Not following: a line that comes does not move it; back at the bottom, it follows again.
    assert!(!follow_next(false, false, false, false));
    assert!(follow_next(false, false, false, true), "came back down");
    assert!(!follow_next(false, false, true, true), "turned off at the bottom: it stays off");
}

#[test]
fn a_slice_counts_characters_not_bytes() {
    assert_eq!(char_slice("status ok", 0, 6), "status");
    assert_eq!(char_slice("status ok", 7, 99), "ok");
    assert_eq!(char_slice("température 21 °C", 0, 11), "température");
    assert_eq!(char_slice("température 21 °C", 15, 17), "°C");
    assert_eq!(char_slice("abc", 5, 9), "");
}

#[test]
fn a_line_is_cut_where_its_colour_changes() {
    let mut console = Console::new(10);
    console.receive(b"ok \x1b[31mfail\x1b[0m done\n", at(0, 0));
    let line = &console.lines()[0];
    let default = theme::CONSOLE_TEXT;
    let red = theme::ink(line.kind, Color::Ansi(1), true);
    assert_eq!(pieces(line, &LOOK), [(0, 3, default), (3, 7, red), (7, usize::MAX, default)]);
    // With its time, the same pieces further right, after the time in its own colour.
    let timed = Look { shown: Shown { timestamps: true, hex: false }, ..LOOK };
    assert_eq!(
        pieces(line, &timed),
        [(0, TIME_WIDTH, theme::CONSOLE_TIME), (TIME_WIDTH, TIME_WIDTH + 3, default), (TIME_WIDTH + 3, TIME_WIDTH + 7, red), (TIME_WIDTH + 7, usize::MAX, default)]
    );
    // Colours switched off: what the device sends is of one colour.
    let plain = Look { ansi_colors: false, ..LOOK };
    assert!(pieces(line, &plain).iter().all(|piece| piece.2 == default));
    // In hexadecimal, the escape sequences are bytes like the others: one colour.
    let hex = Look { shown: Shown { timestamps: false, hex: true }, ..LOOK };
    assert_eq!(pieces(line, &hex), [(0, usize::MAX, default)]);
}

#[test]
fn the_minimap_draws_words_in_the_colours_of_the_text() {
    let mut console = Console::new(10);
    console.receive(b"ok  \x1b[31mfail now\x1b[0m end\n", at(0, 0));
    let line = &console.lines()[0];
    let red = theme::ink(line.kind, Color::Ansi(1), true);
    let runs = ink_runs(&line.display(LOOK.shown), &pieces(line, &LOOK), 0, line.width(LOOK.shown));
    assert_eq!(runs, [(0, 2, theme::CONSOLE_TEXT), (4, 8, red), (9, 12, red), (13, 16, theme::CONSOLE_TEXT)], "blanks left out, colours kept");
    // Only the characters of the row.
    let runs = ink_runs(&line.display(LOOK.shown), &pieces(line, &LOOK), 5, 10);
    assert_eq!(runs, [(5, 8, red), (9, 10, red)]);
}

#[test]
fn a_row_shows_its_part_of_the_line() {
    let mut console = Console::new(10);
    console.receive(b"0123456789abcdef\n", at(0, 0));
    let job = row_job(&console.lines()[0], &LOOK, 10, 16, &FontId::monospace(13.0));
    assert_eq!(job.text, "abcdef");
}

#[test]
fn a_selection_covers_its_cells_row_by_row() {
    let (start, end) = (Position { line: 4, col: 6 }, Position { line: 6, col: 3 });
    assert_eq!(selected_cells(3, 0, 20, start, end), None);
    assert_eq!(selected_cells(4, 0, 20, start, end), Some((6, 20)), "from its start to the end of the line");
    assert_eq!(selected_cells(5, 0, 11, start, end), Some((0, 11)), "a whole line");
    assert_eq!(selected_cells(6, 0, 20, start, end), Some((0, 3)), "to its end");
    assert_eq!(selected_cells(7, 0, 20, start, end), None);
    // A long line folded on two rows of 10: the selection starts in the second.
    let (start, end) = (Position { line: 0, col: 12 }, Position { line: 0, col: 15 });
    assert_eq!(selected_cells(0, 0, 10, start, end), None);
    assert_eq!(selected_cells(0, 10, 20, start, end), Some((12, 15)));
}

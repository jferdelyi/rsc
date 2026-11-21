//! The fold at the console's width: followed as the console changes, a line received, long lines,
//! a byte never cut in two.

use super::*;

fn text_folds(widths: &[usize]) -> impl Iterator<Item = Fold> + '_ {
    widths.iter().map(|&width| Fold { width, body: 0, unit: 1 })
}

/// Two folds of the same lines are the same: every row, and the row of every place of a line.
fn same_fold(a: &Wrap, b: &Wrap, lines: usize) -> bool {
    a.rows() == b.rows() && (0..a.rows()).all(|row| a.at_row(row) == b.at_row(row)) && (0..lines).all(|line| (0..400).step_by(7).all(|col| a.row_of(line, col) == b.row_of(line, col)))
}

#[test]
fn the_fold_followed_as_the_console_changes_is_the_fold_made_again() {
    // What a device sends, cut anywhere: short lines, long ones, CR and LF apart, colours, a
    // character cut in two; lines sent and notices between; the console full, and cleared.
    let pieces: [&[u8]; 9] = [b"abc", b"\r", b"\n", b"\r\n", &[b'x'; 130], b"\x1b[32mok\x1b[0m ", "\u{e9}".as_bytes(), b"\xc3", b"\xa9\n"];
    for shown in [Shown::default(), Shown { timestamps: true, hex: false }, Shown { timestamps: false, hex: true }] {
        let mut console = Console::new(40);
        let mut wrap = Wrap::of(&console, shown, 50);
        let mut seed: u32 = 17;
        for step in 0..2000 {
            seed = seed.wrapping_mul(1_103_515_245).wrapping_add(12_345);
            let pick = (seed >> 16) as usize;
            match pick % 101 {
                0 => console.clear(),
                1..=4 => console.sent("ls", b"ls\n", at(0, 0)),
                5..=8 => console.notice("port lost", at(0, 0)),
                _ => console.receive(pieces[pick % pieces.len()], at(0, 0)),
            }
            wrap.update(&console, shown);
            assert!(same_fold(&wrap, &Wrap::of(&console, shown, 50), console.lines().len()), "step {step}, {shown:?}");
        }
    }
}

#[test]
fn a_line_received_folds_one_or_two_lines_whatever_the_console_holds() {
    let mut console = Console::new(100_000);
    let lines: Vec<u8> = (0..100_000).flat_map(|n| format!("line {n}\n").into_bytes()).collect();
    console.receive(&lines, at(0, 0));
    let shown = Shown::default();
    let mut wrap = Wrap::of(&console, shown, 80);
    console.receive(b"one more\n", at(0, 1));
    assert!(wrap.update(&console, shown) <= 2, "the console holds 100 000 lines");
    // Full: its first line goes as the next one comes.
    console.receive(b"and another\n", at(0, 2));
    assert!(wrap.update(&console, shown) <= 2);
    assert!(same_fold(&wrap, &Wrap::of(&console, shown, 80), 3), "the first lines still where they are");
    assert_eq!(wrap.rows(), Wrap::of(&console, shown, 80).rows());
}

#[test]
fn long_lines_fold_at_the_width_of_the_console() {
    // Widths 0, 5, 10, 11 at 5 columns: 1, 1, 2 and 3 rows.
    let wrap = Wrap::new(text_folds(&[0, 5, 10, 11]), 5);
    assert_eq!(wrap.rows(), 7);
    assert_eq!(wrap.at_row(0), Some((0, 0, 5)));
    assert_eq!(wrap.at_row(1), Some((1, 0, 5)));
    assert_eq!(wrap.at_row(2), Some((2, 0, 5)));
    assert_eq!(wrap.at_row(3), Some((2, 5, 5)));
    assert_eq!(wrap.at_row(6), Some((3, 10, 5)));
    assert_eq!(wrap.at_row(7), None);
    assert_eq!(wrap.row_of(2, 4), 2);
    assert_eq!(wrap.row_of(2, 5), 3);
    assert_eq!(wrap.row_of(2, 10), 3, "the end of a full row stays on it");
    assert_eq!(wrap.row_of(1, 5), 1);
    assert_eq!(wrap.row_of(3, 10), 6);
    assert_eq!(Wrap::new(std::iter::empty(), 80).rows(), 0);
    assert_eq!(Wrap::new(text_folds(&[3]), 0).cols(), 1, "never a width of nothing");
}

#[test]
fn a_byte_is_never_cut_in_two_by_the_fold() {
    // "48 65 6C 6C 6F" at 8 columns: two bytes and their spaces a row, "48 65 " then "6C 6C " then "6F".
    let wrap = Wrap::new([Fold { width: 14, body: 0, unit: 3 }].into_iter(), 8);
    assert_eq!(wrap.rows(), 3);
    assert_eq!(wrap.at_row(1), Some((0, 6, 6)));
    assert_eq!(wrap.at_row(2), Some((0, 12, 6)));
    // After its time (13 characters) at 20 columns: the first row holds two bytes, the next six.
    let wrap = Wrap::new([Fold { width: 13 + 29, body: 13, unit: 3 }].into_iter(), 20);
    assert_eq!(wrap.at_row(0), Some((0, 0, 19)));
    assert_eq!(wrap.at_row(1), Some((0, 19, 18)));
    assert_eq!(wrap.rows(), 3);
    // A console narrower than the time: the rows are as wide as it is.
    let wrap = Wrap::new([Fold { width: 20, body: 13, unit: 3 }].into_iter(), 10);
    assert_eq!(wrap.at_row(0), Some((0, 0, 10)));
    assert_eq!(wrap.at_row(1), Some((0, 10, 9)));
}

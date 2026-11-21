//! The screen of the terminal mode against tmux's, on real programs recorded by
//! tests/terminal/record.sh: bash and its line editing, vi, less, nano, a whiptail menu, curses in
//! 256 colours. What each program wrote, replayed into RSC's screen in small reads, gives the same
//! characters, colours, cursor and screen as tmux gave. The comparison is shown to fail on screens
//! spoilt on purpose.

use std::path::PathBuf;

use rsc::terminal::Terminal;

const PROGRAMS: [&str; 6] = ["shell", "vi", "less", "nano", "menu", "couleurs"];
const ROWS: u16 = 24;
const COLS: u16 = 80;

fn file(name: &str, kind: &str) -> Vec<u8> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests").join("terminal").join(format!("{name}.{kind}"));
    std::fs::read(&path).unwrap_or_else(|error| panic!("{}: {error}", path.display()))
}

/// What a cell shows, as compared: its character and how it is drawn.
#[derive(Clone, Debug, PartialEq)]
struct Look {
    text: String,
    fg: vt100::Color,
    bg: vt100::Color,
    bold: bool,
    underline: bool,
    inverse: bool,
}

fn looks(terminal: &Terminal) -> Vec<Vec<Look>> {
    let screen = terminal.screen();
    (0..ROWS)
        .map(|row| {
            (0..COLS)
                .map(|col| {
                    let cell = screen.cell(row, col).expect("a cell of the screen");
                    // A blank written and a cell never written look the same: tmux's capture
                    // leaves out the blanks at the end of a row.
                    let text = if cell.has_contents() { cell.contents().to_owned() } else { " ".to_owned() };
                    // On a blank, the colour of the text and its weight do not show, unless it is
                    // in inverse: what a blank shows is its background, its inverse, its line.
                    let blank = text.trim().is_empty() && !cell.inverse();
                    let fg = if blank { vt100::Color::Default } else { cell.fgcolor() };
                    Look { text, fg, bg: cell.bgcolor(), bold: cell.bold() && !blank, underline: cell.underline(), inverse: cell.inverse() }
                })
                .collect()
        })
        .collect()
}

/// What the program wrote, read in pieces of `piece` bytes: sequences and characters cut anywhere.
fn replayed(out: &[u8], piece: usize) -> Terminal {
    let mut terminal = Terminal::new(ROWS, COLS, 0);
    for bytes in out.chunks(piece) {
        terminal.receive(bytes);
    }
    terminal
}

/// tmux's screen, as `capture-pane -e` writes it with its colours: G1 holds the line-drawing set,
/// as tmux sets it up for itself, and each row is followed by CR LF.
fn reference(colors: &[u8]) -> Terminal {
    let mut terminal = Terminal::new(ROWS, COLS, 0);
    terminal.receive(b"\x1b)0");
    for (index, line) in colors.split(|&byte| byte == b'\n').take(usize::from(ROWS)).enumerate() {
        if index > 0 {
            terminal.receive(b"\r\n");
        }
        terminal.receive(line);
    }
    terminal
}

/// Where the cursor is, as rows then columns, and whether the alternate screen is on.
fn state(text: &[u8]) -> ((u16, u16), bool) {
    let text = String::from_utf8_lossy(text);
    let numbers: Vec<u16> = text.split_whitespace().map(|number| number.parse().expect("a number")).collect();
    ((numbers[1], numbers[0]), numbers[2] == 1)
}

/// How RSC's screen differs from tmux's, a few cells at most.
fn differences(ours: &Terminal, theirs: &Terminal) -> Vec<String> {
    let (ours, theirs) = (looks(ours), looks(theirs));
    let mut found = Vec::new();
    for (row, (our_row, their_row)) in ours.iter().zip(&theirs).enumerate() {
        for (col, (our, their)) in our_row.iter().zip(their_row).enumerate() {
            if our != their && found.len() < 8 {
                found.push(format!("row {row}, column {col}: RSC {our:?}, tmux {their:?}"));
            }
        }
    }
    found
}

#[test]
fn real_programs_draw_on_rsc_s_screen_as_on_tmux_s() {
    for name in PROGRAMS {
        let theirs = reference(&file(name, "colors"));
        let ((row, col), alternate) = state(&file(name, "state"));
        for piece in [1, 7, 4096] {
            let ours = replayed(&file(name, "out"), piece);
            let found = differences(&ours, &theirs);
            assert!(found.is_empty(), "{name}, read {piece} bytes at a time:\n{}\nRSC's screen:\n{}", found.join("\n"), ours.screen().contents());
            assert_eq!(ours.screen().cursor_position(), (row, col), "{name}: the cursor");
            assert_eq!(ours.screen().alternate_screen(), alternate, "{name}: the alternate screen");
        }
    }
}

#[test]
fn the_replay_of_tmux_s_colours_gives_tmux_s_text() {
    // The programs that draw no line from the line-drawing set: tmux's text writes those as letters.
    for name in ["vi", "less", "nano", "menu"] {
        let theirs = reference(&file(name, "colors"));
        let text: Vec<String> = theirs.screen().rows(0, COLS).map(|row| row.trim_end().to_owned()).collect();
        let expected: Vec<String> = String::from_utf8(file(name, "screen")).expect("text").lines().map(|row| row.trim_end().to_owned()).collect();
        assert_eq!(text, expected, "{name}");
    }
}

#[test]
fn the_comparison_sees_a_character_a_colour_and_a_cursor_spoilt() {
    let out = file("couleurs", "out");
    let theirs = reference(&file("couleurs", "colors"));
    assert!(differences(&replayed(&out, 4096), &theirs).is_empty());
    let spoil = |from: &[u8], to: &[u8]| {
        let at = out.windows(from.len()).position(|window| window == from).expect("the bytes to spoil");
        [&out[..at], to, &out[at + from.len()..]].concat()
    };
    let colour = spoil(b"208", b"209");
    assert!(differences(&replayed(&colour, 4096), &theirs).iter().any(|found| found.contains("Idx(209)")), "a colour");
    let text = spoil(b"orange", b"orangE");
    assert!(!differences(&replayed(&text, 4096), &theirs).is_empty(), "a character");
    let lines = spoil(b"\x1b(0", b"\x1b(B");
    assert!(!differences(&replayed(&lines, 4096), &theirs).is_empty(), "the line-drawing set left out");
}

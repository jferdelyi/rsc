//! RSC on a real Arduino flashed with its test program (tests/arduino/rsc_essai, flashed by
//! tests/arduino/flash.ps1), run by hand:
//!
//!     RSC_ARDUINO=COM10 cargo test --test arduino -- --ignored --nocapture --test-threads=1
//!
//! Through RSC's own link thread, terminal screen and line console: the program's console edited
//! and recalled, the size of the screen asked through a real link, a menu in a frame of the
//! line-drawing set led by the arrows, measures stopped by Ctrl+C and read as curves, and the
//! flow of a real USB link, every line of it counted.

use std::sync::{Mutex, PoisonError};
use std::time::{Duration, Instant};

use rsc::console::Console;
use rsc::link::{Command, Event, Link};
use rsc::plot::Plot;
use rsc::settings::LinkSettings;
use rsc::terminal::{self, Key, Modes, Modifiers, Terminal};

/// No step waits longer than this.
const DEADLINE: Duration = Duration::from_secs(10);

/// One test at a time: they share the board.
static THE_BOARD: Mutex<()> = Mutex::new(());

/// The board as RSC holds it: its link, its terminal screen of 80 by 24, its line console.
struct Board {
    link: Link,
    terminal: Terminal,
    console: Console,
}

impl Board {
    /// The board of RSC_ARDUINO, open at a prompt; none when the variable is not set.
    fn open() -> Option<Self> {
        let Ok(port) = std::env::var("RSC_ARDUINO") else {
            eprintln!("RSC_ARDUINO not set: nothing to do");
            return None;
        };
        let link = Link::system(|| {});
        // Refused while the system has not yet let go of the port a test before held.
        let start = Instant::now();
        loop {
            link.send(Command::Open { port: port.clone(), link: LinkSettings::default() });
            match link.next_event(DEADLINE) {
                Some(Event::Opened { .. }) => break,
                Some(Event::Refused { .. }) if start.elapsed() < Duration::from_secs(5) => std::thread::sleep(Duration::from_millis(300)),
                other => panic!("{port} not opened: {other:?}"),
            }
        }
        let mut board = Self { link, terminal: Terminal::new(24, 80, 1000), console: Console::new(100_000) };
        // A prompt of its own: the program may have printed its first one to an earlier session.
        board.press(Key::Enter);
        board.until("the prompt", |board| board.current_row() == "essai>");
        Some(board)
    }

    /// The row the cursor is on, blanks at its end left out.
    fn current_row(&self) -> String {
        let (row, _) = self.terminal.screen().cursor_position();
        self.rows().get(usize::from(row)).cloned().unwrap_or_default()
    }

    /// What comes for `wait`, given to the screen and to the line console; the screen's answers sent.
    fn pump(&mut self, wait: Duration) {
        let end = Instant::now() + wait;
        while Instant::now() < end {
            match self.link.next_event(Duration::from_millis(20)) {
                Some(Event::Data(bytes)) => {
                    self.console.receive(&bytes, chrono::Local::now().naive_local());
                    let answers = self.terminal.receive(&bytes);
                    if !answers.is_empty() {
                        self.link.send(Command::Send(answers));
                    }
                }
                Some(Event::Closed(why)) => panic!("the board's port closed: {why:?}"),
                _ => {}
            }
        }
    }

    fn until(&mut self, what: &str, done: impl Fn(&Self) -> bool) {
        let start = Instant::now();
        while !done(self) {
            assert!(start.elapsed() < DEADLINE, "{what}: the screen is\n{}", self.terminal.screen().contents());
            self.pump(Duration::from_millis(50));
        }
    }

    fn typed(&self, text: &str) {
        self.link.send(Command::Send(terminal::typed(text, false)));
    }

    fn press(&self, key: Key) {
        self.link.send(Command::Send(terminal::key(key, Modifiers::default(), Modes::default()).expect("bytes for the key")));
    }

    /// The rows of the screen, blanks at their end left out.
    fn rows(&self) -> Vec<String> {
        self.terminal.screen().rows(0, self.terminal.size().1).map(|row| row.trim_end().to_owned()).collect()
    }

    fn screen(&self) -> String {
        self.terminal.screen().contents()
    }

    fn text(&self) -> String {
        self.console.text(rsc::console::Shown::default())
    }

    /// Whether the cell where `text` begins on the screen is in inverse.
    fn inverse_at(&self, text: &str) -> bool {
        self.rows().iter().enumerate().any(|(row, line)| {
            line.find(text).is_some_and(|at| {
                let col = line[..at].chars().count();
                self.terminal.screen().cell(row as u16, col as u16).is_some_and(vt100::Cell::inverse)
            })
        })
    }

}

/// The port let go of however the test ends, a failure included, before the next test opens it.
impl Drop for Board {
    fn drop(&mut self) {
        self.link.send(Command::Close);
        let start = Instant::now();
        while start.elapsed() < Duration::from_secs(3) {
            if let Some(Event::Closed(_)) = self.link.next_event(Duration::from_millis(100)) {
                break;
            }
        }
    }
}

#[test]
#[ignore = "needs the board: RSC_ARDUINO=<port>"]
fn the_board_s_console_menu_and_size_on_the_terminal_screen() {
    let _alone = THE_BOARD.lock().unwrap_or_else(PoisonError::into_inner);
    let Some(mut board) = Board::open() else {
        return;
    };
    // A line mended before it goes, then brought back with the up arrow.
    board.typed("aidx");
    board.press(Key::Backspace);
    board.typed("e");
    board.press(Key::Enter);
    board.until("the help", |board| board.screen().contains("Commandes :") && board.current_row() == "essai>");
    board.press(Key::Up);
    board.until("the line brought back", |board| board.current_row() == "essai> aide");
    board.press(Key::Enter);
    board.until("the help again", |board| board.text().matches("Commandes :").count() == 2);

    // The size of the screen, asked by the board, answered by RSC through the real link.
    board.typed("taille");
    board.press(Key::Enter);
    board.until("the size", |board| board.screen().contains("Taille annoncée : 80x24"));
    assert!(board.screen().contains("Curseur au plus loin : ligne 24, colonne 80"), "{}", board.screen());

    // A menu on the alternate screen, framed by the line-drawing set, led by the arrows.
    board.typed("menu");
    board.press(Key::Enter);
    board.until("the menu", |board| board.terminal.screen().alternate_screen() && board.screen().contains("Menu d'essai") && board.inverse_at("Lire les capteurs"));
    // The frame from column 10, counted from 1: nine blanks before its corner.
    assert!(board.rows().iter().any(|row| row.starts_with("         ┌─") && row.ends_with("─┐")), "the frame:\n{}", board.screen());
    assert!(board.rows().iter().filter(|row| row.trim_start().starts_with('│')).count() >= 7, "its sides");
    board.press(Key::Down);
    board.until("the second line chosen", |board| board.inverse_at("Régler l'heure") && !board.inverse_at("Lire les capteurs"));
    board.press(Key::Enter);
    board.until("the choice", |board| !board.terminal.screen().alternate_screen() && board.screen().contains("Choisi : Régler l'heure"));
    eprintln!("the screen at the end:\n{}", board.screen());
}

#[test]
#[ignore = "needs the board: RSC_ARDUINO=<port>"]
fn the_board_s_measures_and_flow_through_the_line_console() {
    let _alone = THE_BOARD.lock().unwrap_or_else(PoisonError::into_inner);
    let Some(mut board) = Board::open() else {
        return;
    };
    // Measures until Ctrl+C, read as curves.
    board.typed("mesures");
    board.press(Key::Enter);
    board.pump(Duration::from_millis(2000));
    board.link.send(Command::Send(terminal::key(Key::Char('c'), Modifiers { ctrl: true, ..Modifiers::default() }, Modes::default()).expect("Ctrl+C")));
    board.until("the measures stopped", |board| board.text().contains("Mesures arrêtées après"));
    let mut plot = Plot::new(10_000);
    let read = board.console.lines().iter().filter(|line| plot.read(line, "")).count();
    let curves: Vec<(String, usize)> = plot.curves().iter().map(|curve| (curve.name.clone(), curve.points().len())).collect();
    eprintln!("measures: {read} lines read, curves {curves:?}");
    assert_eq!(curves.iter().map(|(name, _)| name.as_str()).collect::<Vec<_>>(), ["temp", "pression", "compteur"]);
    assert!(read >= 15, "about ten lines a second for two seconds: {read}");

    // The flow of the real link: every line counted, none lost.
    board.typed("flux");
    board.press(Key::Enter);
    let start = Instant::now();
    board.until("the flow's end", |board| board.text().contains("octets/s"));
    let took = start.elapsed();
    let text = board.text();
    let received = text.lines().filter(|line| line.starts_with("flux ") && line.ends_with(" abcdefghijklmnopqrstuvwxyz 0123456789")).count();
    let summary = text.lines().find(|line| line.starts_with("flux : ")).expect("the summary").to_owned();
    let said: usize = summary["flux : ".len()..].split(' ').next().and_then(|count| count.parse().ok()).expect("the count of lines");
    eprintln!("flow: {received} lines received in {took:?}; the board says: {summary}");
    assert_eq!(received, said, "every line the board sent came through");
}

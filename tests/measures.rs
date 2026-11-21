//! How light and how fast the console and the curves are, measured in an optimized build, run by hand:
//!
//!     cargo test --release --test measures -- --ignored --nocapture --test-threads=1
//!
//! Each measure writes its raw values as CSV into RSC_MEASURES (build/measures by default), with
//! semicolons and decimal commas, and prints the median, the quartiles, the least and the most. The
//! first run of each series is a warm-up: kept with warmup=1, left out of the figures. Nothing here
//! asserts a time; each measure checks first that it counts what it says.
//!
//! The memory is counted by an allocator of this test program only: the bytes asked of the system
//! and not given back yet.

use std::alloc::{GlobalAlloc, Layout, System};
use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Mutex, PoisonError};
use std::time::{Duration, Instant};

use rsc::console::Console;
use rsc::plot::Plot;
use rsc::terminal::Terminal;

struct Counting;

static LIVE: AtomicUsize = AtomicUsize::new(0);

// SAFETY: each call is the system allocator's, as it is; only the sizes are counted.
unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let pointer = unsafe { System.alloc(layout) };
        if !pointer.is_null() {
            LIVE.fetch_add(layout.size(), Ordering::Relaxed);
        }
        pointer
    }

    unsafe fn dealloc(&self, pointer: *mut u8, layout: Layout) {
        unsafe { System.dealloc(pointer, layout) };
        LIVE.fetch_sub(layout.size(), Ordering::Relaxed);
    }

    unsafe fn realloc(&self, pointer: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        let moved = unsafe { System.realloc(pointer, layout, new_size) };
        if !moved.is_null() {
            LIVE.fetch_sub(layout.size(), Ordering::Relaxed);
            LIVE.fetch_add(new_size, Ordering::Relaxed);
        }
        moved
    }
}

#[global_allocator]
static ALLOCATOR: Counting = Counting;

/// The measures one after the other, whatever the number of test threads: the memory counted is
/// that of one measure only.
static ONE_AT_A_TIME: Mutex<()> = Mutex::new(());

/// Runs of each series, the warm-up not counted.
const RUNS: usize = 15;
/// A run that takes longer ends its series.
const RUN_BOUND: Duration = Duration::from_secs(30);
/// What 921600 bauds bring at most, in 8N1: ten bits a byte.
const BYTES_PER_S_921600: f64 = 92_160.0;
/// A piece of the stream as the window takes it at most in a frame, at 60 frames a second.
const PIECE: usize = 1536;

/// A line of an ESP-IDF board's journal, as COM23's: coloured, ended by CR LF.
fn log_line(n: usize) -> String {
    let ms = 14_964 + n * 7;
    match n % 4 {
        0 => format!("\x1b[0;32mI ({ms}) lte: Checking lte link\x1b[0m\r\n"),
        1 => format!("\x1b[0;32mI ({ms}) lte: Network registration: NET=2,GPRS=4,LTE=2\x1b[0m\r\n"),
        2 => format!("\x1b[0;32mI ({ms}) sampling: Readings in buffer {}/0\x1b[0m\r\n", n % 10),
        _ => format!("\x1b[0;33mW ({ms}) upload: Queue is full\x1b[0m\r\n"),
    }
}

/// A line of measures, as the curves read them.
fn measure_line(n: usize) -> String {
    let x = n as f64 / 50.0;
    format!("temp={:.2} hum={:.2} wind={:.2}\r\n", 21.0 + x.sin(), 40.0 + 3.0 * x.cos(), 12.0 + 4.0 * (x / 5.0).sin())
}

fn stream(lines: usize, line: fn(usize) -> String) -> Vec<u8> {
    (0..lines).flat_map(|n| line(n).into_bytes()).collect()
}

/// A time for each piece, a millisecond apart: the curves count their time from it.
fn time(piece: usize) -> chrono::NaiveDateTime {
    let start = chrono::NaiveDate::from_ymd_opt(2026, 11, 17).and_then(|day| day.and_hms_opt(10, 0, 0)).expect("a date");
    start + chrono::TimeDelta::milliseconds(piece as i64)
}

fn folder() -> PathBuf {
    let folder = std::env::var_os("RSC_MEASURES").map_or_else(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("build").join("measures"), PathBuf::from);
    fs::create_dir_all(&folder).expect("the measures' folder");
    folder
}

fn decimal(value: f64, places: usize) -> String {
    format!("{value:.places$}").replace('.', ",")
}

/// The median, the quartiles, the least and the most of the values (quartiles between the closest
/// ranks, as Excel's QUARTILE.INC).
fn summary(values: &[f64]) -> String {
    let mut sorted = values.to_vec();
    sorted.sort_by(f64::total_cmp);
    let at = |share: f64| {
        let place = share * (sorted.len() - 1) as f64;
        let (low, high) = (place.floor() as usize, place.ceil() as usize);
        sorted[low] + (sorted[high] - sorted[low]) * (place - low as f64)
    };
    format!("median {:.1}, quartiles {:.1} to {:.1}, least {:.1}, most {:.1} ({} runs)", at(0.5), at(0.25), at(0.75), sorted[0], sorted[sorted.len() - 1], sorted.len())
}

/// Times `run` RUNS times after a warm-up, a run ending its series past RUN_BOUND: the seconds of
/// each, the warm-up first.
fn series(mut run: impl FnMut() -> Duration) -> Vec<Duration> {
    let mut times = Vec::new();
    for _ in 0..=RUNS {
        let took = run();
        times.push(took);
        if took > RUN_BOUND {
            println!("a run took {took:?}, past {RUN_BOUND:?}: the series ends there");
            break;
        }
    }
    times
}

#[test]
#[ignore = "a measure: run by hand in an optimized build"]
fn the_console_takes_a_journal_faster_than_the_fastest_link() {
    let _alone = ONE_AT_A_TIME.lock().unwrap_or_else(PoisonError::into_inner);
    let lines = 100_000;
    let bytes = stream(lines, log_line);
    // The check: every line of the stream is one line of the console.
    let mut console = Console::new(lines);
    for (n, piece) in bytes.chunks(PIECE).enumerate() {
        console.receive(piece, time(n));
    }
    assert_eq!((console.lines().len(), console.lines()[1].text()), (lines, "I (14971) lte: Network registration: NET=2,GPRS=4,LTE=2"));

    let times = series(|| {
        // The console of every day: 10 000 lines kept.
        let mut console = Console::new(10_000);
        let start = Instant::now();
        for (n, piece) in bytes.chunks(PIECE).enumerate() {
            console.receive(piece, time(n));
        }
        start.elapsed()
    });
    let mut csv = String::from("run;warmup;bytes;seconds;bytes_per_s\n");
    for (run, took) in times.iter().enumerate() {
        let rate = bytes.len() as f64 / took.as_secs_f64();
        csv += &format!("{run};{};{};{};{}\n", u8::from(run == 0), bytes.len(), decimal(took.as_secs_f64(), 6), decimal(rate, 0));
    }
    fs::write(folder().join("console.csv"), csv).expect("console.csv");
    let rates: Vec<f64> = times.iter().skip(1).map(|took| bytes.len() as f64 / took.as_secs_f64() / 1e6).collect();
    let speeds: Vec<f64> = rates.iter().map(|rate| rate * 1e6 / BYTES_PER_S_921600).collect();
    println!("console, {} bytes of journal ({lines} lines): MB/s {}", bytes.len(), summary(&rates));
    println!("  times 921600 bauds: {}", summary(&speeds));
}

#[test]
#[ignore = "a measure: run by hand in an optimized build"]
fn the_memory_a_line_and_a_point_take() {
    let _alone = ONE_AT_A_TIME.lock().unwrap_or_else(PoisonError::into_inner);
    // The check: the allocator counts what is asked, and gives it back.
    let before = LIVE.load(Ordering::Relaxed);
    let known: Vec<u8> = Vec::with_capacity(1_000_000);
    assert_eq!(LIVE.load(Ordering::Relaxed) - before, 1_000_000);
    drop(known);
    assert_eq!(LIVE.load(Ordering::Relaxed), before);

    let mut csv = String::from("what;count;bytes;bytes_each\n");
    for lines in [10_000, 100_000, 1_000_000] {
        let bytes = stream(lines, log_line);
        let before = LIVE.load(Ordering::Relaxed);
        let mut console = Console::new(lines);
        for (n, piece) in bytes.chunks(PIECE).enumerate() {
            console.receive(piece, time(n));
        }
        assert_eq!(console.lines().len(), lines);
        let used = LIVE.load(Ordering::Relaxed) - before;
        csv += &format!("console_lines;{lines};{used};{}\n", decimal(used as f64 / lines as f64, 1));
        println!("console, {lines} lines kept: {:.1} MB, {:.0} bytes a line", used as f64 / 1e6, used as f64 / lines as f64);
        drop(console);
    }
    for rows in [10_000, 100_000, 1_000_000] {
        let bytes = stream(rows, measure_line);
        let mut console = Console::new(1_000);
        console.follow(true);
        let before = LIVE.load(Ordering::Relaxed);
        let mut plot = Plot::new(rows);
        for (n, piece) in bytes.chunks(PIECE).enumerate() {
            console.receive(piece, time(n));
            for line in console.take_ended() {
                plot.read(&line, "");
            }
        }
        // The lines the console still holds are not the curves': they go with it. It had asked for
        // nothing before the count began.
        drop(console);
        let used = LIVE.load(Ordering::Relaxed).saturating_sub(before);
        let points: usize = plot.curves().iter().map(|curve| curve.points().len()).sum();
        assert_eq!((plot.curves().len(), points), (3, 3 * rows));
        csv += &format!("plot_points;{points};{used};{}\n", decimal(used as f64 / points as f64, 1));
        println!("curves, {points} points kept ({rows} lines of 3 values): {:.1} MB, {:.1} bytes a point", used as f64 / 1e6, used as f64 / points as f64);
    }
    fs::write(folder().join("memory.csv"), csv).expect("memory.csv");
}

/// What the thread of each tab pays every second to see the ports come and go
/// (`rsc::ports::available`, the machine's real ports).
#[test]
#[ignore = "a measure: run by hand in an optimized build"]
fn the_ports_are_listed_in_little_time() {
    let _alone = ONE_AT_A_TIME.lock().unwrap_or_else(PoisonError::into_inner);
    const CALLS: usize = 20;
    let mut times = Vec::new();
    let mut found = 0;
    for _ in 0..=CALLS {
        let start = Instant::now();
        found = rsc::ports::available().len();
        times.push(start.elapsed());
    }
    let mut csv = String::from("call;warmup;ports;milliseconds\n");
    for (call, took) in times.iter().enumerate() {
        csv += &format!("{call};{};{found};{}\n", u8::from(call == 0), decimal(took.as_secs_f64() * 1e3, 3));
    }
    fs::write(folder().join("ports.csv"), csv).expect("ports.csv");
    let millis: Vec<f64> = times.iter().skip(1).map(|took| took.as_secs_f64() * 1e3).collect();
    println!("ports listed ({found} found): ms a list {}", summary(&millis));
}

/// The terminal's screen, 80 by 24 as it begins, fed a stream in the pieces a frame takes.
fn on_terminal(bytes: &[u8], history: usize) -> Terminal {
    let mut terminal = Terminal::new(24, 80, history);
    for piece in bytes.chunks(PIECE) {
        terminal.receive(piece);
    }
    terminal
}

/// The terminal mode's screen, on an ESP-IDF journal (rows that scroll into the history) and on a
/// real whiptail menu recorded by tests/terminal/record.sh, a thousand times over (a full screen
/// drawn again and again).
#[test]
#[ignore = "a measure: run by hand in an optimized build"]
fn the_terminal_draws_faster_than_the_fastest_link() {
    let _alone = ONE_AT_A_TIME.lock().unwrap_or_else(PoisonError::into_inner);
    let journal = stream(100_000, log_line);
    let menu = fs::read(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests").join("terminal").join("menu.out")).expect("the recorded menu");
    let menus = menu.repeat(1000);
    // The checks: the journal's last line on the screen, the menu drawn.
    assert!(on_terminal(&journal, 10_000).screen().contents().contains("W (714957) upload: Queue is full"));
    assert!(on_terminal(&menus, 0).screen().contents().contains("Choisir un débit"));

    let mut csv = String::from("what;run;warmup;bytes;seconds;bytes_per_s\n");
    for (what, bytes, history) in [("journal", &journal, 10_000), ("menu", &menus, 0)] {
        let times = series(|| {
            let start = Instant::now();
            drop(on_terminal(bytes, history));
            start.elapsed()
        });
        for (run, took) in times.iter().enumerate() {
            csv += &format!("{what};{run};{};{};{};{}\n", u8::from(run == 0), bytes.len(), decimal(took.as_secs_f64(), 6), decimal(bytes.len() as f64 / took.as_secs_f64(), 0));
        }
        let rates: Vec<f64> = times.iter().skip(1).map(|took| bytes.len() as f64 / took.as_secs_f64() / 1e6).collect();
        let speeds: Vec<f64> = rates.iter().map(|rate| rate * 1e6 / BYTES_PER_S_921600).collect();
        println!("terminal, {what}, {} bytes: MB/s {}", bytes.len(), summary(&rates));
        println!("  times 921600 bauds: {}", summary(&speeds));
    }
    fs::write(folder().join("terminal.csv"), csv).expect("terminal.csv");
}

/// What a row of the terminal's history takes, 80 and 200 columns wide.
#[test]
#[ignore = "a measure: run by hand in an optimized build"]
fn the_memory_a_row_of_terminal_history_takes() {
    let _alone = ONE_AT_A_TIME.lock().unwrap_or_else(PoisonError::into_inner);
    let rows = 10_000;
    let bytes = stream(rows + 24, log_line);
    let mut csv = String::from("what;rows;bytes;bytes_each\n");
    for cols in [80_u16, 200] {
        let before = LIVE.load(Ordering::Relaxed);
        let mut terminal = Terminal::new(24, cols, rows);
        for piece in bytes.chunks(PIECE) {
            terminal.receive(piece);
        }
        assert_eq!(terminal.history(), rows, "the check: the history full");
        let used = LIVE.load(Ordering::Relaxed) - before;
        csv += &format!("terminal_history_{cols}_columns;{rows};{used};{}\n", decimal(used as f64 / rows as f64, 1));
        println!("terminal, {rows} rows of history, {cols} columns: {:.1} MB, {:.0} bytes a row", used as f64 / 1e6, used as f64 / rows as f64);
        drop(terminal);
    }
    fs::write(folder().join("terminal_memory.csv"), csv).expect("terminal_memory.csv");
}

#[test]
#[ignore = "a measure: run by hand in an optimized build"]
fn the_curves_read_measures_faster_than_the_fastest_link() {
    let _alone = ONE_AT_A_TIME.lock().unwrap_or_else(PoisonError::into_inner);
    let lines = 100_000;
    let bytes = stream(lines, measure_line);
    let read = |plot: &mut Plot| {
        let mut console = Console::new(10_000);
        console.follow(true);
        for (n, piece) in bytes.chunks(PIECE).enumerate() {
            console.receive(piece, time(n));
            for line in console.take_ended() {
                plot.read(&line, "");
            }
        }
    };
    // The check: three curves, a point each for every line.
    let mut plot = Plot::new(lines);
    read(&mut plot);
    assert_eq!(plot.curves().iter().map(|curve| curve.points().len()).collect::<Vec<_>>(), [lines, lines, lines]);

    let times = series(|| {
        // The curves of every day: 100 000 points kept.
        let mut plot = Plot::new(100_000);
        let start = Instant::now();
        read(&mut plot);
        start.elapsed()
    });
    let mut csv = String::from("run;warmup;lines;bytes;seconds;lines_per_s\n");
    for (run, took) in times.iter().enumerate() {
        csv += &format!("{run};{};{lines};{};{};{}\n", u8::from(run == 0), bytes.len(), decimal(took.as_secs_f64(), 6), decimal(lines as f64 / took.as_secs_f64(), 0));
    }
    fs::write(folder().join("curves.csv"), csv).expect("curves.csv");
    let rates: Vec<f64> = times.iter().skip(1).map(|took| lines as f64 / took.as_secs_f64() / 1e3).collect();
    let speeds: Vec<f64> = times.iter().skip(1).map(|took| bytes.len() as f64 / took.as_secs_f64() / BYTES_PER_S_921600).collect();
    println!("console and curves, {lines} lines of 3 measures ({} bytes): thousand lines/s {}", bytes.len(), summary(&rates));
    println!("  times 921600 bauds: {}", summary(&speeds));
}

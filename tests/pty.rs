//! The thread of a port on a real serial device of the system: a pair of pseudo-terminals, whose
//! one end the tool opens by its path, as it would /dev/ttyUSB0. Linux and macOS.
#![cfg(unix)]

use std::io::{Read, Write};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use rsc::link::{Command, Event, Link};
use rsc::settings::LinkSettings;
use rsc::terminal::{self, Key, Modes, Modifiers, Terminal};
use serialport::{SerialPort, TTYPort};

/// No test waits longer than this.
const DEADLINE: Duration = Duration::from_secs(5);

fn next(link: &Link) -> Event {
    link.next_event(DEADLINE).expect("an event before the deadline")
}

#[test]
fn a_pseudo_terminal_is_read_and_written_through_its_path() {
    let (mut device, tool_end) = TTYPort::pair().expect("a pair of pseudo-terminals");
    let path = tool_end.name().expect("the path of its end");
    // Closed here, opened again by the tool through its path.
    drop(tool_end);

    let link = Link::system(|| {});
    link.send(Command::Open { port: path.clone(), link: LinkSettings::default() });
    match next(&link) {
        Event::Opened { port, .. } => assert_eq!(port, path),
        other => panic!("{path} not opened: {other:?}"),
    }

    device.write_all(b"hello from the device\r\n").expect("the device writes");
    let mut received = Vec::new();
    let start = Instant::now();
    while !received.ends_with(b"\r\n") {
        assert!(start.elapsed() < DEADLINE, "received so far: {received:?}");
        if let Event::Data(bytes) = next(&link) {
            received.extend(bytes);
        }
    }
    assert_eq!(received, b"hello from the device\r\n", "byte for byte, line end included");

    link.send(Command::Send(b"AT\r\n\x03".to_vec()));
    device.set_timeout(DEADLINE).expect("a timeout");
    let mut answer = [0u8; 5];
    device.read_exact(&mut answer).expect("the device reads");
    assert_eq!(&answer, b"AT\r\n\x03", "byte for byte, a control character included");

    link.send(Command::Close);
    assert_eq!(next(&link), Event::Closed(None));
}

/// Copies what one pseudo-terminal's end reads to the other's, both ways, until `stop`: what socat
/// does between the end the tool opens and the end a program runs on.
fn bridge(a: &TTYPort, b: &TTYPort, stop: &Arc<AtomicBool>) -> Vec<std::thread::JoinHandle<()>> {
    [(a, b), (b, a)]
        .into_iter()
        .map(|(from, to)| {
            let (mut from, mut to, stop) = (from.try_clone_native().expect("a clone"), to.try_clone_native().expect("a clone"), Arc::clone(stop));
            from.set_timeout(Duration::from_millis(50)).expect("a timeout");
            std::thread::spawn(move || {
                let mut buffer = [0u8; 4096];
                while !stop.load(Ordering::Relaxed) {
                    match from.read(&mut buffer) {
                        Ok(0) => {}
                        Ok(n) => {
                            if to.write_all(&buffer[..n]).is_err() {
                                return;
                            }
                        }
                        Err(error) if error.kind() == std::io::ErrorKind::TimedOut => {}
                        // No one holds the other end yet (the tool has not opened it): an error
                        // until it does.
                        Err(_) => std::thread::sleep(Duration::from_millis(20)),
                    }
                }
            })
        })
        .collect()
}

/// A program on a pseudo-terminal of its own, the controlling one of its session, in a folder of
/// its own, with nothing of the user's: no rc file, no home, the prompt "$ ".
fn run_on(path: &str, folder: &std::path::Path, script: &str) -> std::process::Child {
    let tty = || std::fs::OpenOptions::new().read(true).write(true).open(path).expect("the program's end");
    std::process::Command::new("setsid")
        .args(["-c", "sh", "-c", script])
        .env_clear()
        .env("PATH", "/usr/bin:/bin")
        .env("HOME", folder)
        .env("TERM", "xterm-256color")
        .env("LANG", "C.UTF-8")
        .env("PS1", "$ ")
        .current_dir(folder)
        .stdin(tty())
        .stdout(tty())
        .stderr(tty())
        .spawn()
        .expect("setsid and sh")
}

/// The terminal's screen fed by the tool's link, its answers sent back, until `done` holds of it.
fn until(link: &Link, terminal: &mut Terminal, what: &str, done: impl Fn(&Terminal) -> bool) {
    let start = Instant::now();
    while !done(terminal) {
        assert!(start.elapsed() < DEADLINE * 2, "{what}: the screen is\n{}", terminal.screen().contents());
        if let Some(Event::Data(bytes)) = link.next_event(Duration::from_millis(100)) {
            let answers = terminal.receive(&bytes);
            if !answers.is_empty() {
                link.send(Command::Send(answers));
            }
        }
    }
}

fn rows(terminal: &Terminal) -> Vec<String> {
    terminal.screen().rows(0, terminal.size().1).map(|row| row.trim_end().to_owned()).collect()
}

#[test]
fn a_shell_and_vi_run_on_the_terminal_screen() {
    let folder = std::env::temp_dir().join("rsc-tests").join("pty-shell");
    let _ = std::fs::remove_dir_all(&folder);
    std::fs::create_dir_all(&folder).expect("a folder");
    let (device, tool_end) = TTYPort::pair().expect("the pair the tool opens");
    let tool_path = tool_end.name().expect("its path");
    drop(tool_end);
    let (program_master, program_end) = TTYPort::pair().expect("the pair the program runs on");
    let program_path = program_end.name().expect("its path");
    let stop = Arc::new(AtomicBool::new(false));
    let threads = bridge(&device, &program_master, &stop);
    let mut shell = run_on(&program_path, &folder, "stty sane rows 24 cols 80; exec bash --norc --noprofile -i");
    drop(program_end);

    let link = Link::system(|| {});
    link.send(Command::Open { port: tool_path, link: LinkSettings::default() });
    assert!(matches!(next(&link), Event::Opened { .. }));
    let mut terminal = Terminal::new(24, 80, 1000);
    let keys = |bytes: Vec<u8>| link.send(Command::Send(bytes));
    let press = |key: Key| keys(terminal::key(key, Modifiers::default(), Modes::default()).expect("bytes for the key"));
    until(&link, &mut terminal, "the prompt", |terminal| rows(terminal).iter().any(|row| row == "$"));

    // A line typed, mended with the arrows before it goes, then brought back from the history.
    keys(terminal::typed("echo ac", false));
    press(Key::Left);
    keys(terminal::typed("b", false));
    press(Key::Enter);
    until(&link, &mut terminal, "the line mended", |terminal| rows(terminal).iter().any(|row| row == "abc"));
    press(Key::Up);
    until(&link, &mut terminal, "the line brought back", |terminal| rows(terminal).iter().filter(|row| row.as_str() == "$ echo abc").count() == 2);
    press(Key::Enter);
    until(&link, &mut terminal, "the line run again", |terminal| rows(terminal).iter().filter(|row| row.as_str() == "abc").count() == 2);

    // vi on the alternate screen: a line written, saved, and the shell's screen back as it was.
    keys(terminal::typed("vi -u NONE -i NONE -n notes.txt", false));
    press(Key::Enter);
    until(&link, &mut terminal, "vi", |terminal| terminal.screen().alternate_screen() && rows(terminal).iter().filter(|row| row.as_str() == "~").count() > 10);
    keys(terminal::typed("iligne un", false));
    until(&link, &mut terminal, "the line in vi", |terminal| rows(terminal)[0] == "ligne un");
    press(Key::Escape);
    // vi waits a moment for what may follow an escape before it takes it as such.
    std::thread::sleep(Duration::from_millis(1200));
    keys(terminal::typed(":wq", false));
    press(Key::Enter);
    until(&link, &mut terminal, "the shell again", |terminal| !terminal.screen().alternate_screen() && rows(terminal).iter().any(|row| row == "$ vi -u NONE -i NONE -n notes.txt"));
    keys(terminal::typed("cat notes.txt", false));
    press(Key::Enter);
    until(&link, &mut terminal, "the file vi wrote", |terminal| rows(terminal).iter().any(|row| row == "ligne un"));
    assert_eq!(std::fs::read_to_string(folder.join("notes.txt")).expect("the file"), "ligne un\n");

    link.send(Command::Close);
    let _ = shell.kill();
    let _ = shell.wait();
    stop.store(true, Ordering::Relaxed);
    for thread in threads {
        let _ = thread.join();
    }
    let _ = std::fs::remove_dir_all(&folder);
}

//! The tests of the threads of the ports, on ports played in memory.

use std::io::ErrorKind;
use std::time::Instant;

use super::fake::FakePorts;
use super::*;
use super::watch::list_every;

/// No test waits longer than this for the thread.
const DEADLINE: Duration = Duration::from_secs(5);

fn link(fake: &FakePorts) -> Link {
    Link::spawn(fake.clone(), || {})
}

fn next(link: &Link) -> Event {
    link.next_event(DEADLINE).expect("an event before the deadline")
}

fn open(link: &Link, port: &str, settings: LinkSettings) {
    link.send(Command::Open { port: port.into(), link: settings.clone() });
    assert_eq!(next(link), Event::Opened { port: port.into(), link: settings });
}

fn names(names: &[&str]) -> Vec<PortInfo> {
    FakePorts::with(names).0.lock().unwrap().listed.clone()
}

/// The next list of the ports, waited for.
fn next_list(watch: &PortWatch) -> Vec<PortInfo> {
    let start = Instant::now();
    loop {
        if let Some(list) = watch.latest() {
            return list;
        }
        assert!(start.elapsed() < DEADLINE, "a list before the deadline");
        std::thread::sleep(Duration::from_millis(5));
    }
}

#[test]
fn the_ports_are_listed_half_as_often_when_every_port_is_open() {
    assert_eq!((list_every(false), list_every(true)), (Duration::from_secs(1), Duration::from_secs(2)));
    // Two watches side by side for 1.3 s: lists at start and after a second, or at start only.
    let (waiting, open) = (FakePorts::with(&["COM3"]), FakePorts::with(&["COM3"]));
    let _waiting = PortWatch::spawn(waiting.clone(), || {});
    let mut all_open = PortWatch::spawn(open.clone(), || {});
    all_open.pace(true);
    std::thread::sleep(Duration::from_millis(1300));
    assert_eq!((waiting.0.lock().unwrap().lists, open.0.lock().unwrap().lists), (2, 1));
}

#[test]
fn the_ports_are_listed_at_start_and_when_they_change() {
    let fake = FakePorts::with(&["COM3"]);
    let watch = PortWatch::spawn(fake.clone(), || {});
    assert_eq!(next_list(&watch), names(&["COM3"]));
    fake.0.lock().unwrap().listed = names(&["COM3", "COM23"]);
    assert_eq!(next_list(&watch), names(&["COM3", "COM23"]), "a port plugged in");
}

#[test]
fn the_watch_of_the_ports_ends_with_the_window() {
    let fake = FakePorts::with(&["COM3"]);
    let watch = PortWatch::spawn(fake.clone(), || {});
    next_list(&watch);
    drop(watch);
    let lists = fake.0.lock().unwrap().lists;
    std::thread::sleep(Duration::from_millis(1300));
    assert_eq!(fake.0.lock().unwrap().lists, lists, "no list after the window is gone");
}

#[test]
fn a_tabs_thread_does_not_list_the_ports() {
    let fake = FakePorts::with(&["COM3"]);
    let link = link(&fake);
    open(&link, "COM3", LinkSettings::default());
    std::thread::sleep(Duration::from_millis(300));
    assert_eq!(fake.0.lock().unwrap().lists, 0);
}

#[test]
fn a_port_opens_with_its_settings_and_what_the_device_sends_comes_through() {
    let fake = FakePorts::default();
    let link = link(&fake);
    let settings = LinkSettings { baud_rate: 9600, ..Default::default() };
    open(&link, "COM7", settings.clone());
    assert_eq!(fake.0.lock().unwrap().opened, [("COM7".to_owned(), settings)]);
    fake.device_sends(b"hello ");
    fake.device_sends(b"world\r\n");
    assert_eq!(next(&link), Event::Data(b"hello ".to_vec()));
    assert_eq!(next(&link), Event::Data(b"world\r\n".to_vec()));
}

#[test]
fn what_is_sent_reaches_the_device_byte_for_byte() {
    let fake = FakePorts::default();
    let link = link(&fake);
    open(&link, "COM7", LinkSettings::default());
    link.send(Command::Send(b"AT+CSQ\r\n".to_vec()));
    link.send(Command::Send(vec![0x03, 0xff]));
    link.send(Command::Close);
    assert_eq!(next(&link), Event::Closed(None));
    assert_eq!(fake.written(), b"AT+CSQ\r\n\x03\xff");
}

#[test]
fn a_port_that_does_not_open_says_why() {
    let fake = FakePorts::default();
    fake.0.lock().unwrap().refuse = Some("Access is denied.".into());
    let link = link(&fake);
    link.send(Command::Open { port: "COM7".into(), link: LinkSettings::default() });
    assert_eq!(next(&link), Event::Refused { port: "COM7".into(), error: "Access is denied.".into() });
    // Nothing is open: a send goes nowhere, and the next opening works.
    link.send(Command::Send(b"lost".to_vec()));
    open(&link, "COM7", LinkSettings::default());
    assert!(fake.written().is_empty());
}

#[test]
fn a_read_that_fails_closes_the_port_and_a_timeout_does_not() {
    let fake = FakePorts::default();
    let link = link(&fake);
    open(&link, "COM7", LinkSettings::default());
    fake.0.lock().unwrap().incoming.push_back(Err(io::Error::new(ErrorKind::TimedOut, "nothing yet")));
    fake.device_sends(b"still here");
    assert_eq!(next(&link), Event::Data(b"still here".to_vec()));
    fake.0.lock().unwrap().incoming.push_back(Err(io::Error::new(ErrorKind::BrokenPipe, "device unplugged")));
    assert_eq!(next(&link), Event::Closed(Some("device unplugged".into())));
}

#[test]
fn a_write_that_fails_closes_the_port() {
    let fake = FakePorts::default();
    let link = link(&fake);
    open(&link, "COM7", LinkSettings::default());
    fake.0.lock().unwrap().write_error = Some(io::Error::new(ErrorKind::PermissionDenied, "write refused"));
    link.send(Command::Send(b"x".to_vec()));
    assert_eq!(next(&link), Event::Closed(Some("write refused".into())));
}

#[test]
fn opening_again_closes_the_first_port_and_takes_the_new_settings() {
    let fake = FakePorts::default();
    let link = link(&fake);
    open(&link, "COM7", LinkSettings::default());
    let slower = LinkSettings { baud_rate: 1200, data_bits: 7, ..Default::default() };
    link.send(Command::Open { port: "COM8".into(), link: slower.clone() });
    assert_eq!(next(&link), Event::Closed(None));
    assert_eq!(next(&link), Event::Opened { port: "COM8".into(), link: slower.clone() });
    assert_eq!(fake.0.lock().unwrap().opened.last(), Some(&("COM8".to_owned(), slower)));
}

#[test]
fn the_window_is_woken_at_each_event() {
    let fake = FakePorts::default();
    let (woken, wakes) = channel();
    let link = Link::spawn(fake.clone(), move || {
        let _ = woken.send(());
    });
    open(&link, "COM7", LinkSettings::default());
    assert!(wakes.recv_timeout(DEADLINE).is_ok());
}

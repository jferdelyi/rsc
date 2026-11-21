//! The thread of a port on a real device, when one is named: run by hand,
//!
//!     RSC_PORT=COM23 cargo test --test real_port -- --ignored --nocapture
//!
//! It opens the port (RSC_BAUD, 115200 by default), shows what comes for RSC_SECONDS (3 by
//! default), sends nothing, and closes it.

use std::time::{Duration, Instant};

use rsc::console::{Console, Line};
use rsc::link::{Command, Event, Link};
use rsc::plot::Plot;
use rsc::ports::{self, PortInfo};
use rsc::settings::LinkSettings;

#[test]
#[ignore = "needs a device: RSC_PORT=<port>"]
fn a_real_port_opens_and_what_it_sends_comes_through() {
    let Some(port) = std::env::var("RSC_PORT").ok() else {
        eprintln!("RSC_PORT not set: nothing to do");
        return;
    };
    let number = |name: &str, default: u64| std::env::var(name).ok().and_then(|value| value.parse().ok()).unwrap_or(default);
    let link_settings = LinkSettings { baud_rate: number("RSC_BAUD", 115_200) as u32, ..LinkSettings::default() };
    let seconds = Duration::from_secs(number("RSC_SECONDS", 3));

    eprintln!("ports: {:?}", ports::available().iter().map(PortInfo::label).collect::<Vec<_>>());
    let link = Link::system(|| {});
    link.send(Command::Open { port: port.clone(), link: link_settings.clone() });
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        match link.next_event(Duration::from_secs(5)) {
            Some(Event::Opened { .. }) => break,
            Some(Event::Refused { error, .. }) => panic!("{port} does not open: {error}"),
            other => assert!(Instant::now() < deadline, "{port}: {other:?}"),
        }
    }
    eprintln!("{port} open at {}", link_settings.summary());

    let mut received = Vec::new();
    let end = Instant::now() + seconds;
    while Instant::now() < end {
        match link.next_event(Duration::from_millis(100)) {
            Some(Event::Data(bytes)) => received.extend(bytes),
            Some(Event::Closed(why)) => panic!("{port} closed: {why:?}"),
            _ => {}
        }
    }
    eprintln!("{} bytes in {seconds:?}:\n{}", received.len(), String::from_utf8_lossy(&received));

    // What the curves would make of it: the lines read as measures, and their curves.
    let mut console = Console::new(100_000);
    console.receive(&received, chrono::Local::now().naive_local());
    let mut plot = Plot::new(100_000);
    let measures: Vec<&str> = console.lines().iter().filter(|line| plot.read(line, "")).map(Line::text).collect();
    let curves: Vec<&str> = plot.curves().iter().map(|curve| curve.name.as_str()).collect();
    eprintln!("{} lines, {} read as measures {measures:?}, curves {curves:?}", console.lines().len(), measures.len());

    link.send(Command::Close);
    loop {
        match link.next_event(Duration::from_secs(5)).expect("closed") {
            Event::Closed(None) => break,
            Event::Closed(Some(why)) => panic!("{port} lost while closing: {why}"),
            _ => {}
        }
    }
    eprintln!("{port} closed");
}

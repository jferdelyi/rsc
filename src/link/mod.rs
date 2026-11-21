//! The threads of the ports. Each tab's port is owned by a thread of its own; one more lists the
//! ports of the machine for every tab. The window never waits for them: it sends commands and reads
//! events.

#[cfg(test)]
pub(crate) mod fake;
mod system;
#[cfg(test)]
mod tests;
mod watch;
mod worker;

use std::io;
use std::sync::mpsc::{Receiver, Sender, channel};
use std::time::Duration;

use crate::ports::PortInfo;
use crate::settings::LinkSettings;
pub use watch::PortWatch;
use system::System;
use worker::Worker;

/// How long a read waits for the device before the commands are looked at again.
const READ_TIMEOUT: Duration = Duration::from_millis(20);

/// An open port, or what plays one in a test.
pub trait Transport: Send {
    /// What the device sent; 0 or a timeout when nothing came in time.
    fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize>;
    fn write_all(&mut self, bytes: &[u8]) -> io::Result<()>;
}

/// What lists the ports and opens them: the system, or a test.
pub trait Ports: Send + 'static {
    fn list(&mut self) -> Vec<PortInfo>;
    fn open(&mut self, port: &str, link: &LinkSettings) -> Result<Box<dyn Transport>, String>;
}

#[derive(Debug)]
pub enum Command {
    Open { port: String, link: LinkSettings },
    Close,
    Send(Vec<u8>),
}

#[derive(Debug, PartialEq, Eq)]
pub enum Event {
    Opened { port: String, link: LinkSettings },
    /// The port could not be opened: why, in the system's words.
    Refused { port: String, error: String },
    Data(Vec<u8>),
    /// The port is closed: as asked, or lost, and then why.
    Closed(Option<String>),
}

/// The window's end of the thread.
pub struct Link {
    commands: Sender<Command>,
    events: Receiver<Event>,
}

impl Link {
    /// Starts the thread. `wake` is called after each event: the window draws again.
    pub fn spawn(ports: impl Ports, wake: impl Fn() + Send + 'static) -> Self {
        let (commands, inbox) = channel();
        let (out, events) = channel();
        std::thread::spawn(move || Worker { ports, out, wake, port: None }.run(inbox));
        Self { commands, events }
    }

    /// The thread on the real ports of the machine.
    pub fn system(wake: impl Fn() + Send + 'static) -> Self {
        Self::spawn(System, wake)
    }

    pub fn send(&self, command: Command) {
        // The thread only ends with the window.
        let _ = self.commands.send(command);
    }

    /// The events that came since the last call.
    pub fn events(&self) -> impl Iterator<Item = Event> + '_ {
        self.events.try_iter()
    }

    /// The next event, waited for at most `within`.
    pub fn next_event(&self, within: Duration) -> Option<Event> {
        self.events.recv_timeout(within).ok()
    }
}

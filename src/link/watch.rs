//! The one thread that lists the ports of the machine for every tab, and how often it does.

use std::sync::mpsc::{Receiver, RecvTimeoutError, Sender, channel};
use std::time::{Duration, Instant};

use super::{Ports, System};
use crate::ports::PortInfo;

/// How often the ports of the machine are listed again: every second while a tab has no open port
/// (a device plugged in shows, a lost port is opened again as soon as it is back), every two when
/// every tab's port is open. A list costs 5.6 ms under Windows (17/11/2026), and the read of an
/// open port already tells when it is lost.
pub(super) fn list_every(all_open: bool) -> Duration {
    Duration::from_secs(if all_open { 2 } else { 1 })
}

/// The one thread that lists the ports of the machine, for every tab: at start, then at the pace
/// of `list_every`. The window gets the list each time it changes.
pub struct PortWatch {
    pace: Sender<bool>,
    lists: Receiver<Vec<PortInfo>>,
    /// The pace last asked for: every tab's port open, or not.
    all_open: Option<bool>,
}

impl PortWatch {
    /// Starts the thread. `wake` is called after each new list: the window draws again.
    pub fn spawn(mut ports: impl Ports, wake: impl Fn() + Send + 'static) -> Self {
        let (pace, inbox) = channel();
        let (out, lists) = channel();
        std::thread::spawn(move || {
            let mut every = list_every(false);
            let mut listed: Option<(Instant, Vec<PortInfo>)> = None;
            loop {
                // Until the next list, the pace may change; the window's end going away ends the
                // thread.
                let due = listed.as_ref().map_or(Duration::ZERO, |(at, _)| (*at + every).saturating_duration_since(Instant::now()));
                match inbox.recv_timeout(due) {
                    Ok(all_open) => {
                        every = list_every(all_open);
                        continue;
                    }
                    Err(RecvTimeoutError::Disconnected) => return,
                    Err(RecvTimeoutError::Timeout) => {}
                }
                let ports = ports.list();
                let changed = listed.as_ref().is_none_or(|(_, known)| *known != ports);
                listed = Some((Instant::now(), ports.clone()));
                if changed {
                    if out.send(ports).is_err() {
                        return;
                    }
                    wake();
                }
            }
        });
        Self { pace, lists, all_open: None }
    }

    /// The thread on the real ports of the machine.
    pub fn system(wake: impl Fn() + Send + 'static) -> Self {
        Self::spawn(System, wake)
    }

    /// The pace of the lists: every two seconds when every tab's port is open, else every second.
    /// Said to the thread only when it changes.
    pub fn pace(&mut self, all_open: bool) {
        if self.all_open != Some(all_open) {
            self.all_open = Some(all_open);
            // The thread only ends with the window.
            let _ = self.pace.send(all_open);
        }
    }

    /// The latest list of the ports, if a new one came since the last call.
    pub fn latest(&self) -> Option<Vec<PortInfo>> {
        self.lists.try_iter().last()
    }
}

//! Ports played in memory, for the tests of the thread and of the window.

use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

use super::*;

/// What the test and the played port share.
#[derive(Default)]
pub(crate) struct Shared {
    /// What the device is about to send, read by read.
    pub incoming: VecDeque<io::Result<Vec<u8>>>,
    /// What was sent to the device.
    pub written: Vec<u8>,
    /// Each port opened, with its settings.
    pub opened: Vec<(String, LinkSettings)>,
    /// The ports of the machine.
    pub listed: Vec<PortInfo>,
    /// How many times they were listed.
    pub lists: usize,
    /// Why the next opening fails, if it must.
    pub refuse: Option<String>,
    /// Why the next write fails, if it must.
    pub write_error: Option<io::Error>,
}

#[derive(Clone, Default)]
pub(crate) struct FakePorts(pub Arc<Mutex<Shared>>);

impl FakePorts {
    pub(crate) fn with(names: &[&str]) -> Self {
        let fake = Self::default();
        fake.0.lock().unwrap().listed =
            names.iter().map(|name| PortInfo { name: (*name).to_owned(), description: String::new(), usb: None }).collect();
        fake
    }

    /// What the device sends next.
    pub(crate) fn device_sends(&self, bytes: &[u8]) {
        self.0.lock().unwrap().incoming.push_back(Ok(bytes.to_vec()));
    }

    pub(crate) fn written(&self) -> Vec<u8> {
        self.0.lock().unwrap().written.clone()
    }
}

impl Ports for FakePorts {
    fn list(&mut self) -> Vec<PortInfo> {
        let mut shared = self.0.lock().unwrap();
        shared.lists += 1;
        shared.listed.clone()
    }

    fn open(&mut self, port: &str, link: &LinkSettings) -> Result<Box<dyn Transport>, String> {
        let mut shared = self.0.lock().unwrap();
        if let Some(error) = shared.refuse.take() {
            return Err(error);
        }
        shared.opened.push((port.to_owned(), link.clone()));
        Ok(Box::new(self.clone()))
    }
}

impl Transport for FakePorts {
    fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        match self.0.lock().unwrap().incoming.pop_front() {
            Some(Ok(bytes)) => {
                buffer[..bytes.len()].copy_from_slice(&bytes);
                Ok(bytes.len())
            }
            Some(Err(error)) => Err(error),
            None => Ok(0),
        }
    }

    fn write_all(&mut self, bytes: &[u8]) -> io::Result<()> {
        let mut shared = self.0.lock().unwrap();
        if let Some(error) = shared.write_error.take() {
            return Err(error);
        }
        shared.written.extend_from_slice(bytes);
        Ok(())
    }
}

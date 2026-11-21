//! The thread of a tab: it owns the tab's port, opens it, reads it, writes to it and closes it,
//! as the window's commands say.

use std::io::ErrorKind;
use std::sync::mpsc::{Receiver, Sender, TryRecvError};

use super::{Command, Event, Ports, READ_TIMEOUT, Transport};

pub(super) struct Worker<P, W> {
    pub(super) ports: P,
    pub(super) out: Sender<Event>,
    pub(super) wake: W,
    pub(super) port: Option<Box<dyn Transport>>,
}

impl<P: Ports, W: Fn()> Worker<P, W> {
    pub(super) fn run(mut self, inbox: Receiver<Command>) {
        loop {
            // An open port is read between two looks at the commands; with none, the thread sleeps
            // until a command comes. Either way the window's end going away ends the thread.
            let command = if self.port.is_some() {
                match inbox.try_recv() {
                    Ok(command) => Some(command),
                    Err(TryRecvError::Empty) => None,
                    Err(TryRecvError::Disconnected) => return,
                }
            } else {
                match inbox.recv() {
                    Ok(command) => Some(command),
                    Err(_) => return,
                }
            };
            // After a command, the port is read too: a file sent in pieces does not hold back what
            // the device says meanwhile.
            if let Some(command) = command {
                self.handle(command);
            }
            self.read();
        }
    }

    fn emit(&self, event: Event) {
        let _ = self.out.send(event);
        (self.wake)();
    }

    fn handle(&mut self, command: Command) {
        match command {
            Command::Open { port, link } => {
                self.close(None);
                match self.ports.open(&port, &link) {
                    Ok(transport) => {
                        self.port = Some(transport);
                        self.emit(Event::Opened { port, link });
                    }
                    Err(error) => self.emit(Event::Refused { port, error }),
                }
            }
            Command::Close => self.close(None),
            Command::Send(bytes) => {
                let Some(port) = &mut self.port else {
                    return;
                };
                if let Err(error) = port.write_all(&bytes) {
                    self.close(Some(error.to_string()));
                }
            }
        }
    }

    fn close(&mut self, why: Option<String>) {
        if self.port.take().is_some() {
            self.emit(Event::Closed(why));
        }
    }

    fn read(&mut self) {
        let Some(port) = &mut self.port else {
            return;
        };
        let mut buffer = [0u8; 4096];
        match port.read(&mut buffer) {
            // Nothing, and at once: without a pause the thread would spin.
            Ok(0) => std::thread::sleep(READ_TIMEOUT),
            Ok(count) => self.emit(Event::Data(buffer[..count].to_vec())),
            Err(error) if matches!(error.kind(), ErrorKind::TimedOut | ErrorKind::WouldBlock | ErrorKind::Interrupted) => {}
            // Unplugged, most of the time.
            Err(error) => self.close(Some(error.to_string())),
        }
    }
}

//! The real ports of the machine, through the `serialport` crate: how a port opens with its
//! settings, how it is read and written.

use std::io;
use std::time::Duration;

use super::{Ports, READ_TIMEOUT, Transport};
use crate::ports::{self, PortInfo};
use crate::settings::{FlowControl, LinkSettings, Parity, StopBits};

/// The ports of the machine.
pub(super) struct System;

impl Ports for System {
    fn list(&mut self) -> Vec<PortInfo> {
        ports::available()
    }

    fn open(&mut self, port: &str, link: &LinkSettings) -> Result<Box<dyn Transport>, String> {
        let data_bits = match link.data_bits {
            5 => serialport::DataBits::Five,
            6 => serialport::DataBits::Six,
            7 => serialport::DataBits::Seven,
            _ => serialport::DataBits::Eight,
        };
        let parity = match link.parity {
            Parity::None => serialport::Parity::None,
            Parity::Even => serialport::Parity::Even,
            Parity::Odd => serialport::Parity::Odd,
        };
        let stop_bits = match link.stop_bits {
            StopBits::One => serialport::StopBits::One,
            StopBits::Two => serialport::StopBits::Two,
        };
        let flow_control = match link.flow_control {
            FlowControl::None => serialport::FlowControl::None,
            FlowControl::Software => serialport::FlowControl::Software,
            FlowControl::Hardware => serialport::FlowControl::Hardware,
        };
        let mut opened = serialport::new(port, opening_speed(port, link.baud_rate, cfg!(target_os = "macos")))
            .data_bits(data_bits)
            .parity(parity)
            .stop_bits(stop_bits)
            .flow_control(flow_control)
            .timeout(READ_TIMEOUT)
            .dtr_on_open(link.dtr)
            .open()
            .map_err(|error| error.to_string())?;
        // The hardware flow control drives RTS itself. As for DTR in the crate, a port that has no
        // such line (a pseudo-terminal) is not refused for it.
        if link.flow_control != FlowControl::Hardware {
            let _ = opened.write_request_to_send(link.rts);
        }
        Ok(Box::new(Serial { port: opened, baud_rate: link.baud_rate }))
    }
}

/// The line speed a port opens with: the link's, but none for a pseudo-terminal of macOS
/// (`/dev/ttys003`, the name `openpty` gives there). The `serialport` crate sets the speed there by
/// an ioctl that a pseudo-terminal answers with ENOTTY, and skips it at 0: a pseudo-terminal has no
/// line, hence no speed. A pseudo-terminal of Linux takes the speed, and so does a real port.
fn opening_speed(port: &str, baud_rate: u32, macos: bool) -> u32 {
    let pseudo_terminal = port.strip_prefix("/dev/ttys").is_some_and(|number| !number.is_empty() && number.bytes().all(|byte| byte.is_ascii_digit()));
    if macos && pseudo_terminal { 0 } else { baud_rate }
}

struct Serial {
    port: Box<dyn serialport::SerialPort>,
    /// The link's speed, even when the port opened without one: the time a write takes.
    baud_rate: u32,
}

impl Transport for Serial {
    fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        io::Read::read(&mut self.port, buffer)
    }

    fn write_all(&mut self, bytes: &[u8]) -> io::Result<()> {
        // The port's timeout is the read's, far too short for a long line at a slow rate: the
        // time the bytes take on the wire, and a second.
        let on_wire = Duration::from_secs_f64(bytes.len() as f64 * 12.0 / f64::from(self.baud_rate.max(1)));
        self.port.set_timeout(on_wire + Duration::from_secs(1))?;
        let written = io::Write::write_all(&mut self.port, bytes).and_then(|()| io::Write::flush(&mut self.port));
        self.port.set_timeout(READ_TIMEOUT)?;
        written
    }
}

#[cfg(test)]
mod tests {
    use super::opening_speed;

    #[test]
    fn a_pseudo_terminal_of_macos_opens_without_a_line_speed() {
        assert_eq!(opening_speed("/dev/ttys003", 115200, true), 0);
        assert_eq!(opening_speed("/dev/cu.usbserial-0001", 115200, true), 115200, "a real port of macOS");
        assert_eq!(opening_speed("/dev/tty.usbmodem1101", 921600, true), 921600, "the other name of a real port");
        assert_eq!(opening_speed("/dev/ttysa", 9600, true), 9600, "not a number");
        assert_eq!(opening_speed("/dev/ttys", 9600, true), 9600, "no number");
        assert_eq!(opening_speed("/dev/ttys003", 115200, false), 115200, "elsewhere, the link's speed");
        assert_eq!(opening_speed("/dev/pts/3", 115200, false), 115200, "a pseudo-terminal of Linux takes it");
    }
}

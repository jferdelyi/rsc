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
        let mut opened = serialport::new(port, link.baud_rate)
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

struct Serial {
    port: Box<dyn serialport::SerialPort>,
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

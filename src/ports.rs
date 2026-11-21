//! The serial ports of the machine, each with what the system says of it, and the USB device behind
//! it: a device found again when it comes back under another name.

use std::cmp::Ordering;

use serde::{Deserialize, Serialize};
use serialport::SerialPortType;

/// What tells a USB device from another: its vendor and product, and its serial number when it has
/// one. Two boards of the same model without a serial number cannot be told apart.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct UsbId {
    pub vid: u16,
    pub pid: u16,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub serial: Option<String>,
}

impl UsbId {
    /// As it is shown: "10C4:EA60 · n° 0001".
    pub fn label(&self) -> String {
        match &self.serial {
            Some(serial) => format!("{:04X}:{:04X} · n° {serial}", self.vid, self.pid),
            None => format!("{:04X}:{:04X}", self.vid, self.pid),
        }
    }

    /// Whether `other` is this device: the same model, and the same serial number when this one
    /// was known.
    pub fn matches(&self, other: &UsbId) -> bool {
        self.vid == other.vid && self.pid == other.pid && (self.serial.is_none() || self.serial == other.serial)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PortInfo {
    /// What opens it: "COM23", "/dev/ttyUSB0", "/dev/cu.usbserial-0001".
    pub name: String,
    /// The product behind it, when the system knows it.
    pub description: String,
    pub usb: Option<UsbId>,
}

impl PortInfo {
    /// What the selector shows: the name, then the product when there is one.
    pub fn label(&self) -> String {
        if self.description.is_empty() { self.name.clone() } else { format!("{} · {}", self.name, self.description) }
    }
}

/// The ports present now, the USB ones first.
pub fn available() -> Vec<PortInfo> {
    let mut ports: Vec<PortInfo> = serialport::available_ports()
        .unwrap_or_default()
        .into_iter()
        .map(|port| {
            let (description, usb) = match port.port_type {
                SerialPortType::UsbPort(usb) => {
                    let product = usb.product.or(usb.manufacturer).unwrap_or_else(|| format!("USB {:04x}:{:04x}", usb.vid, usb.pid));
                    let serial = usb.serial_number.filter(|serial| !serial.trim().is_empty());
                    (without_port_name(&product, &port.port_name), Some(UsbId { vid: usb.vid, pid: usb.pid, serial }))
                }
                SerialPortType::BluetoothPort => ("Bluetooth".to_owned(), None),
                SerialPortType::PciPort | SerialPortType::Unknown => (String::new(), None),
            };
            PortInfo { name: port.port_name, description, usb }
        })
        .collect();
    sort(&mut ports);
    ports
}

/// The port to open for a device known by its port's name and, if it is a USB one, by its identity:
/// its own name while it still carries the same device, else the one port that does. None when the
/// device is absent, or when several ports could be it.
pub fn resolve(name: &str, usb: Option<&UsbId>, ports: &[PortInfo]) -> Option<String> {
    let same_name = ports.iter().find(|port| port.name == name);
    let Some(usb) = usb else {
        return same_name.map(|port| port.name.clone());
    };
    if let Some(port) = same_name
        && port.usb.as_ref().is_none_or(|other| usb.matches(other))
    {
        return Some(port.name.clone());
    }
    let mut candidates = ports.iter().filter(|port| port.usb.as_ref().is_some_and(|other| usb.matches(other)));
    match (candidates.next(), candidates.next()) {
        (Some(port), None) => Some(port.name.clone()),
        _ => None,
    }
}

/// Windows ends a product's name with its port, "USB Serial Device (COM23)": said once is enough.
fn without_port_name(product: &str, port: &str) -> String {
    let product = product.trim();
    product.strip_suffix(&format!("({port})")).map_or(product, str::trim_end).to_owned()
}

fn sort(ports: &mut [PortInfo]) {
    ports.sort_by(|a, b| b.usb.is_some().cmp(&a.usb.is_some()).then_with(|| by_number(&a.name, &b.name)));
}

/// COM3 before COM10: the number a name ends with counts as a number.
fn by_number(a: &str, b: &str) -> Ordering {
    let (a_stem, a_number) = split_number(a);
    let (b_stem, b_number) = split_number(b);
    a_stem.cmp(b_stem).then(a_number.cmp(&b_number)).then_with(|| a.cmp(b))
}

fn split_number(name: &str) -> (&str, Option<u64>) {
    let stem = name.trim_end_matches(|c: char| c.is_ascii_digit());
    (stem, name[stem.len()..].parse().ok())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn usb(vid: u16, pid: u16, serial: Option<&str>) -> UsbId {
        UsbId { vid, pid, serial: serial.map(str::to_owned) }
    }

    fn port(name: &str, description: &str, id: Option<UsbId>) -> PortInfo {
        PortInfo { name: name.into(), description: description.into(), usb: id }
    }

    const CP2102: (u16, u16) = (0x10c4, 0xea60);

    #[test]
    fn usb_ports_come_first_then_the_numbers_in_order() {
        let board = Some(usb(CP2102.0, CP2102.1, None));
        let mut ports = vec![port("COM10", "", None), port("COM3", "", None), port("COM23", "CP2102N", board.clone()), port("COM4", "FT232R", board)];
        sort(&mut ports);
        let names: Vec<&str> = ports.iter().map(|port| port.name.as_str()).collect();
        assert_eq!(names, ["COM4", "COM23", "COM3", "COM10"]);
    }

    #[test]
    fn unix_names_sort_by_their_number_too() {
        let mut ports = vec![port("/dev/ttyUSB10", "", None), port("/dev/ttyACM0", "", None), port("/dev/ttyUSB2", "", None)];
        sort(&mut ports);
        let names: Vec<&str> = ports.iter().map(|port| port.name.as_str()).collect();
        assert_eq!(names, ["/dev/ttyACM0", "/dev/ttyUSB2", "/dev/ttyUSB10"]);
    }

    #[test]
    fn a_label_is_the_name_then_the_product() {
        assert_eq!(port("COM23", "CP2102N USB to UART Bridge Controller", None).label(), "COM23 · CP2102N USB to UART Bridge Controller");
        assert_eq!(port("COM1", "", None).label(), "COM1");
    }

    #[test]
    fn the_port_name_windows_adds_to_a_product_is_dropped() {
        assert_eq!(without_port_name("Silicon Labs CP210x USB to UART Bridge (COM23)", "COM23"), "Silicon Labs CP210x USB to UART Bridge");
        assert_eq!(without_port_name("USB Serial Device (COM2)", "COM23"), "USB Serial Device (COM2)", "another port's name stays");
        assert_eq!(without_port_name("FT232R USB UART", "/dev/ttyUSB0"), "FT232R USB UART");
    }

    #[test]
    fn a_device_shows_its_model_and_serial_number() {
        assert_eq!(usb(CP2102.0, CP2102.1, Some("0001")).label(), "10C4:EA60 · n° 0001");
        assert_eq!(usb(0x0403, 0x6001, None).label(), "0403:6001");
    }

    #[test]
    fn a_device_is_the_same_model_with_the_same_serial_number_when_known() {
        let known = usb(CP2102.0, CP2102.1, Some("0001"));
        assert!(known.matches(&usb(CP2102.0, CP2102.1, Some("0001"))));
        assert!(!known.matches(&usb(CP2102.0, CP2102.1, Some("0002"))));
        assert!(!known.matches(&usb(CP2102.0, CP2102.1, None)));
        assert!(!known.matches(&usb(0x0403, 0x6001, Some("0001"))));
        assert!(usb(CP2102.0, CP2102.1, None).matches(&usb(CP2102.0, CP2102.1, Some("0002"))), "no serial known: the model is enough");
    }

    #[test]
    fn a_port_is_found_by_its_name_without_usb_identity() {
        let ports = [port("COM3", "", None), port("COM23", "", None)];
        assert_eq!(resolve("COM23", None, &ports).as_deref(), Some("COM23"));
        assert_eq!(resolve("COM7", None, &ports), None);
    }

    #[test]
    fn a_usb_device_that_changed_port_is_found_again() {
        let board = usb(CP2102.0, CP2102.1, Some("A1"));
        let ports = [port("COM3", "", None), port("COM24", "CP2102N", Some(board.clone()))];
        assert_eq!(resolve("COM23", Some(&board), &ports).as_deref(), Some("COM24"));
    }

    #[test]
    fn a_name_now_given_to_another_device_is_not_taken() {
        let board = usb(CP2102.0, CP2102.1, Some("A1"));
        let ports = [port("COM23", "", Some(usb(0x0403, 0x6001, None))), port("COM25", "", Some(board.clone()))];
        assert_eq!(resolve("COM23", Some(&board), &ports).as_deref(), Some("COM25"));
        let ports = [port("COM23", "", Some(usb(0x0403, 0x6001, None)))];
        assert_eq!(resolve("COM23", Some(&board), &ports), None, "the device is absent");
    }

    #[test]
    fn its_own_name_wins_and_two_lookalikes_are_not_guessed() {
        let model = usb(CP2102.0, CP2102.1, None);
        let ports = [port("COM23", "", Some(model.clone())), port("COM24", "", Some(model.clone()))];
        assert_eq!(resolve("COM24", Some(&model), &ports).as_deref(), Some("COM24"));
        assert_eq!(resolve("COM9", Some(&model), &ports), None, "two boards of the same model, no serial number");
        // A port whose USB identity the system does not give is taken by its name.
        assert_eq!(resolve("COM9", Some(&model), &[port("COM9", "", None)]).as_deref(), Some("COM9"));
    }
}

//! Each value of a setting said as a person reads it, in the window's language.

use super::{FlowControl, LineEnding, Parity, StopBits};
use crate::plot::XAxis;

/// A value as a person reads it, for the tooltip that gives the general profile's.
pub trait Describe {
    fn describe(&self) -> String;
}

impl Describe for bool {
    fn describe(&self) -> String {
        if *self { t!("oui", "yes") } else { t!("non", "no") }.to_owned()
    }
}

impl Describe for u8 {
    fn describe(&self) -> String {
        self.to_string()
    }
}

impl Describe for u32 {
    fn describe(&self) -> String {
        self.to_string()
    }
}

impl Describe for usize {
    fn describe(&self) -> String {
        self.to_string()
    }
}

impl Describe for f32 {
    fn describe(&self) -> String {
        format!("{self:.1}")
    }
}

impl Describe for String {
    fn describe(&self) -> String {
        if self.is_empty() { t!("vide", "empty").to_owned() } else { self.clone() }
    }
}

impl Describe for Parity {
    fn describe(&self) -> String {
        match self {
            Parity::None => t!("aucune", "none"),
            Parity::Even => t!("paire", "even"),
            Parity::Odd => t!("impaire", "odd"),
        }
        .to_owned()
    }
}

impl Describe for StopBits {
    fn describe(&self) -> String {
        match self {
            StopBits::One => "1",
            StopBits::Two => "2",
        }
        .to_owned()
    }
}

impl Describe for FlowControl {
    fn describe(&self) -> String {
        match self {
            FlowControl::None => t!("aucun", "none"),
            FlowControl::Software => "XON/XOFF",
            FlowControl::Hardware => "RTS/CTS",
        }
        .to_owned()
    }
}

impl Describe for XAxis {
    fn describe(&self) -> String {
        match self {
            XAxis::Time => t!("temps", "time"),
            XAxis::Number => t!("numéro de la mesure", "number of the measure"),
        }
        .to_owned()
    }
}

/// Names, as a list.
impl Describe for Vec<String> {
    fn describe(&self) -> String {
        if self.is_empty() { t!("aucune", "none").to_owned() } else { self.join(", ") }
    }
}

impl Describe for LineEnding {
    fn describe(&self) -> String {
        match self {
            LineEnding::None => t!("rien", "none"),
            LineEnding::Lf => "LF",
            LineEnding::Cr => "CR",
            LineEnding::CrLf => "CR LF",
        }
        .to_owned()
    }
}

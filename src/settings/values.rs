//! The values a setting takes: the baud rates offered and the bounds of the numbers, parity,
//! stop bits, flow control, line ending, theme, language, and a port's link with its summary.

use serde::{Deserialize, Serialize};

/// The rates the selector offers; any other one can be typed.
pub const BAUD_RATES: [u32; 14] =
    [300, 1200, 2400, 4800, 9600, 19_200, 38_400, 57_600, 74_880, 115_200, 230_400, 460_800, 921_600, 1_000_000];

/// The lines the console keeps, at least and at most.
pub const SCROLLBACK: std::ops::RangeInclusive<usize> = 1_000..=1_000_000;

/// The console's font size, at least and at most.
pub const FONT_SIZE: std::ops::RangeInclusive<f32> = 9.0..=28.0;

/// The points each curve keeps, at least and at most.
pub const PLOT_POINTS: std::ops::RangeInclusive<usize> = 1_000..=1_000_000;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Parity {
    #[default]
    None,
    Even,
    Odd,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StopBits {
    #[default]
    One,
    Two,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FlowControl {
    #[default]
    None,
    /// XON/XOFF.
    Software,
    /// RTS/CTS.
    Hardware,
}

/// What follows a line that is sent.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LineEnding {
    None,
    #[default]
    Lf,
    Cr,
    CrLf,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Theme {
    /// The system's, followed when it changes.
    #[default]
    System,
    Light,
    Dark,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Language {
    #[default]
    #[serde(rename = "fr")]
    French,
    #[serde(rename = "en")]
    English,
}

/// How the port is opened: what the thread of a port needs of a profile.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct LinkSettings {
    pub baud_rate: u32,
    /// 5 to 8.
    pub data_bits: u8,
    pub parity: Parity,
    pub stop_bits: StopBits,
    pub flow_control: FlowControl,
    /// The DTR line once the port is open. Some boards restart on it.
    pub dtr: bool,
    /// The RTS line once the port is open, unless the hardware flow control drives it.
    pub rts: bool,
}

impl Default for LinkSettings {
    fn default() -> Self {
        Self {
            baud_rate: 115_200,
            data_bits: 8,
            parity: Parity::None,
            stop_bits: StopBits::One,
            flow_control: FlowControl::None,
            dtr: true,
            rts: true,
        }
    }
}

impl LinkSettings {
    /// The usual short form: "115200 8N1".
    pub fn summary(&self) -> String {
        let parity = match self.parity {
            Parity::None => 'N',
            Parity::Even => 'E',
            Parity::Odd => 'O',
        };
        let stop = match self.stop_bits {
            StopBits::One => 1,
            StopBits::Two => 2,
        };
        format!("{} {}{parity}{stop}", self.baud_rate, self.data_bits)
    }
}

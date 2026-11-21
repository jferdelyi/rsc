//! The settings of a profile, written once in `profile!`: the profile, what differs between two
//! profiles, and the name of each setting; then what they do.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use super::{Describe, FONT_SIZE, FlowControl, LineEnding, LinkSettings, PLOT_POINTS, Parity, SCROLLBACK, StopBits};
use crate::plot::XAxis;

/// The settings of a profile, written once: the profile, what differs between two profiles, and the
/// name of each setting come from this list.
macro_rules! profile {
    ($( $(#[doc = $doc:literal])* $variant:ident $field:ident : $ty:ty = $default:expr; )*) => {
        /// How a device is talked to and shown: the general profile, or a profile of the library.
        #[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
        #[serde(default)]
        pub struct Profile {
            $( $(#[doc = $doc])* pub $field: $ty, )*
        }

        impl Default for Profile {
            fn default() -> Self {
                Self { $( $field: $default, )* }
            }
        }

        /// A setting of a profile.
        #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
        pub enum Field {
            $( $variant, )*
        }

        impl Field {
            /// Every setting, in the order of the list.
            pub const ALL: &'static [Field] = &[ $( Field::$variant, )* ];
        }

        /// What a profile changes of another: a value for each setting that differs, nothing for the
        /// others. A profile is written in the file as what it changes of the defaults.
        #[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
        #[serde(default)]
        pub struct Overrides {
            $( #[serde(skip_serializing_if = "Option::is_none")] pub $field: Option<$ty>, )*
        }

        impl Profile {
            /// This profile with what another changes of it laid over it.
            pub fn with(&self, overrides: &Overrides) -> Self {
                Self { $( $field: overrides.$field.clone().unwrap_or_else(|| self.$field.clone()), )* }
            }

            /// A setting's value, as a person reads it.
            pub fn describe(&self, field: Field) -> String {
                match field {
                    $( Field::$variant => self.$field.describe(), )*
                }
            }
        }

        impl Overrides {
            /// Keeps each setting `edited` changed from `before`.
            pub fn take_changes(&mut self, before: &Profile, edited: &Profile) {
                $( if before.$field != edited.$field { self.$field = Some(edited.$field.clone()); } )*
            }

            /// Every setting of `profile` that differs from `base`.
            pub fn between(base: &Profile, profile: &Profile) -> Self {
                let mut overrides = Self::default();
                overrides.take_changes(base, profile);
                overrides
            }

            /// How many settings are changed.
            pub fn len(&self) -> usize {
                0 $( + usize::from(self.$field.is_some()) )*
            }

            pub fn is_empty(&self) -> bool {
                self.len() == 0
            }
        }
    };
}

profile! {
    BaudRate baud_rate: u32 = 115_200;
    /// 5 to 8.
    DataBits data_bits: u8 = 8;
    Parity parity: Parity = Parity::None;
    StopBits stop_bits: StopBits = StopBits::One;
    FlowControl flow_control: FlowControl = FlowControl::None;
    /// The DTR line once the port is open. Some boards restart on it.
    Dtr dtr: bool = true;
    /// The RTS line once the port is open, unless the hardware flow control drives it.
    Rts rts: bool = true;
    /// What follows a line that is sent.
    LineEnding line_ending: LineEnding = LineEnding::Lf;
    Timestamps timestamps: bool = false;
    AnsiColors ansi_colors: bool = true;
    /// The whole console in miniature at its right.
    Minimap minimap: bool = true;
    /// The terminal mode: the keys go to the device one by one, and its screen is drawn as a
    /// terminal draws it.
    Terminal terminal: bool = false;
    /// A line that is sent is shown in the console.
    LocalEcho local_echo: bool = true;
    /// The lines the console keeps.
    Scrollback scrollback: usize = 10_000;
    FontSize font_size: f32 = 13.0;
    /// The console shows bytes in hexadecimal.
    HexView hex_view: bool = false;
    /// The send line is read as hexadecimal bytes.
    HexInput hex_input: bool = false;
    /// `\n`, `\r`, `\t`, `\\` and `\xNN` are read as such in a line to send.
    Escapes escapes: bool = false;
    /// The curves of the measures, under the console.
    Plot plot: bool = false;
    /// Only the lines that begin with it make curves; empty, every line of values does.
    PlotPrefix plot_prefix: String = String::new();
    /// The points each curve keeps.
    PlotPoints plot_points: usize = 100_000;
    /// What the curves are drawn against: the time, or the number of the line of measures.
    PlotX plot_x: XAxis = XAxis::Time;
    /// The names of the curves drawn in the chart below; empty, a single chart.
    PlotBelow plot_below: Vec<String> = Vec::new();
    /// A port lost is opened again as soon as it comes back.
    Reconnect reconnect: bool = true;
    /// Where journals are made; empty, a file is asked for each time.
    JournalFolder journal_folder: String = String::new();
    /// A journal starts at each connection, in the folder.
    JournalOnConnect journal_on_connect: bool = false;
    /// Each line of the journal bears its time.
    JournalTimestamps journal_timestamps: bool = true;
}

impl Profile {
    /// How its port is opened.
    pub fn link(&self) -> LinkSettings {
        LinkSettings {
            baud_rate: self.baud_rate,
            data_bits: self.data_bits,
            parity: self.parity,
            stop_bits: self.stop_bits,
            flow_control: self.flow_control,
            dtr: self.dtr,
            rts: self.rts,
        }
    }

    pub(super) fn set_link(&mut self, link: &LinkSettings) {
        self.baud_rate = link.baud_rate;
        self.data_bits = link.data_bits;
        self.parity = link.parity;
        self.stop_bits = link.stop_bits;
        self.flow_control = link.flow_control;
        self.dtr = link.dtr;
        self.rts = link.rts;
    }

    /// The folder of the journals, if there is one.
    pub fn journal_folder(&self) -> Option<PathBuf> {
        (!self.journal_folder.trim().is_empty()).then(|| PathBuf::from(&self.journal_folder))
    }

    pub(super) fn sanitized(mut self) -> Self {
        let mut overrides = Overrides::default();
        overrides.take_changes(&Profile::default(), &self);
        overrides.sanitize();
        self = Profile::default().with(&overrides);
        self
    }
}

impl Overrides {
    /// Values brought back within their range.
    pub(super) fn sanitize(&mut self) {
        if self.baud_rate == Some(0) {
            self.baud_rate = Some(Profile::default().baud_rate);
        }
        self.data_bits = self.data_bits.map(|bits| bits.clamp(5, 8));
        self.scrollback = self.scrollback.map(|lines| lines.clamp(*SCROLLBACK.start(), *SCROLLBACK.end()));
        self.plot_points = self.plot_points.map(|points| points.clamp(*PLOT_POINTS.start(), *PLOT_POINTS.end()));
        if let Some(names) = &mut self.plot_below {
            *names = curve_names(std::mem::take(names));
        }
        self.font_size = self.font_size.map(|size| if size.is_finite() { size.clamp(*FONT_SIZE.start(), *FONT_SIZE.end()) } else { 13.0 });
    }
}

/// Names of curves as a profile keeps them: trimmed, none empty, each once, in their order.
fn curve_names(names: Vec<String>) -> Vec<String> {
    let mut kept: Vec<String> = Vec::with_capacity(names.len());
    for name in names {
        let name = name.trim();
        if !name.is_empty() && !kept.iter().any(|known| known == name) {
            kept.push(name.to_owned());
        }
    }
    kept
}

//! The files of the earlier versions, in three forms. The first form ("presets"): settings common
//! to all tabs, a link per tab, presets of a link (with a port and a USB device in its very first
//! files), device profiles pointing to a preset. The "equipments" form: the "device_profiles" form,
//! but for the device profiles, then called "equipments". The "device_profiles" form: a device
//! profile was a device and its profile at once, its profile what it changed of the general one.
//! Their keys keep their names here, as the files wrote them.

use serde::Deserialize;

use super::{Device, Language, LineEnding, LinkSettings, NamedProfile, Overrides, Profile, Settings, Tab, Theme};
use crate::ports::UsbId;

/// A file of the "device_profiles" form, as it was written.
#[derive(Deserialize, Default)]
#[serde(default)]
struct DeviceProfiles {
    #[serde(with = "super::by_differences")]
    general: Profile,
    device_profiles: Vec<OldDeviceProfile>,
    tabs: Vec<OldNamedTab>,
    active: usize,
    theme: Theme,
    language: Language,
}

/// A device profile: a USB device under a name, and what its profile changed of the general one.
#[derive(Deserialize)]
struct OldDeviceProfile {
    name: String,
    usb: UsbId,
    #[serde(default)]
    last_port: String,
    #[serde(default)]
    overrides: Overrides,
}

#[derive(Deserialize, Default)]
#[serde(default)]
struct OldNamedTab {
    port: String,
    device_profile: Option<String>,
}

impl DeviceProfiles {
    /// Each device profile becomes a device of its name. One that changed something of the general
    /// profile also becomes a profile of the library, under that name, with the values it had: the
    /// general profile and what it changed of it; the device is given it. One that changed nothing
    /// stays on the general profile, which it followed.
    /// Every tab and every device keeps its values.
    fn library(self) -> Settings {
        let own = |old: &OldDeviceProfile| !old.overrides.is_empty();
        let profiles = self.device_profiles.iter().filter(|old| own(old)).map(|old| NamedProfile { name: old.name.clone(), values: self.general.with(&old.overrides) }).collect();
        let devices = self.device_profiles.into_iter().map(|old| Device { profile: own(&old).then(|| old.name.clone()), name: old.name, usb: old.usb, last_port: old.last_port }).collect();
        let tabs = self.tabs.into_iter().map(|tab| Tab { port: tab.port, device: tab.device_profile, profile: None }).collect();
        Settings { general: self.general, profiles, devices, tabs, active: self.active, theme: self.theme, language: self.language }
    }
}

/// The settings of a file of the "device_profiles" form, if it reads as one.
pub(super) fn device_profiles(value: serde_json::Value) -> Option<Settings> {
    serde_json::from_value::<DeviceProfiles>(value).ok().map(DeviceProfiles::library)
}

#[derive(Deserialize, Default)]
#[serde(default)]
struct Old {
    tabs: Vec<OldTab>,
    active: usize,
    presets: Vec<OldPreset>,
    equipments: Vec<OldEquipment>,
    local_echo: Option<bool>,
    escapes: Option<bool>,
    timestamps: Option<bool>,
    ansi_colors: Option<bool>,
    minimap: Option<bool>,
    scrollback: Option<usize>,
    font_size: Option<f32>,
    reconnect: Option<bool>,
    plot_prefix: Option<String>,
    plot_points: Option<usize>,
    journal: Option<OldJournal>,
    theme: Theme,
    language: Language,
}

#[derive(Deserialize, Default)]
#[serde(default)]
struct OldTab {
    port: String,
    equipment: Option<String>,
    link: LinkSettings,
    line_ending: LineEnding,
    preset: Option<String>,
    hex_view: bool,
    hex_input: bool,
    plot: bool,
}

#[derive(Deserialize)]
struct OldPreset {
    name: String,
    #[serde(default)]
    link: LinkSettings,
    #[serde(default)]
    line_ending: LineEnding,
    #[serde(default)]
    port: Option<String>,
    #[serde(default)]
    usb: Option<UsbId>,
}

#[derive(Deserialize)]
struct OldEquipment {
    name: String,
    usb: UsbId,
    #[serde(default)]
    last_port: String,
    #[serde(default)]
    preset: Option<String>,
}

#[derive(Deserialize)]
#[serde(default)]
struct OldJournal {
    folder: Option<std::path::PathBuf>,
    on_connect: bool,
    timestamps: bool,
}

impl Default for OldJournal {
    fn default() -> Self {
        Self { folder: None, on_connect: false, timestamps: true }
    }
}

/// The settings of a file of an earlier version, if it reads as one.
pub(super) fn convert(value: serde_json::Value) -> Option<Settings> {
    let mut old: Old = serde_json::from_value(value).ok()?;
    // First version: a preset with a USB device is that device, named after it.
    for preset in &old.presets {
        let Some(usb) = &preset.usb else {
            continue;
        };
        if old.equipments.iter().any(|known| known.name == preset.name || &known.usb == usb) {
            continue;
        }
        old.equipments.push(OldEquipment { name: preset.name.clone(), usb: usb.clone(), last_port: preset.port.clone().unwrap_or_default(), preset: Some(preset.name.clone()) });
        for tab in &mut old.tabs {
            if tab.preset.as_deref() == Some(preset.name.as_str()) {
                tab.equipment = Some(preset.name.clone());
            }
        }
    }
    let known = |name: &Option<String>| name.as_ref().is_some_and(|name| old.equipments.iter().any(|equipment| &equipment.name == name));
    // The general profile: the settings common to all tabs, and the tab in front's link when it
    // is on a bare port (else the first such tab's).
    let mut general = Profile::default();
    let common = &mut general;
    if let Some(value) = old.local_echo {
        common.local_echo = value;
    }
    if let Some(value) = old.escapes {
        common.escapes = value;
    }
    if let Some(value) = old.timestamps {
        common.timestamps = value;
    }
    if let Some(value) = old.ansi_colors {
        common.ansi_colors = value;
    }
    if let Some(value) = old.minimap {
        common.minimap = value;
    }
    if let Some(value) = old.scrollback {
        common.scrollback = value;
    }
    if let Some(value) = old.font_size {
        common.font_size = value;
    }
    if let Some(value) = old.reconnect {
        common.reconnect = value;
    }
    if let Some(value) = &old.plot_prefix {
        common.plot_prefix.clone_from(value);
    }
    if let Some(value) = old.plot_points {
        common.plot_points = value;
    }
    if let Some(journal) = &old.journal {
        common.journal_folder = journal.folder.as_ref().map(|folder| folder.display().to_string()).unwrap_or_default();
        common.journal_on_connect = journal.on_connect;
        common.journal_timestamps = journal.timestamps;
    }
    let bare = old.tabs.get(old.active).filter(|tab| !known(&tab.equipment)).or_else(|| old.tabs.iter().find(|tab| !known(&tab.equipment)));
    if let Some(tab) = bare {
        general.set_link(&tab.link);
        general.line_ending = tab.line_ending;
        general.hex_view = tab.hex_view;
        general.hex_input = tab.hex_input;
        general.plot = tab.plot;
    }
    // Each device profile: the link of its preset, else of its tab, kept where it differs.
    let device_profiles = old
        .equipments
        .iter()
        .map(|equipment| {
            let mut profile = general.clone();
            let tab = old.tabs.iter().find(|tab| tab.equipment.as_deref() == Some(equipment.name.as_str()));
            if let Some(tab) = tab {
                profile.set_link(&tab.link);
                profile.line_ending = tab.line_ending;
                profile.hex_view = tab.hex_view;
                profile.hex_input = tab.hex_input;
                profile.plot = tab.plot;
            }
            if let Some(preset) = equipment.preset.as_ref().and_then(|name| old.presets.iter().find(|preset| &preset.name == name)) {
                profile.set_link(&preset.link);
                profile.line_ending = preset.line_ending;
            }
            OldDeviceProfile { name: equipment.name.clone(), usb: equipment.usb.clone(), last_port: equipment.last_port.clone(), overrides: Overrides::between(&general, &profile) }
        })
        .collect();
    let tabs = old.tabs.iter().map(|tab| OldNamedTab { port: tab.port.clone(), device_profile: tab.equipment.clone().filter(|_| known(&tab.equipment)) }).collect();
    Some(DeviceProfiles { general, device_profiles, tabs, active: old.active, theme: old.theme, language: old.language }.library())
}

/// A file of the "equipments" form in the "device_profiles" form: its "equipments" are the
/// device profiles, a tab's "equipment" its device profile. Nothing else changed.
pub(super) fn renamed(mut value: serde_json::Value) -> serde_json::Value {
    if let Some(root) = value.as_object_mut() {
        if let Some(profiles) = root.remove("equipments") {
            root.insert("device_profiles".to_owned(), profiles);
        }
        for tab in root.get_mut("tabs").and_then(serde_json::Value::as_array_mut).into_iter().flatten().filter_map(serde_json::Value::as_object_mut) {
            if let Some(name) = tab.remove("equipment") {
                tab.insert("device_profile".to_owned(), name);
            }
        }
    }
    value
}

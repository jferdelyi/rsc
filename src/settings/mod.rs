//! What the tool remembers from one run to the next: the general profile, the choices by default; a
//! library of profiles, each named, with all its values; the named devices, each with the profile it
//! is given; the tabs; the window's theme and language. One JSON file, readable and editable by
//! hand. The files of the earlier versions are read too.

mod describe;
mod legacy;
mod profile;
#[cfg(test)]
mod tests;
mod values;

use std::ffi::OsString;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::ports::UsbId;

pub use describe::Describe;
pub use profile::{Field, Overrides, Profile};
pub use values::{BAUD_RATES, FONT_SIZE, FlowControl, Language, LineEnding, LinkSettings, PLOT_POINTS, Parity, SCROLLBACK, StopBits, Theme};

/// The names the general profile bears in the window, which no profile of the library can take.
const GENERAL_NAMES: [&str; 2] = ["Général", "General"];

/// A profile of the library: a name, and all its values.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct NamedProfile {
    pub name: String,
    /// In the file, only what differs from the defaults.
    #[serde(default, with = "by_differences")]
    pub values: Profile,
}

/// A USB device given a name: it is found on whatever port it comes back to, and talked to with the
/// profile it is given.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Device {
    pub name: String,
    pub usb: UsbId,
    /// The port it was last seen on.
    #[serde(default)]
    pub last_port: String,
    /// Its profile of the library, by name; none, the general one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub profile: Option<String>,
}

/// Why a name cannot be given.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum NameError {
    Empty,
    /// Another one already bears it.
    Taken,
    /// The general profile's name.
    Reserved,
    /// The device already has a name: this one.
    Known(String),
    /// None of that name.
    Unknown,
}

/// A tab: a port, or a named device.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Tab {
    /// The port in use, or last used.
    pub port: String,
    /// The named device the tab follows: its port is found by its USB identity, and its profile is
    /// used.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub device: Option<String>,
    /// On a port without a named device, the profile of the library the tab uses; none, the general
    /// one.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub profile: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    /// The choices by default: for the ports and the devices that have no profile of the library.
    /// Always written, even empty: a file without it is one of an earlier version.
    #[serde(with = "by_differences")]
    pub general: Profile,
    /// The library of profiles.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub profiles: Vec<NamedProfile>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub devices: Vec<Device>,
    /// One at least.
    pub tabs: Vec<Tab>,
    /// The tab in front.
    pub active: usize,
    #[serde(skip_serializing_if = "is_default")]
    pub theme: Theme,
    #[serde(skip_serializing_if = "is_default")]
    pub language: Language,
}

fn is_default<T: Default + PartialEq>(value: &T) -> bool {
    *value == T::default()
}

/// A profile in the file, the general one or one of the library: only what differs from the
/// defaults. A profile written in full (the first files of the "equipments" form) reads the same.
mod by_differences {
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    use super::{Overrides, Profile};

    pub fn serialize<S: Serializer>(general: &Profile, serializer: S) -> Result<S::Ok, S::Error> {
        Overrides::between(&Profile::default(), general).serialize(serializer)
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Profile, D::Error> {
        Overrides::deserialize(deserializer).map(|differences| Profile::default().with(&differences))
    }
}

impl Default for Settings {
    fn default() -> Self {
        Self { general: Profile::default(), profiles: Vec::new(), devices: Vec::new(), tabs: vec![Tab::default()], active: 0, theme: Theme::default(), language: Language::default() }
    }
}

/// What `Settings::load` found.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Loaded {
    /// No file yet: the defaults.
    Fresh,
    Read,
    /// A file of an earlier version, taken in: the file as it was is kept there.
    Converted(PathBuf),
    /// A file that could not be read as settings: the defaults, and its content kept there.
    SetAside(PathBuf),
}

impl Settings {
    /// The settings of `path`. A file is never lost: one of an earlier version is copied beside it
    /// before it is written again in this version's form, and one that does not read as settings
    /// is copied beside it before the defaults take its place.
    pub fn load(path: &Path) -> (Self, Loaded) {
        let Ok(bytes) = fs::read(path) else {
            return (Self::default(), Loaded::Fresh);
        };
        let read = serde_json::from_slice::<serde_json::Value>(&bytes).ok().and_then(|value| {
            let settings = match version_of(&value) {
                Version::Current => return serde_json::from_value::<Self>(value).ok().map(|settings| (settings, Loaded::Read)),
                Version::First => legacy::convert(value),
                Version::Equipments => legacy::device_profiles(legacy::renamed(value)),
                Version::DeviceProfiles => legacy::device_profiles(value),
            }?;
            let kept = free_beside(path, "old");
            // Best effort, as for a file set aside.
            let _ = fs::write(&kept, &bytes);
            Some((settings, Loaded::Converted(kept)))
        });
        if let Some((settings, loaded)) = read {
            return (settings.sanitized(), loaded);
        }
        let kept = beside(path, "bad");
        // Best effort: where this fails, saving fails too, and says so.
        let _ = fs::write(&kept, bytes);
        (Self::default(), Loaded::SetAside(kept))
    }

    /// Written beside the file first, then moved over it: never a file cut in half.
    pub fn save(&self, path: &Path) -> io::Result<()> {
        if let Some(folder) = path.parent() {
            fs::create_dir_all(folder)?;
        }
        let text = self.text().map_err(io::Error::other)?;
        let partial = beside(path, "tmp");
        fs::write(&partial, text)?;
        fs::rename(&partial, path)
    }

    /// The file's text, as `save` writes it.
    pub fn text(&self) -> serde_json::Result<String> {
        serde_json::to_string_pretty(self).map(|text| text + "\n")
    }

    /// Whether the file at `path` holds these settings in another form than `save` would write: of an
    /// earlier version, or the general profile written in full. Such a file is written again.
    pub fn written_otherwise(&self, path: &Path) -> bool {
        fs::read_to_string(path).is_ok_and(|text| self.text().is_ok_and(|ours| ours != text))
    }

    fn sanitized(mut self) -> Self {
        self.general = self.general.sanitized();
        let mut profiles: Vec<String> = Vec::new();
        self.profiles.retain(|profile| {
            let keep = !profile.name.trim().is_empty() && !is_general_name(&profile.name) && !profiles.contains(&profile.name);
            if keep {
                profiles.push(profile.name.clone());
            }
            keep
        });
        for profile in &mut self.profiles {
            profile.values = std::mem::take(&mut profile.values).sanitized();
        }
        let known = |name: &Option<String>| name.as_ref().is_some_and(|name| profiles.contains(name));
        let mut devices: Vec<String> = Vec::new();
        self.devices.retain(|device| !devices.contains(&device.name) && {
            devices.push(device.name.clone());
            true
        });
        for device in &mut self.devices {
            if !known(&device.profile) {
                device.profile = None;
            }
        }
        if self.tabs.is_empty() {
            self.tabs.push(Tab::default());
        }
        self.active = self.active.min(self.tabs.len() - 1);
        for tab in &mut self.tabs {
            if tab.device.as_ref().is_some_and(|name| !devices.contains(name)) {
                tab.device = None;
            }
            if !known(&tab.profile) {
                tab.profile = None;
            }
        }
        self
    }

    /// A profile of the library, by its name.
    pub fn find_profile(&self, name: &str) -> Option<&NamedProfile> {
        self.profiles.iter().find(|profile| profile.name == name)
    }

    /// A named device, by its name.
    pub fn find_device(&self, name: &str) -> Option<&Device> {
        self.devices.iter().find(|device| device.name == name)
    }

    /// The named device this USB device is, if it has a name.
    pub fn device_of(&self, usb: &UsbId) -> Option<&Device> {
        self.devices.iter().find(|device| device.usb.matches(usb))
    }

    /// A profile's values: a profile of the library's, by its name; the general one's for none.
    pub fn profile(&self, name: Option<&str>) -> Profile {
        match name.and_then(|name| self.find_profile(name)) {
            Some(profile) => profile.values.clone(),
            None => self.general.clone(),
        }
    }

    /// The profile of the library a tab uses, by its name: its named device's, else the tab's own;
    /// none, the general one.
    pub fn profile_name_of(&self, tab: usize) -> Option<&str> {
        let tab = &self.tabs[tab];
        let name = match tab.device.as_deref().and_then(|name| self.find_device(name)) {
            Some(device) => device.profile.as_deref(),
            None => tab.profile.as_deref(),
        };
        name.filter(|name| self.find_profile(name).is_some())
    }

    /// The values a tab uses.
    pub fn profile_of(&self, tab: usize) -> Profile {
        self.profile(self.profile_name_of(tab))
    }

    /// Keeps a profile's new values: a profile of the library's, by its name; the general one's for
    /// none. A name the library does not know changes nothing.
    pub fn set_profile(&mut self, name: Option<&str>, values: Profile) {
        match name {
            None => self.general = values,
            Some(name) => {
                if let Some(profile) = self.profiles.iter_mut().find(|profile| profile.name == name) {
                    profile.values = values;
                }
            }
        }
    }

    /// Changes the profile a tab uses.
    pub fn edit_tab_profile(&mut self, tab: usize, change: impl FnOnce(&mut Profile)) {
        let before = self.profile_of(tab);
        let mut edited = before.clone();
        change(&mut edited);
        if edited != before {
            let name = self.profile_name_of(tab).map(str::to_owned);
            self.set_profile(name.as_deref(), edited);
        }
    }

    /// A new profile of the library, with the values of `from`, a profile of the library, or of the
    /// general one for none. Its name, trimmed.
    pub fn add_profile(&mut self, name: &str, from: Option<&str>) -> Result<String, NameError> {
        if from.is_some_and(|from| self.find_profile(from).is_none()) {
            return Err(NameError::Unknown);
        }
        let name = self.free_profile_name(name)?;
        let values = self.profile(from);
        self.profiles.push(NamedProfile { name: name.clone(), values });
        Ok(name)
    }

    /// Renames a profile of the library: the devices and the tabs that use it keep it. Its new name,
    /// trimmed.
    pub fn rename_profile(&mut self, old: &str, new: &str) -> Result<String, NameError> {
        if self.find_profile(old).is_none() {
            return Err(NameError::Unknown);
        }
        let new = self.free_profile_name(new)?;
        for profile in &mut self.profiles {
            if profile.name == old {
                profile.name.clone_from(&new);
            }
        }
        for name in self.devices.iter_mut().map(|device| &mut device.profile).chain(self.tabs.iter_mut().map(|tab| &mut tab.profile)) {
            if name.as_deref() == Some(old) {
                *name = Some(new.clone());
            }
        }
        Ok(new)
    }

    /// Deletes a profile of the library: the devices and the tabs that used it take the general one.
    pub fn delete_profile(&mut self, name: &str) {
        self.profiles.retain(|profile| profile.name != name);
        for used in self.devices.iter_mut().map(|device| &mut device.profile).chain(self.tabs.iter_mut().map(|tab| &mut tab.profile)) {
            if used.as_deref() == Some(name) {
                *used = None;
            }
        }
    }

    /// Who uses a profile of the library: the named devices, and how many tabs on a port without a
    /// named device.
    pub fn users_of(&self, name: &str) -> (Vec<&str>, usize) {
        let devices = self.devices.iter().filter(|device| device.profile.as_deref() == Some(name)).map(|device| device.name.as_str()).collect();
        let tabs = self.tabs.iter().filter(|tab| tab.device.as_deref().and_then(|device| self.find_device(device)).is_none() && tab.profile.as_deref() == Some(name)).count();
        (devices, tabs)
    }

    /// Gives a tab a profile of the library, or the general one for none: to its named device, which
    /// keeps it on any port; else to the tab. A name the library does not know is the general one.
    pub fn choose_profile(&mut self, tab: usize, name: Option<&str>) {
        let name = name.filter(|name| self.find_profile(name).is_some()).map(str::to_owned);
        let device = self.tabs[tab].device.clone();
        match device.and_then(|device| self.devices.iter_mut().find(|known| known.name == device)) {
            Some(device) => device.profile = name,
            None => self.tabs[tab].profile = name,
        }
    }

    /// Gives a named device a profile of the library, or the general one for none.
    pub fn give_profile(&mut self, device: &str, name: Option<&str>) {
        let name = name.filter(|name| self.find_profile(name).is_some()).map(str::to_owned);
        if let Some(device) = self.devices.iter_mut().find(|known| known.name == device) {
            device.profile = name;
        }
    }

    /// A new tab on no port yet, put in front.
    pub fn add_tab(&mut self) -> usize {
        self.tabs.push(Tab::default());
        self.active = self.tabs.len() - 1;
        self.active
    }

    /// Closes a tab; the last one stays.
    pub fn close_tab(&mut self, index: usize) {
        if self.tabs.len() > 1 && index < self.tabs.len() {
            self.tabs.remove(index);
            if self.active > index || self.active == self.tabs.len() {
                self.active = self.active.saturating_sub(1);
            }
        }
    }

    /// Names the USB device on a tab's port: the tab follows it from now on, and the device keeps the
    /// profile the tab used.
    pub fn name_device(&mut self, tab: usize, name: &str, usb: UsbId) -> Result<(), NameError> {
        if let Some(known) = self.devices.iter().find(|known| known.usb == usb) {
            return Err(NameError::Known(known.name.clone()));
        }
        let name = self.free_device_name(name)?;
        let profile = self.profile_name_of(tab).map(str::to_owned);
        let port = self.tabs[tab].port.clone();
        self.devices.push(Device { name: name.clone(), usb, last_port: port, profile });
        self.tabs[tab].device = Some(name);
        self.tabs[tab].profile = None;
        Ok(())
    }

    pub fn rename_device(&mut self, old: &str, new: &str) -> Result<(), NameError> {
        let new = self.free_device_name(new)?;
        let device = self.devices.iter_mut().find(|device| device.name == old).ok_or(NameError::Unknown)?;
        device.name.clone_from(&new);
        for tab in &mut self.tabs {
            if tab.device.as_deref() == Some(old) {
                tab.device = Some(new.clone());
            }
        }
        Ok(())
    }

    /// Forgets a named device: the tabs that followed it stay on its last port, with the general profile.
    pub fn forget_device(&mut self, name: &str) {
        self.devices.retain(|device| device.name != name);
        for tab in &mut self.tabs {
            if tab.device.as_deref() == Some(name) {
                tab.device = None;
                tab.profile = None;
            }
        }
    }

    /// Puts a tab on a named device, at its last port.
    pub fn choose_device(&mut self, tab: usize, name: &str) -> bool {
        let Some(port) = self.find_device(name).map(|device| device.last_port.clone()) else {
            return false;
        };
        self.tabs[tab].device = Some(name.to_owned());
        self.tabs[tab].port = port;
        true
    }

    /// Puts a tab on a port: on the named device it carries, when it has a name.
    pub fn choose_port(&mut self, tab: usize, port: &str, usb: Option<&UsbId>) {
        if let Some(name) = usb.and_then(|usb| self.device_of(usb)).map(|device| device.name.clone()) {
            self.tabs[tab].device = Some(name.clone());
            self.seen(&name, port);
        } else {
            self.tabs[tab].device = None;
        }
        port.clone_into(&mut self.tabs[tab].port);
    }

    /// A named device is on that port now.
    pub fn seen(&mut self, name: &str, port: &str) {
        if let Some(device) = self.devices.iter_mut().find(|device| device.name == name) {
            port.clone_into(&mut device.last_port);
        }
    }

    /// A name, trimmed, that no profile of the library bears yet, and not the general one's.
    fn free_profile_name(&self, name: &str) -> Result<String, NameError> {
        let name = name.trim();
        if name.is_empty() {
            Err(NameError::Empty)
        } else if is_general_name(name) {
            Err(NameError::Reserved)
        } else if self.find_profile(name).is_some() {
            Err(NameError::Taken)
        } else {
            Ok(name.to_owned())
        }
    }

    /// A name, trimmed, that no named device bears yet.
    fn free_device_name(&self, name: &str) -> Result<String, NameError> {
        let name = name.trim();
        if name.is_empty() {
            Err(NameError::Empty)
        } else if self.find_device(name).is_some() {
            Err(NameError::Taken)
        } else {
            Ok(name.to_owned())
        }
    }
}

/// Whether a name is the general profile's, in either language and whatever its case.
fn is_general_name(name: &str) -> bool {
    let name = name.trim().to_lowercase();
    GENERAL_NAMES.iter().any(|general| general.to_lowercase() == name)
}

/// `path` with `.suffix` after its whole name: "settings.json.bad".
fn beside(path: &Path, suffix: &str) -> PathBuf {
    let mut name = OsString::from(path.as_os_str());
    name.push(".");
    name.push(suffix);
    PathBuf::from(name)
}

/// `path` with a suffix, the first such name no file bears: ".old", then ".old-2", ".old-3".
fn free_beside(path: &Path, suffix: &str) -> PathBuf {
    let first = beside(path, suffix);
    if !first.exists() {
        return first;
    }
    (2..u32::MAX).map(|n| beside(path, &format!("{suffix}-{n}"))).find(|name| !name.exists()).unwrap_or(first)
}

/// Which version wrote a settings file.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Version {
    Current,
    /// The first form: settings common to all tabs, a link per tab, presets.
    First,
    /// The form of `DeviceProfiles`, the device profiles called "equipments".
    Equipments,
    /// A device profile was a device and its profile at once, its profile what it changed of the
    /// general one.
    DeviceProfiles,
}

/// The version of a settings file, by a key that only it wrote, and always did when it had
/// something to write there. A file with none of them, with the general profile or without, is of
/// this one.
fn version_of(value: &serde_json::Value) -> Version {
    const SETTINGS: [&str; 14] =
        ["presets", "local_echo", "escapes", "timestamps", "auto_scroll", "ansi_colors", "minimap", "scrollback", "font_size", "reconnect", "plot_prefix", "plot_points", "journal", "follow_usb"];
    const TAB: [&str; 7] = ["link", "line_ending", "preset", "hex_view", "hex_input", "plot", "usb"];
    // An entry of "equipments", in the first form.
    const ENTRY: [&str; 1] = ["preset"];
    let holds = |item: &serde_json::Value, keys: &[&str]| keys.iter().any(|key| item.get(key).is_some());
    let one_of = |list: &str, keys: &[&str]| value.get(list).and_then(serde_json::Value::as_array).is_some_and(|items| items.iter().any(|item| holds(item, keys)));
    if holds(value, &SETTINGS) || one_of("tabs", &TAB) || one_of("equipments", &ENTRY) {
        Version::First
    } else if holds(value, &["equipments"]) || one_of("tabs", &["equipment"]) {
        Version::Equipments
    } else if holds(value, &["device_profiles"]) || one_of("tabs", &["device_profile"]) {
        Version::DeviceProfiles
    } else {
        Version::Current
    }
}

/// A folder of its own for a test's files, emptied first: the same from one run to the next, so
/// that runs do not pile folders up.
#[cfg(test)]
pub(crate) fn test_folder(test: &str) -> PathBuf {
    let folder = std::env::temp_dir().join("rsc-tests").join(test);
    let _ = fs::remove_dir_all(&folder);
    fs::create_dir_all(&folder).expect("a folder for the test");
    folder
}

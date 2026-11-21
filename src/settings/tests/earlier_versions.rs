//! The files of the earlier versions, taken in: the first form (presets), at its very first files
//! and later, the device profiles once called equipments, the "device_profiles" form, the file kept
//! beside as it was.

use super::*;

#[test]
fn a_file_with_presets_and_device_profiles_is_taken_in() {
    // The shape of a real file of the first form on 17/11/2026, and a second device profile and a
    // bare tab beside it.
    let folder = test_folder("presets");
    let path = folder.join("settings.json");
    fs::write(
        &path,
        r#"{"tabs":[
                  {"port":"COM23","equipment":"Nano B100042","link":{"baud_rate":115200},"line_ending":"lf","preset":"Usine"},
                  {"port":"COM4","link":{"baud_rate":57600,"parity":"even"},"line_ending":"cr_lf","hex_view":true},
                  {"port":"COM7","equipment":"Balise","link":{"baud_rate":4800},"plot":true}
                ],
                "active":1,
                "presets":[{"name":"Usine","link":{"baud_rate":115200}},{"name":"Lent","link":{"baud_rate":9600}},{"name":"Seul","link":{"baud_rate":300}}],
                "equipments":[
                  {"name":"Nano B100042","usb":{"vid":4292,"pid":60000,"serial":"0A1B"},"last_port":"COM23","preset":"Usine"},
                  {"name":"Balise","usb":{"vid":1659,"pid":8963},"last_port":"COM7","preset":"Lent"}
                ],
                "timestamps":true,"auto_scroll":true,"font_size":9.5,"plot_prefix":"PLOT","journal":{"folder":"C:\\journaux","on_connect":true,"timestamps":false},
                "theme":"dark","language":"fr"}"#,
    )
    .unwrap();
    let (settings, loaded) = Settings::load(&path);
    assert_eq!(loaded, Loaded::Converted(beside(&path, "old")));
    let general = &settings.general;
    assert_eq!((general.baud_rate, general.parity, general.line_ending, general.hex_view), (57_600, Parity::Even, LineEnding::CrLf, true), "the bare tab in front gives the link");
    assert_eq!((general.timestamps, general.font_size, general.plot_prefix.as_str()), (true, 9.5, "PLOT"), "the common settings");
    assert_eq!((general.journal_folder.as_str(), general.journal_on_connect, general.journal_timestamps), ("C:\\journaux", true, false));
    // Each device: a profile of its name, with its preset's values, given to it.
    let board = &settings.find_profile("Nano B100042").unwrap().values;
    assert_eq!((board.baud_rate, board.parity, board.line_ending, board.hex_view), (115_200, Parity::None, LineEnding::Lf, false));
    let beacon = &settings.find_profile("Balise").unwrap().values;
    assert_eq!((beacon.baud_rate, beacon.plot), (9600, true), "the preset's link, the tab's curves");
    assert_eq!(settings.find_device("Balise").and_then(|device| device.profile.as_deref()), Some("Balise"));
    assert_eq!(settings.profile_of(2).baud_rate, 9600);
    assert_eq!(settings.tabs.iter().map(|tab| tab.device.clone()).collect::<Vec<_>>(), [Some("Nano B100042".into()), None, Some("Balise".into())]);
    assert_eq!(settings.theme, Theme::Dark);
    // Written again, in the new form: no preset any more.
    settings.save(&path).unwrap();
    let text = fs::read_to_string(&path).unwrap();
    assert!(!text.contains("preset") && !text.contains("Seul"), "{text}");
    assert_eq!(Settings::load(&path), (settings, Loaded::Read), "read again, the same");
    let _ = fs::remove_dir_all(folder);
}

#[test]
fn a_file_taken_in_is_kept_beside_as_it_was() {
    let folder = test_folder("kept");
    let path = folder.join("settings.json");
    let old = r#"{"tabs":[{"port":"COM4","link":{"baud_rate":4800}}],"presets":[]}"#;
    fs::write(&path, old).unwrap();
    let (settings, loaded) = Settings::load(&path);
    assert_eq!((loaded, settings.general.baud_rate), (Loaded::Converted(folder.join("settings.json.old")), 4800));
    assert_eq!(fs::read_to_string(folder.join("settings.json.old")).unwrap(), old);
    // Another file of an earlier version put back by hand: the first copy is not written over.
    let other = old.replace("4800", "9600");
    fs::write(&path, &other).unwrap();
    let (_, loaded) = Settings::load(&path);
    assert_eq!(loaded, Loaded::Converted(folder.join("settings.json.old-2")));
    assert_eq!(fs::read_to_string(folder.join("settings.json.old")).unwrap(), old, "the first copy as it was");
    assert_eq!(fs::read_to_string(folder.join("settings.json.old-2")).unwrap(), other);
    let _ = fs::remove_dir_all(folder);
}

#[test]
fn a_file_without_the_general_profile_keeps_its_profiles() {
    // Written by hand, as window-check.ps1 does: no "general", which RSC always writes, and nothing
    // of an earlier version.
    let folder = test_folder("no-general");
    let path = folder.join("settings.json");
    fs::write(
        &path,
        r#"{"theme":"dark","tabs":[{"port":"COM27"}],"profiles":[{"name":"Balise GPS","values":{"baud_rate":9600}}],"devices":[{"name":"Balise GPS","usb":{"vid":1659,"pid":8963},"last_port":"COM7","profile":"Balise GPS"}]}"#,
    )
    .unwrap();
    let (settings, loaded) = Settings::load(&path);
    assert_eq!(loaded, Loaded::Read, "a file of this version");
    assert_eq!(settings.find_profile("Balise GPS").map(|profile| profile.values.baud_rate), Some(9600), "its baud rate kept");
    assert_eq!(settings.find_device("Balise GPS").and_then(|device| device.profile.as_deref()), Some("Balise GPS"));
    assert_eq!((settings.general.clone(), settings.theme), (Profile::default(), Theme::Dark));
    let _ = fs::remove_dir_all(folder);
}

#[test]
fn a_file_whose_device_profiles_are_called_equipments_is_taken_in_whole() {
    // As the "equipments" form wrote it, with the general profile, settings of their own and a
    // tab on a device profile.
    let folder = test_folder("equipments");
    let path = folder.join("settings.json");
    let old = r#"{
  "general": {"baud_rate": 9600, "timestamps": true},
  "equipments": [
    {"name": "Carte A1", "usb": {"vid": 4292, "pid": 60000, "serial": "0A1B"}, "last_port": "COM23", "overrides": {"baud_rate": 921600, "line_ending": "cr_lf"}},
    {"name": "Balise", "usb": {"vid": 1659, "pid": 8963}, "last_port": "COM7"}
  ],
  "tabs": [{"port": "COM23", "equipment": "Carte A1"}, {"port": "COM4"}],
  "active": 1,
  "theme": "dark"
}"#;
    fs::write(&path, old).unwrap();
    let (settings, loaded) = Settings::load(&path);
    assert_eq!(loaded, Loaded::Converted(folder.join("settings.json.old")));
    assert_eq!(fs::read_to_string(folder.join("settings.json.old")).unwrap(), old, "the file as it was, beside");
    // Each device profile whole: a device with its name, USB identity and last port, given a profile
    // of its name with the general profile's values and those it changed; one that changed nothing,
    // on the general profile.
    assert_eq!(
        settings.devices,
        [
            Device { name: "Carte A1".into(), usb: UsbId { vid: 4292, pid: 60000, serial: Some("0A1B".into()) }, last_port: "COM23".into(), profile: Some("Carte A1".into()) },
            Device { name: "Balise".into(), usb: UsbId { vid: 1659, pid: 8963, serial: None }, last_port: "COM7".into(), profile: None },
        ]
    );
    let carte = &settings.find_profile("Carte A1").unwrap().values;
    assert_eq!((carte.baud_rate, carte.line_ending, carte.timestamps), (921_600, LineEnding::CrLf, true), "its own, and the general profile's for the rest");
    assert!(settings.find_profile("Balise").is_none(), "nothing of its own: no profile in the library");
    assert_eq!(settings.tabs.iter().map(|tab| (tab.port.as_str(), tab.device.as_deref())).collect::<Vec<_>>(), [("COM23", Some("Carte A1")), ("COM4", None)]);
    assert_eq!((settings.general.baud_rate, settings.general.timestamps, settings.active, settings.theme), (9600, true, 1, Theme::Dark));
    assert_eq!(settings.profile_of(0).baud_rate, 921_600, "the tab uses its device's profile");
    // Written again in this version's form, and read as such.
    settings.save(&path).unwrap();
    let text = fs::read_to_string(&path).unwrap();
    assert!(text.contains("\"profiles\"") && text.contains("\"devices\"") && text.contains("\"device\"") && !text.contains("equipment") && !text.contains("device_profile"), "{text}");
    assert_eq!(Settings::load(&path), (settings, Loaded::Read));
    let _ = fs::remove_dir_all(folder);
}

/// Run once on a real file of the "equipments" or "device_profiles" form, which is copied into a test
/// folder first: nothing is written beside it.
///   RSC_SETTINGS_COPY=<file> cargo test --lib -- --ignored a_real_file_keeps_every_tab_s_values
#[test]
#[ignore = "on a real file, named by RSC_SETTINGS_COPY"]
fn a_real_file_keeps_every_tab_s_values() {
    let source = PathBuf::from(std::env::var("RSC_SETTINGS_COPY").expect("RSC_SETTINGS_COPY names the file"));
    let folder = test_folder("real-file");
    let path = folder.join("settings.json");
    fs::copy(&source, &path).expect("a copy of the file");
    let before: serde_json::Value = serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
    let (settings, loaded) = Settings::load(&path);
    assert_eq!(loaded, Loaded::Converted(folder.join("settings.json.old")));
    // The values each tab used, read from the file as its version read them: the general profile
    // (the file may write it in full), and what the tab's device profile changed of it.
    let general = Profile::default().with(&serde_json::from_value::<Overrides>(before["general"].clone()).expect("a general profile"));
    assert_eq!(settings.general, general, "the general profile");
    let old = before.get("equipments").or_else(|| before.get("device_profiles")).and_then(serde_json::Value::as_array).cloned().unwrap_or_default();
    let changes = |name: &str| {
        old.iter().find(|entry| entry["name"] == name).map(|entry| entry.get("overrides").map_or_else(Overrides::default, |own| serde_json::from_value(own.clone()).expect("what it changed")))
    };
    let tabs = before["tabs"].as_array().expect("tabs");
    assert_eq!(settings.tabs.len(), tabs.len());
    for (i, tab) in tabs.iter().enumerate() {
        let name = tab.get("equipment").or_else(|| tab.get("device_profile")).and_then(serde_json::Value::as_str);
        let values = name.and_then(changes).map_or_else(|| general.clone(), |own| general.with(&own));
        assert_eq!(settings.profile_of(i), values, "tab {i} keeps its values");
        assert_eq!(settings.tabs[i].device.as_deref(), name, "tab {i} on its device");
    }
    // Each device kept under its name, with its USB identity, its last port and its values.
    for entry in &old {
        let name = entry["name"].as_str().expect("a name");
        let device = settings.find_device(name).expect("the device");
        assert_eq!((serde_json::to_value(&device.usb).unwrap(), device.last_port.as_str()), (entry["usb"].clone(), entry["last_port"].as_str().unwrap_or_default()));
        assert_eq!(settings.profile(device.profile.as_deref()), general.with(&changes(name).unwrap_or_default()), "{name}'s values");
    }
    eprintln!(
        "{} profiles, {} devices, {} tabs, {} settings of the general profile",
        settings.profiles.len(),
        settings.devices.len(),
        settings.tabs.len(),
        Overrides::between(&Profile::default(), &settings.general).len()
    );
    let _ = fs::remove_dir_all(folder);
}

#[test]
fn a_device_whose_preset_is_the_general_link_stays_on_the_general_profile() {
    let folder = test_folder("preset-as-general");
    let path = folder.join("settings.json");
    fs::write(
        &path,
        r#"{"tabs":[{"port":"COM23","equipment":"Nano B100042","link":{"baud_rate":115200,"data_bits":8,"parity":"none","stop_bits":"one","flow_control":"none","dtr":true,"rts":true},"line_ending":"lf","preset":"Usine","hex_view":false,"hex_input":false,"plot":false}],
                "active":0,"presets":[{"name":"Usine","link":{"baud_rate":115200,"data_bits":8,"parity":"none","stop_bits":"one","flow_control":"none","dtr":true,"rts":true},"line_ending":"lf"}],
                "equipments":[{"name":"Nano B100042","usb":{"vid":4292,"pid":60000,"serial":"0A1B2C3D4E5F60718293A4B5C6D7E8F9"},"last_port":"COM23","preset":"Usine"}],
                "local_echo":true,"escapes":false,"timestamps":false,"auto_scroll":true,"ansi_colors":true,"minimap":true,"scrollback":10000,"font_size":9.5,
                "reconnect":true,"plot_prefix":"","plot_points":100000,"journal":{"folder":null,"on_connect":false,"timestamps":true},"theme":"system","language":"fr"}"#,
    )
    .unwrap();
    let (settings, loaded) = Settings::load(&path);
    assert_eq!(loaded, Loaded::Converted(beside(&path, "old")));
    // Usine is the general link: the device changed nothing, it stays on the general profile.
    assert_eq!((settings.devices.len(), settings.devices[0].profile.clone(), settings.profiles.len()), (1, None, 0));
    assert_eq!((settings.tabs[0].device.as_deref(), settings.profile_of(0).font_size), (Some("Nano B100042"), 9.5));
    let _ = fs::remove_dir_all(folder);
}

#[test]
fn the_file_of_the_first_version_is_taken_in() {
    let folder = test_folder("first-version");
    let path = folder.join("settings.json");
    fs::write(
        &path,
        r#"{"tabs":[{"port":"COM23","preset":"Carte Nano","usb":{"vid":4292,"pid":60000,"serial":"A1"}},{"port":"COM4","preset":"Sonde","link":{"baud_rate":4800}}],
                "presets":[
                  {"name":"Carte Nano","port":"COM23","usb":{"vid":4292,"pid":60000,"serial":"A1"},"link":{"baud_rate":9600,"dtr":false},"line_ending":"cr"},
                  {"name":"Sonde","port":"COM4","link":{"baud_rate":4800}}
                ],
                "timestamps":true,"follow_usb":true}"#,
    )
    .unwrap();
    let (settings, loaded) = Settings::load(&path);
    assert_eq!(loaded, Loaded::Converted(beside(&path, "old")));
    assert_eq!(settings.devices.len(), 1, "a preset with a device is that device");
    let device = &settings.devices[0];
    assert_eq!((device.name.as_str(), device.usb.clone(), device.last_port.as_str(), device.profile.as_deref()), ("Carte Nano", board("A1"), "COM23", Some("Carte Nano")));
    let values = &settings.find_profile("Carte Nano").unwrap().values;
    assert_eq!((values.baud_rate, values.dtr, values.line_ending), (9600, false, LineEnding::Cr));
    assert_eq!(settings.tabs[0].device.as_deref(), Some("Carte Nano"));
    assert_eq!((settings.tabs[1].device.clone(), settings.general.baud_rate, settings.general.timestamps), (None, 4800, true), "the first bare tab gives the link");
    let _ = fs::remove_dir_all(folder);
}

#[test]
fn a_file_of_device_profiles_is_taken_in_with_the_same_values() {
    // As the "device_profiles" form wrote it: a device profile with what it changed of the general
    // profile, another with nothing of its own, a tab on each and a bare one.
    let folder = test_folder("device-profiles");
    let path = folder.join("settings.json");
    let old = r#"{
  "general": {"font_size": 11.0, "timestamps": true},
  "device_profiles": [
    {"name": "Carte n°3", "usb": {"vid": 4292, "pid": 60000, "serial": "A1"}, "last_port": "COM23", "overrides": {"baud_rate": 921600, "line_ending": "cr_lf"}},
    {"name": "Balise", "usb": {"vid": 1659, "pid": 8963}, "last_port": "COM7"}
  ],
  "tabs": [{"port": "COM23", "device_profile": "Carte n°3"}, {"port": "COM7", "device_profile": "Balise"}, {"port": "COM4"}],
  "active": 2
}"#;
    fs::write(&path, old).unwrap();
    let (mut settings, loaded) = Settings::load(&path);
    assert_eq!(loaded, Loaded::Converted(folder.join("settings.json.old")));
    assert_eq!(fs::read_to_string(folder.join("settings.json.old")).unwrap(), old, "the file as it was, beside");
    // Every tab keeps its values.
    let carte = settings.profile_of(0);
    assert_eq!((carte.baud_rate, carte.line_ending, carte.font_size, carte.timestamps), (921_600, LineEnding::CrLf, 11.0, true));
    assert_eq!((settings.profile_of(1), settings.profile_of(2)), (settings.general.clone(), settings.general.clone()));
    assert_eq!(settings.tabs.iter().map(|tab| tab.device.as_deref()).collect::<Vec<_>>(), [Some("Carte n°3"), Some("Balise"), None]);
    assert_eq!(settings.devices.iter().map(|device| device.profile.as_deref()).collect::<Vec<_>>(), [Some("Carte n°3"), None], "a device given the profile of its name; one that changed nothing, the general one");
    assert_eq!(settings.profiles.iter().map(|profile| profile.name.as_str()).collect::<Vec<_>>(), ["Carte n°3"]);
    assert_eq!(settings.active, 2);
    // From now on a profile keeps its values when the general one changes; a device on the general
    // profile follows it, as it did.
    let mut general = settings.general.clone();
    general.font_size = 14.0;
    settings.set_profile(None, general);
    assert_eq!((settings.profile_of(0).font_size, settings.profile_of(1).font_size, settings.profile_of(2).font_size), (11.0, 14.0, 14.0));
    // Written again in this version's form, and read as such.
    settings.save(&path).unwrap();
    let text = fs::read_to_string(&path).unwrap();
    assert!(text.contains("\"profiles\"") && text.contains("\"devices\"") && !text.contains("device_profile"), "{text}");
    assert_eq!(Settings::load(&path), (settings, Loaded::Read));
    let _ = fs::remove_dir_all(folder);
}

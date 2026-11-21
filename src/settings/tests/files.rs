//! The file: settings back as saved, only what differs, a general profile written in full, no
//! file, an unreadable file set aside.

use super::*;

#[test]
fn settings_come_back_as_saved() {
    let folder = test_folder("round-trip");
    let path = folder.join("deep").join("settings.json");
    let mut settings = with_device();
    settings.theme = Theme::Dark;
    settings.language = Language::English;
    settings.general.plot_prefix = "PLOT".into();
    settings.general.journal_folder = folder.join("journaux").display().to_string();
    settings.edit_tab_profile(0, |profile| {
        profile.baud_rate = 9600;
        profile.flow_control = FlowControl::Hardware;
        profile.line_ending = LineEnding::CrLf;
    });
    settings.add_tab();
    settings.save(&path).expect("saved");
    assert_eq!(Settings::load(&path), (settings, Loaded::Read));
    assert!(!beside(&path, "tmp").exists(), "nothing left beside the file");
    let text = fs::read_to_string(&path).unwrap();
    assert!(text.contains("\"language\": \"en\"") && text.contains("\"theme\": \"dark\""), "{text}");
    let own = &text[text.find("\"values\"").unwrap()..];
    assert!(own.contains("\"baud_rate\": 9600") && !own[..own.find('}').unwrap()].contains("font_size"), "a profile is written by what differs from the defaults: {own}");
    let _ = fs::remove_dir_all(folder);
}

#[test]
fn the_file_holds_only_what_differs_from_the_defaults() {
    let folder = test_folder("differences");
    let path = folder.join("settings.json");
    // Nothing changed: the general profile is written empty, but written, and read as this version's.
    Settings::default().save(&path).expect("saved");
    let value: serde_json::Value = serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
    assert_eq!(value["general"], serde_json::json!({}), "{value}");
    assert!(value.get("theme").is_none() && value.get("language").is_none() && value.get("profiles").is_none() && value.get("devices").is_none(), "{value}");
    assert_eq!(Settings::load(&path), (Settings::default(), Loaded::Read));
    // A setting changed: it alone is written, and comes back.
    let mut settings = Settings::default();
    settings.general.font_size = 9.5;
    settings.save(&path).expect("saved");
    let value: serde_json::Value = serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
    assert_eq!(value["general"], serde_json::json!({ "font_size": 9.5 }));
    assert_eq!(Settings::load(&path), (settings, Loaded::Read));
    let _ = fs::remove_dir_all(folder);
}

#[test]
fn a_general_profile_written_in_full_reads_the_same() {
    // The general profile as the first files of the "equipments" form wrote it: every setting.
    let folder = test_folder("general-in-full");
    let path = folder.join("settings.json");
    let mut full = serde_json::to_value(Profile::default()).unwrap();
    full["baud_rate"] = serde_json::json!(9600);
    fs::write(&path, serde_json::json!({ "general": full, "tabs": [{ "port": "COM23" }], "active": 0, "theme": "system", "language": "fr" }).to_string()).unwrap();
    let (settings, loaded) = Settings::load(&path);
    assert_eq!(loaded, Loaded::Read);
    assert_eq!(settings.general, Profile { baud_rate: 9600, ..Profile::default() });
    // Written again: the difference only.
    settings.save(&path).expect("saved");
    let value: serde_json::Value = serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
    assert_eq!(value["general"], serde_json::json!({ "baud_rate": 9600 }));
    let _ = fs::remove_dir_all(folder);
}

#[test]
fn no_file_gives_the_defaults() {
    let folder = test_folder("fresh");
    assert_eq!(Settings::load(&folder.join("settings.json")), (Settings::default(), Loaded::Fresh));
    let _ = fs::remove_dir_all(folder);
}

#[test]
fn an_unreadable_file_is_set_aside_not_lost() {
    let folder = test_folder("bad");
    let path = folder.join("settings.json");
    fs::write(&path, "{ not json").unwrap();
    let (settings, loaded) = Settings::load(&path);
    let kept = folder.join("settings.json.bad");
    assert_eq!(settings, Settings::default());
    assert_eq!(loaded, Loaded::SetAside(kept.clone()));
    assert_eq!(fs::read_to_string(kept).unwrap(), "{ not json");
    let _ = fs::remove_dir_all(folder);
}

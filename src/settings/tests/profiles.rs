//! A profile: the link by default and its summary, a profile that keeps its values, a bare port's
//! tab, values read in clear, values out of range, the axis and the curves below.

use super::*;

#[test]
fn the_defaults_are_the_usual_link() {
    let settings = Settings::default();
    let general = &settings.general;
    assert_eq!(general.link().summary(), "115200 8N1");
    assert_eq!(general.flow_control, FlowControl::None);
    assert!(general.dtr && general.rts);
    assert_eq!(general.line_ending, LineEnding::Lf);
    assert!(!general.hex_view && !general.hex_input && !general.plot && !general.timestamps && !general.escapes);
    assert!(general.local_echo && general.ansi_colors && general.minimap && general.reconnect && general.journal_timestamps);
    assert_eq!((general.scrollback, general.plot_points, general.journal_folder()), (10_000, 100_000, None));
    assert_eq!((settings.tabs.len(), settings.profiles.len(), settings.devices.len()), (1, 0, 0));
    assert_eq!((settings.theme, settings.language), (Theme::System, Language::French));
}

#[test]
fn the_summary_names_parity_and_stop_bits() {
    let link = LinkSettings { baud_rate: 9600, data_bits: 7, parity: Parity::Even, stop_bits: StopBits::Two, ..Default::default() };
    assert_eq!(link.summary(), "9600 7E2");
    let link = LinkSettings { parity: Parity::Odd, ..Default::default() };
    assert_eq!(link.summary(), "115200 8O1");
}

#[test]
fn a_profile_keeps_its_values_when_the_general_one_changes() {
    let mut settings = with_device();
    settings.edit_tab_profile(0, |profile| profile.baud_rate = 9600);
    assert_eq!(settings.general.baud_rate, 115_200, "the general profile is not touched");
    assert_eq!(settings.find_profile("Carte").map(|profile| profile.values.baud_rate), Some(9600));
    // The general profile changes: the profile keeps all its values, those it never changed too.
    let mut general = settings.general.clone();
    general.font_size = 11.0;
    general.baud_rate = 57_600;
    settings.set_profile(None, general);
    let profile = settings.profile_of(0);
    assert_eq!((profile.font_size, profile.baud_rate), (13.0, 9600));
    assert_eq!(settings.profile(None).baud_rate, 57_600);
    // A name the library does not know changes nothing.
    settings.set_profile(Some("Inconnu"), Profile::default());
    assert_eq!((settings.general.baud_rate, settings.profile_of(0).baud_rate), (57_600, 9600));
}

#[test]
fn a_tab_on_a_bare_port_changes_the_general_profile() {
    let mut settings = with_device();
    let bare = settings.add_tab();
    settings.tabs[bare].port = "COM4".into();
    settings.edit_tab_profile(bare, |profile| profile.hex_view = true);
    assert!(settings.general.hex_view);
    assert!(!settings.profile_of(0).hex_view, "the device's profile keeps its own values");
}

#[test]
fn a_value_reads_as_a_person_reads_it() {
    let profile = Profile { parity: Parity::Even, ..Profile::default() };
    assert_eq!(profile.describe(Field::BaudRate), "115200");
    assert_eq!(profile.describe(Field::Parity), "paire");
    assert_eq!(profile.describe(Field::Dtr), "oui");
    assert_eq!(profile.describe(Field::LineEnding), "LF");
    assert_eq!(profile.describe(Field::JournalFolder), "vide");
    assert_eq!(profile.describe(Field::FontSize), "13.0");
}

#[test]
fn values_out_of_range_are_brought_back() {
    let folder = test_folder("range");
    let path = folder.join("settings.json");
    fs::write(
        &path,
        r#"{"general":{"baud_rate":0,"data_bits":12,"scrollback":3,"font_size":400,"plot_points":5},
                "profiles":[{"name":"E","values":{"scrollback":2000000}},{"name":"E","values":{"baud_rate":9600}},{"name":" "},{"name":"Général"}],
                "devices":[{"name":"D","usb":{"vid":1,"pid":2},"profile":"Absent"},{"name":"D","usb":{"vid":3,"pid":4}}],
                "tabs":[],"active":4}"#,
    )
    .unwrap();
    let (settings, loaded) = Settings::load(&path);
    assert_eq!(loaded, Loaded::Read);
    let general = &settings.general;
    assert_eq!((general.baud_rate, general.data_bits, general.scrollback, general.font_size, general.plot_points), (115_200, 8, 1_000, 28.0, 1_000));
    assert_eq!(settings.profiles.iter().map(|profile| profile.name.as_str()).collect::<Vec<_>>(), ["E"], "one of each name, none empty nor the general one's");
    assert_eq!(settings.profiles[0].values.scrollback, 1_000_000);
    assert_eq!((settings.devices.len(), settings.devices[0].profile.clone()), (1, None), "one of each name, no profile that does not exist");
    assert_eq!((settings.tabs.clone(), settings.active), (vec![Tab::default()], 0), "one tab at least");
    fs::write(&path, r#"{"general":{},"tabs":[{"port":"COM3","device":"Absent","profile":"Absent"}]}"#).unwrap();
    assert_eq!(Settings::load(&path).0.tabs[0], Tab { port: "COM3".into(), device: None, profile: None }, "no device nor profile that does not exist");
    let _ = fs::remove_dir_all(folder);
}

#[test]
fn the_axis_and_the_curves_below_are_kept_by_profile() {
    let folder = test_folder("curves");
    let path = folder.join("settings.json");
    // Without them, as an earlier version wrote: the time, a single chart.
    fs::write(&path, r#"{"general":{"plot":true},"tabs":[{"port":"COM3"}]}"#).unwrap();
    let (settings, _) = Settings::load(&path);
    assert_eq!((settings.general.plot_x, settings.general.plot_below.clone()), (XAxis::Time, Vec::<String>::new()));
    // Names trimmed, none empty, each once.
    fs::write(
        &path,
        r#"{"general":{"plot_x":"number","plot_below":[" pression","","pression","compteur"]},
                "profiles":[{"name":"Carte","values":{"plot_x":"time","plot_below":["temp"]}}],
                "tabs":[{"port":"COM3"},{"port":"COM4","profile":"Carte"}]}"#,
    )
    .unwrap();
    let (settings, loaded) = Settings::load(&path);
    assert_eq!(loaded, Loaded::Read);
    assert_eq!((settings.general.plot_x, settings.general.plot_below.clone()), (XAxis::Number, vec!["pression".to_owned(), "compteur".to_owned()]));
    let own = settings.profile_of(1);
    assert_eq!((own.plot_x, own.plot_below.clone()), (XAxis::Time, vec!["temp".to_owned()]), "the profile's own");
    assert_eq!(settings.general.describe(Field::PlotBelow), "pression, compteur");
    assert_eq!(settings.general.describe(Field::PlotX), "numéro de la mesure");
    settings.save(&path).unwrap();
    assert_eq!(Settings::load(&path).0, settings, "written and read back");
    let _ = fs::remove_dir_all(folder);
}

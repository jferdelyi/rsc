//! The profiles: the profile a tab uses and what changes it, the profile's pill and its menu, the
//! settings window on a profile, the library of profiles and the named devices in the settings.

use super::*;

fn bare_port(name: &str) -> PortInfo {
    PortInfo { name: name.into(), description: String::new(), usb: None }
}

/// Tab 0 on the named device "Sonde" at COM9, given the profile "Mesure"; tab 1 on the bare port
/// COM3; tab 0 in front.
fn sonde_and_bare_port(fake: &FakePorts) -> Settings {
    fake.0.lock().unwrap().listed = vec![usb_port("COM9"), bare_port("COM3")];
    let mut settings = on("COM9");
    settings.name_device(0, "Sonde", board()).unwrap();
    settings.add_profile("Mesure", None).unwrap();
    settings.choose_profile(0, Some("Mesure"));
    let bare = settings.add_tab();
    settings.tabs[bare].port = "COM3".into();
    settings.active = 0;
    settings
}

#[test]
fn a_device_chosen_opens_its_port_with_its_profile() {
    let fake = FakePorts::default();
    fake.0.lock().unwrap().listed = vec![usb_port("COM9"), bare_port("COM3")];
    let mut settings = on("COM9");
    settings.name_device(0, "Sonde", board()).unwrap();
    settings.add_profile("Mesure", None).unwrap();
    settings.choose_profile(0, Some("Mesure"));
    settings.edit_tab_profile(0, |profile| profile.baud_rate = 9600);
    settings.tabs[0] = settings::Tab { port: "COM3".into(), ..Default::default() };
    let path = test_settings("device");
    let mut harness = harness(&fake, settings, "device");
    until(&mut harness, "ports listed", |app| !app.ports.is_empty());
    harness.state_mut().settings.choose_device(0, "Sonde");
    harness.run();
    harness.get_by_label("Connecter").click();
    until(&mut harness, "connected", |app| app.sessions[0].is_open());
    let opened = fake.0.lock().unwrap().opened.clone();
    assert_eq!((opened[0].0.as_str(), opened[0].1.baud_rate), ("COM9", 9600));
    assert_eq!(harness.state().settings.general.baud_rate, 115_200, "the general profile keeps its own");
    let (saved, _) = Settings::load(&path);
    assert_eq!(saved.tabs[0].device.as_deref(), Some("Sonde"), "written at once: {}", path.display());
}

#[test]
fn a_baud_rate_chosen_in_the_bar_changes_the_profile_of_the_tab() {
    let fake = FakePorts::default();
    let settings = sonde_and_bare_port(&fake);
    let mut harness = harness(&fake, settings, "bar-baud");
    harness.run();
    // On the named device's tab: its profile, the general profile as it was.
    harness.get_by_value("115200").click();
    harness.run();
    harness.get_by_label("9600").click();
    harness.run();
    {
        let app = harness.state();
        assert_eq!((app.settings.profile_of(0).baud_rate, app.settings.general.baud_rate), (9600, 115_200));
        assert_eq!(app.settings.profile(Some("Mesure")).baud_rate, 9600);
    }
    // On the bare port's tab: the general profile, which the device's profile does not follow.
    harness.state_mut().settings.active = 1;
    harness.run();
    harness.get_by_value("115200").click();
    harness.run();
    harness.get_by_label("4800").click();
    harness.run();
    let app = harness.state();
    assert_eq!((app.settings.general.baud_rate, app.settings.profile_of(0).baud_rate), (4800, 9600));
}

#[test]
fn a_switch_of_the_status_bar_changes_the_profile_of_the_tab() {
    let fake = FakePorts::default();
    let settings = sonde_and_bare_port(&fake);
    let mut harness = harness(&fake, settings, "bar-switch");
    harness.run();
    harness.get_by_label("Affichage hex").click();
    harness.run();
    {
        let app = harness.state();
        assert!(app.settings.profile(Some("Mesure")).hex_view, "the device's profile");
        assert!(!app.settings.general.hex_view, "the general profile as it was");
        assert!(app.shown(0).hex && !app.shown(1).hex);
    }
    harness.state_mut().settings.active = 1;
    harness.run();
    harness.get_by_label("Horodatage").click();
    harness.run();
    let app = harness.state();
    assert!(app.settings.general.timestamps, "a bare port changes the general profile");
    assert!(!app.shown(0).timestamps, "the device's profile keeps its own values");
}

#[test]
fn the_settings_change_the_profile_chosen_at_their_top() {
    let fake = FakePorts::default();
    let settings = sonde_and_bare_port(&fake);
    let mut harness = harness(&fake, settings, "settings-profile");
    harness.run();
    harness.get_by_label("Réglages").click();
    harness.run();
    assert_eq!(harness.state().window.as_ref().and_then(|window| window.profile.clone()).as_deref(), Some("Mesure"), "opened on the tab's profile");
    harness.get_by_label("« Mesure », utilisé par Sonde. Un réglage changé ici change ce profil seulement.");
    harness.get_by_label("7").click();
    harness.run();
    {
        let app = harness.state();
        assert_eq!((app.settings.profile_of(0).data_bits, app.settings.general.data_bits), (7, 8));
    }
    // A whole profile: no setting of its own to mark, nothing that follows the general one.
    assert!(harness.query_by_label("Revenir au général").is_none() && harness.query_by_label_contains("général : ").is_none());
    // The general profile chosen: the device's profile keeps its value.
    if let Some(window) = &mut harness.state_mut().window {
        window.profile = None;
    }
    harness.run();
    harness.get_by_label("6").click();
    harness.run();
    let app = harness.state();
    assert_eq!((app.settings.general.data_bits, app.settings.profile_of(0).data_bits), (6, 7));
    assert!(harness.query_by_label_contains("remplacé par").is_none());
}

#[test]
fn the_pill_says_which_profile_the_tab_uses_and_opens_its_menu() {
    let fake = FakePorts::default();
    let settings = sonde_and_bare_port(&fake);
    let mut harness = harness(&fake, settings, "pill");
    harness.run();
    harness.get_by_label("Profil Mesure").click();
    harness.run();
    assert!(harness.state().window.is_none(), "a menu, not the settings");
    harness.get_by_label("Le profil choisi va à « Sonde », sur tout port.");
    harness.get_by_label("Modifier dans les réglages…").click();
    harness.run();
    let window = harness.state().window.as_ref().map(|window| (window.profile.clone(), window.section));
    assert_eq!(window, Some((Some("Mesure".to_owned()), Section::Link)), "opened on the profile");
    assert!(harness.query_by_label("Modifier dans les réglages…").is_none(), "the menu is closed");
    assert_eq!(short_name("Carte Nano B100042 rev D", 16), "Carte Nano B100…");
}

#[test]
fn a_long_pill_keeps_the_connection_bar_within_the_window() {
    let fake = FakePorts::default();
    let mut settings = sonde_and_bare_port(&fake);
    settings.rename_profile("Mesure", "Carte Nano B100042 rev D").unwrap();
    let mut harness = harness(&fake, settings, "pill-width");
    harness.run();
    harness.get_by_label("Profil Carte Nano B100042 rev D");
    let long = harness.get_by_label("Connecter").rect().right();
    harness.state_mut().settings.active = 1;
    harness.run();
    harness.get_by_label("Profil général");
    let short = harness.get_by_label("Connecter").rect().right();
    assert!((long - short).abs() < 0.5, "the bar ends at {long} with the long pill, at {short} with the short one");
}

fn other_board() -> UsbId {
    UsbId { vid: 0x10c4, pid: 0xea60, serial: Some("B2".into()) }
}

/// "Sonde" at COM9, its profile "Mesure" at 9600 bauds, on tab 0; on tab 1, in front, another board on
/// COM5 that has no name yet.
fn sonde_and_unnamed_board(fake: &FakePorts) -> Settings {
    fake.0.lock().unwrap().listed =
        vec![usb_port("COM9"), PortInfo { name: "COM5".into(), description: "CP2102N USB to UART Bridge Controller".into(), usb: Some(other_board()) }];
    let mut settings = on("COM9");
    settings.name_device(0, "Sonde", board()).unwrap();
    settings.add_profile("Mesure", None).unwrap();
    settings.choose_profile(0, Some("Mesure"));
    settings.edit_tab_profile(0, |profile| profile.baud_rate = 9600);
    let unnamed = settings.add_tab();
    settings.tabs[unnamed].port = "COM5".into();
    settings
}

#[test]
fn the_pill_of_a_bare_usb_port_names_its_device() {
    let fake = FakePorts::default();
    let settings = sonde_and_unnamed_board(&fake);
    let mut harness = harness(&fake, settings, "pill-name");
    until(&mut harness, "ports listed", |app| !app.ports.is_empty());
    harness.get_by_label("Profil général").click();
    harness.run();
    harness.get_by_label("Le profil choisi reste à cet onglet. Nommez l'appareil pour qu'il le garde sur tout port.");
    harness.get_by_label("Nommer cet appareil…").click();
    harness.run();
    let window = harness.state().window.as_ref().map(|window| (window.section, window.device_name.clone()));
    assert_eq!(window, Some((Section::Devices, "CP2102N USB to UART Bridge Controller".to_owned())), "the Devices, its product offered as its name");
    // A name another device bears: said in the section.
    if let Some(window) = &mut harness.state_mut().window {
        window.device_name = "Sonde".into();
    }
    harness.run();
    harness.get_by_label("Nommer cet appareil").click();
    harness.run();
    harness.get_by_label("Ce nom est déjà pris.");
    if let Some(window) = &mut harness.state_mut().window {
        window.device_name = " Carte Nano n°3 ".into();
    }
    harness.run();
    harness.get_by_label("Nommer cet appareil").click();
    harness.run();
    let app = harness.state();
    let made = app.settings.find_device("Carte Nano n°3").expect("named");
    assert_eq!((made.usb.clone(), made.last_port.as_str(), made.profile.clone()), (other_board(), "COM5", None), "the profile the tab used: the general one");
    assert_eq!((app.settings.tabs[1].device.as_deref(), tab_name(&app.settings.tabs[1]).as_str()), (Some("Carte Nano n°3"), "Carte Nano n°3"));
}

#[test]
fn the_pill_chooses_a_profile_and_makes_one_from_the_one_in_use() {
    let fake = FakePorts::default();
    let settings = sonde_and_unnamed_board(&fake);
    let mut harness = harness(&fake, settings, "pill-choose");
    until(&mut harness, "ports listed", |app| !app.ports.is_empty());
    // A USB port without a name: the tab keeps the profile chosen.
    harness.get_by_label("Profil général").click();
    harness.run();
    harness.get_by_label("Mesure").click();
    harness.run();
    {
        let app = harness.state();
        assert_eq!((app.settings.tabs[1].profile.as_deref(), app.settings.profile_of(1).baud_rate), (Some("Mesure"), 9600));
        assert!(app.settings.devices.iter().all(|device| device.name == "Sonde"), "no device named");
    }
    // A new profile from the one in use, chosen at once; a name taken is said in the menu.
    harness.get_by_label("Profil Mesure").click();
    harness.run();
    harness.state_mut().pill_name = "Mesure".into();
    harness.get_by_label("Créer et choisir ce profil").click();
    harness.run();
    harness.get_by_label("Ce nom est déjà pris.");
    harness.state_mut().pill_name = " Rapide ".into();
    harness.get_by_label("Créer et choisir ce profil").click();
    harness.run();
    let app = harness.state();
    assert_eq!(app.settings.profile(Some("Rapide")), app.settings.profile(Some("Mesure")), "a copy of the profile in use");
    assert_eq!(app.settings.tabs[1].profile.as_deref(), Some("Rapide"));
    harness.get_by_label("Profil Rapide");
    assert!(harness.query_by_label("Créer et choisir ce profil").is_none(), "the menu is closed");
}

#[test]
fn the_pill_of_a_port_without_usb_identity_keeps_a_profile_for_its_tab() {
    let fake = FakePorts::default();
    let mut settings = sonde_and_bare_port(&fake);
    settings.edit_tab_profile(0, |profile| profile.baud_rate = 9600);
    settings.active = 1;
    let mut harness = harness(&fake, settings, "pill-bare");
    until(&mut harness, "ports listed", |app| !app.ports.is_empty());
    harness.get_by_label("Profil général").click();
    harness.run();
    harness.get_by_label("Le profil choisi reste à cet onglet.");
    assert!(harness.query_by_label("Nommer cet appareil…").is_none(), "nothing to name: no USB identity");
    harness.get_by_label("Mesure").click();
    harness.run();
    let app = harness.state();
    assert_eq!((app.settings.tabs[1].profile.as_deref(), app.settings.profile_of(1).baud_rate), (Some("Mesure"), 9600));
    harness.get_by_label("Profil Mesure");
}

#[test]
fn the_pill_of_a_named_device_gives_it_another_profile_or_the_general_one() {
    let fake = FakePorts::default();
    let mut settings = sonde_and_unnamed_board(&fake);
    settings.add_profile("Carte", None).unwrap();
    settings.set_profile(Some("Carte"), Profile { line_ending: LineEnding::CrLf, ..Profile::default() });
    settings.active = 0;
    let mut harness = harness(&fake, settings, "pill-device");
    harness.run();
    harness.get_by_label("Profil Mesure").click();
    harness.run();
    harness.get_by_label("Carte").click();
    harness.run();
    {
        let app = harness.state();
        assert_eq!(app.settings.find_device("Sonde").and_then(|device| device.profile.as_deref()), Some("Carte"), "given to the device");
        assert_eq!((app.settings.profile_of(0).baud_rate, app.settings.profile_of(0).line_ending), (115_200, LineEnding::CrLf));
    }
    harness.get_by_label("Profil Carte").click();
    harness.run();
    harness.get_by_label("Général").click();
    harness.run();
    let app = harness.state();
    assert_eq!(app.settings.find_device("Sonde").map(|device| device.profile.clone()), Some(None));
    assert_eq!(app.settings.profiles.len(), 2, "the profiles stay in the library");
    harness.get_by_label("Profil général");
}

/// The button `name` on the row of the element named `on`: on a card, the buttons of the other
/// cards bear the same names.
fn beside<'a>(harness: &'a Harness<'static, App>, name: &'a str, on: &str) -> egui_kittest::Node<'a> {
    let row = harness.get_by_label(on).rect();
    harness.query_all_by_label(name).find(|node| (node.rect().center().y - row.center().y).abs() < 6.0).unwrap_or_else(|| panic!("« {name} » beside « {on} »"))
}

/// Clicks the button `name` on the row of the element named `on`, brought into view first, as a
/// person scrolls the section down to it.
fn click_beside(harness: &mut Harness<'static, App>, name: &str, on: &str) {
    beside(harness, name, on).scroll_to_me();
    harness.run();
    beside(harness, name, on).click();
    harness.run();
}

#[test]
fn the_profiles_section_makes_duplicates_renames_and_deletes_a_profile() {
    let fake = FakePorts::default();
    let settings = sonde_and_bare_port(&fake);
    let mut harness = harness(&fake, settings, "library");
    harness.state_mut().window = Some(settings_window::Window::at(Section::Profiles, None));
    harness.run();
    harness.get_by_label("Utilisé par Sonde");
    // Made, a copy of the general profile.
    if let Some(window) = &mut harness.state_mut().window {
        window.profile_name = "Lent".into();
    }
    harness.run();
    harness.get_by_label("Créer le profil").click();
    harness.run();
    assert_eq!(harness.state().settings.profile(Some("Lent")), harness.state().settings.general);
    harness.get_by_label("Utilisé par aucun appareil ni onglet");
    // Mesure duplicated: the copy's name, offered to be changed at once.
    click_beside(&mut harness, "Dupliquer", "Mesure");
    let names = |harness: &Harness<'static, App>| harness.state().settings.profiles.iter().map(|profile| profile.name.clone()).collect::<Vec<_>>();
    assert_eq!(names(&harness), ["Mesure", "Lent", "Mesure (copie)"]);
    let renaming = harness.state().window.as_ref().and_then(|window| window.renaming_profile.clone());
    assert_eq!(renaming, Some(("Mesure (copie)".to_owned(), "Mesure (copie)".to_owned())));
    if let Some(window) = &mut harness.state_mut().window {
        window.renaming_profile = Some(("Mesure (copie)".into(), "Rapide".into()));
    }
    harness.run();
    click_beside(&mut harness, "Renommer", "Nouveau nom du profil");
    assert_eq!(names(&harness), ["Mesure", "Lent", "Rapide"]);
    // Unused, deleted at once.
    click_beside(&mut harness, "Supprimer", "Lent");
    assert_eq!(names(&harness), ["Mesure", "Rapide"]);
    // In use: the deletion is confirmed first, in its card.
    click_beside(&mut harness, "Supprimer", "Mesure");
    let asked = "Supprimer « Mesure » ? Il est utilisé par Sonde : ils prendront le profil général.";
    assert_eq!(names(&harness), ["Mesure", "Rapide"], "not yet");
    // The confirmation's button, under the question.
    let confirm = |harness: &Harness<'static, App>| {
        let question = harness.get_by_label(asked).rect();
        harness.query_all_by_label("Supprimer").position(|node| node.rect().top() >= question.bottom()).expect("the confirmation")
    };
    let at = confirm(&harness);
    harness.query_all_by_label("Supprimer").nth(at).expect("the confirmation").scroll_to_me();
    harness.run();
    let at = confirm(&harness);
    harness.query_all_by_label("Supprimer").nth(at).expect("the confirmation").click();
    harness.run();
    let app = harness.state();
    assert_eq!(names(&harness), ["Rapide"]);
    assert_eq!((app.settings.find_device("Sonde").map(|device| device.profile.clone()), app.settings.profile_of(0)), (Some(None), app.settings.general.clone()));
}

#[test]
fn the_devices_section_gives_a_profile_renames_and_forgets_a_device() {
    let fake = FakePorts::default();
    let mut settings = sonde_and_bare_port(&fake);
    settings.add_profile("Lent", None).unwrap();
    let mut harness = harness(&fake, settings, "devices");
    until(&mut harness, "ports listed", |app| !app.ports.is_empty());
    harness.state_mut().window = Some(settings_window::Window::at(Section::Devices, None));
    harness.run();
    harness.get_by_label("Cet onglet suit « Sonde ».");
    harness.get_by_label("branché sur COM9");
    // Its profile, from the library.
    harness.get_by_value("Mesure").click();
    harness.run();
    harness.get_by_label("Lent").click();
    harness.run();
    assert_eq!((harness.state().settings.profile_name_of(0), harness.state().settings.find_device("Sonde").and_then(|device| device.profile.clone())), (Some("Lent"), Some("Lent".to_owned())));
    // Renamed: its tab follows it.
    if let Some(window) = &mut harness.state_mut().window {
        window.renaming_device = Some(("Sonde".into(), "Sonde n°2".into()));
    }
    harness.run();
    harness.get_by_label("Renommer").click();
    harness.run();
    assert_eq!(harness.state().settings.tabs[0].device.as_deref(), Some("Sonde n°2"));
    // Forgotten: its tab stays on its port, with the general profile.
    harness.get_by_label("Oublier").click();
    harness.run();
    let app = harness.state();
    assert!(app.settings.devices.is_empty());
    assert_eq!((app.settings.tabs[0].port.as_str(), app.settings.profile_name_of(0)), ("COM9", None));
}

#[test]
fn the_cross_of_the_settings_stands_off_their_edge() {
    let fake = FakePorts::with(&["COM7"]);
    let mut harness = harness(&fake, on("COM7"), "cross");
    harness.get_by_label("Réglages").click();
    harness.run();
    let window = harness.ctx.memory(|memory| memory.area_rect(egui::Id::new("settings"))).expect("the settings window");
    let cross = harness.get_by_label("Fermer les réglages").rect();
    assert!(window.contains_rect(cross), "{cross:?} in {window:?}");
    assert!(window.right() - cross.right() >= 12.0 && cross.top() - window.top() >= 8.0, "{cross:?} off the edge of {window:?}");
    harness.get_by_label("Fermer les réglages").click();
    harness.run();
    assert!(harness.state().window.is_none());
}

#[test]
fn the_device_of_a_tab_is_named_in_the_settings() {
    let fake = FakePorts::default();
    fake.0.lock().unwrap().listed = vec![usb_port("COM9")];
    let mut harness = harness(&fake, on("COM9"), "name");
    until(&mut harness, "ports listed", |app| !app.ports.is_empty());
    harness.get_by_label("Réglages").click();
    harness.run();
    assert_eq!(harness.state().window.as_ref().map(|window| window.section), Some(Section::Link));
    harness.get_by_label("Appareils").click();
    harness.run();
    assert_eq!(harness.state().window.as_ref().map(|window| window.section), Some(Section::Devices));
    if let Some(window) = &mut harness.state_mut().window {
        window.device_name = "Carte n°3".into();
    }
    harness.run();
    harness.get_by_label("Nommer cet appareil").click();
    harness.run();
    let app = harness.state();
    assert_eq!(app.settings.devices.iter().map(|device| device.name.as_str()).collect::<Vec<_>>(), ["Carte n°3"]);
    assert_eq!(app.settings.tabs[0].device.as_deref(), Some("Carte n°3"));
    assert_eq!(tab_name(&app.settings.tabs[0]), "Carte n°3", "the tab bears its name");
}

#[test]
fn the_button_that_names_a_device_leaves_the_settings_at_their_width() {
    // The settings at their size by default, in both languages. A button too wide would widen
    // the window when its section is shown: the "Close" button, at the window's right, would move.
    for english in [false, true] {
        let fake = FakePorts::default();
        fake.0.lock().unwrap().listed = vec![usb_port("COM9")];
        let mut settings = on("COM9");
        settings.language = if english { settings::Language::English } else { settings::Language::French };
        let mut harness = harness(&fake, settings, "name-width");
        until(&mut harness, "ports listed", |app| !app.ports.is_empty());
        let (name, close) = if english { ("Name this device", "Close") } else { ("Nommer cet appareil", "Fermer") };
        harness.state_mut().window = Some(settings_window::Window::at(Section::Link, None));
        harness.run();
        let before = harness.get_by_label(close).rect().right();
        harness.state_mut().window = Some(settings_window::Window::at(Section::Devices, None));
        harness.run();
        let (name, after) = (harness.get_by_label(name).rect().right(), harness.get_by_label(close).rect().right());
        assert!((after - before).abs() < 0.5 && name <= after + 0.5, "english {english}: the window ends at {before}, then at {after}; the button at {name}");
    }
}

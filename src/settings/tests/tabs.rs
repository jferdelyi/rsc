//! Tabs and named devices: a device followed by its tab, a device or a port chosen, renamed,
//! forgotten; the profile a bare port keeps, the one a named device imposes; a tab closed.

use super::*;

#[test]
fn a_named_device_is_followed_by_its_tab() {
    let mut settings = Settings::default();
    settings.tabs[0].port = "COM23".into();
    assert_eq!(settings.name_device(0, "  Carte Nano n°3 ", board("A1")), Ok(()));
    assert_eq!(settings.devices, [Device { name: "Carte Nano n°3".into(), usb: board("A1"), last_port: "COM23".into(), profile: None }]);
    assert_eq!(settings.tabs[0].device.as_deref(), Some("Carte Nano n°3"));
    assert_eq!(settings.name_device(0, "Autre", board("A1")), Err(NameError::Known("Carte Nano n°3".into())), "a device has one name");
    assert_eq!(settings.name_device(0, "Carte Nano n°3", board("B2")), Err(NameError::Taken));
    assert_eq!(settings.name_device(0, "", board("B2")), Err(NameError::Empty));
}

#[test]
fn choosing_a_device_gives_its_port_and_its_profile() {
    let mut settings = with_device();
    settings.edit_tab_profile(0, |profile| profile.baud_rate = 9600);
    let other = settings.add_tab();
    assert_eq!(settings.profile_of(other).baud_rate, 115_200);
    assert!(settings.choose_device(other, "Carte"));
    assert_eq!((settings.tabs[other].port.as_str(), settings.profile_of(other).baud_rate), ("COM23", 9600));
    assert!(!settings.choose_device(other, "Inconnue"));
}

#[test]
fn choosing_a_port_takes_the_device_on_it_when_it_has_a_name() {
    let mut settings = with_device();
    settings.choose_port(0, "COM5", None);
    assert_eq!((settings.tabs[0].device.clone(), settings.tabs[0].port.as_str()), (None, "COM5"), "a bare port");
    settings.choose_port(0, "COM24", Some(&board("A1")));
    assert_eq!((settings.tabs[0].device.as_deref(), settings.tabs[0].port.as_str()), (Some("Carte"), "COM24"));
    assert_eq!(settings.devices[0].last_port, "COM24", "seen there");
}

#[test]
fn a_renamed_device_stays_followed_and_a_forgotten_one_leaves_its_port() {
    let mut settings = with_device();
    assert_eq!(settings.rename_device("Carte", "Carte n°3"), Ok(()));
    assert_eq!(settings.tabs[0].device.as_deref(), Some("Carte n°3"));
    assert_eq!(settings.profile_name_of(0), Some("Carte"), "with its profile");
    assert_eq!(settings.rename_device("Absente", "X"), Err(NameError::Unknown));
    settings.forget_device("Carte n°3");
    assert!(settings.devices.is_empty());
    assert_eq!((settings.tabs[0].device.clone(), settings.tabs[0].port.as_str()), (None, "COM23"));
    assert_eq!(settings.profile_name_of(0), None, "the general profile");
    assert!(settings.find_profile("Carte").is_some(), "the profile stays in the library");
}

#[test]
fn a_bare_port_keeps_its_profile_and_a_named_device_imposes_its_own() {
    let mut settings = with_device();
    settings.add_profile("Lent", None).unwrap();
    settings.edit_tab_profile(0, |profile| profile.baud_rate = 921_600);
    // A port without a named device: the tab keeps the profile chosen for it.
    settings.choose_port(0, "COM5", None);
    settings.choose_profile(0, Some("Lent"));
    assert_eq!((settings.tabs[0].profile.as_deref(), settings.profile_of(0).baud_rate), (Some("Lent"), 115_200));
    // The named device back: its profile, not the tab's.
    settings.choose_port(0, "COM23", Some(&board("A1")));
    assert_eq!((settings.profile_name_of(0), settings.profile_of(0).baud_rate), (Some("Carte"), 921_600));
    // A bare port again: the tab's.
    settings.choose_port(0, "COM5", None);
    assert_eq!(settings.profile_name_of(0), Some("Lent"));
}

#[test]
fn a_named_device_takes_the_profile_its_tab_used() {
    let mut settings = Settings::default();
    settings.tabs[0].port = "COM9".into();
    settings.add_profile("Lent", None).unwrap();
    settings.choose_profile(0, Some("Lent"));
    settings.name_device(0, "Sonde", board("B2")).unwrap();
    assert_eq!((settings.devices[0].profile.as_deref(), settings.tabs[0].profile.clone()), (Some("Lent"), None), "the device keeps it, the tab no longer");
    assert_eq!(settings.profile_name_of(0), Some("Lent"));
}

#[test]
fn a_device_given_a_profile_keeps_it_on_any_port() {
    let mut settings = with_device();
    settings.add_profile("Rapide", None).unwrap();
    settings.edit_tab_profile(0, |profile| profile.baud_rate = 9600);
    settings.choose_profile(0, Some("Rapide"));
    assert_eq!(settings.devices[0].profile.as_deref(), Some("Rapide"), "given to the device, not kept by the tab");
    assert_eq!(settings.tabs[0].profile, None);
    let other = settings.add_tab();
    settings.choose_port(other, "COM30", Some(&board("A1")));
    assert_eq!(settings.profile_name_of(other), Some("Rapide"), "on another port, in another tab");
    settings.give_profile("Carte", Some("Carte"));
    assert_eq!((settings.profile_of(0).baud_rate, settings.profile_of(other).baud_rate), (9600, 9600));
    settings.give_profile("Carte", Some("Inconnu"));
    assert_eq!(settings.devices[0].profile, None, "a name the library does not know: the general profile");
}

#[test]
fn closing_a_tab_keeps_the_one_in_front_when_it_can() {
    let mut settings = Settings::default();
    settings.tabs[0].port = "A".into();
    settings.add_tab();
    settings.tabs[1].port = "B".into();
    settings.add_tab();
    settings.tabs[2].port = "C".into();
    settings.active = 1;
    settings.close_tab(0);
    assert_eq!((settings.tabs.len(), settings.tabs[settings.active].port.as_str()), (2, "B"));
    settings.close_tab(1);
    assert_eq!(settings.tabs[settings.active].port, "B");
    settings.close_tab(0);
    assert_eq!(settings.tabs.len(), 1, "the last tab stays");
}

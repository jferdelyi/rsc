//! The library of profiles: a profile made under a free name, duplicated, renamed, deleted, and the
//! general profile chosen again.

use super::*;

#[test]
fn a_profile_is_made_from_the_general_one_under_a_free_name() {
    let mut settings = Settings::default();
    settings.general.font_size = 11.0;
    assert_eq!(settings.add_profile("  Lent ", None), Ok("Lent".to_owned()));
    assert_eq!(settings.find_profile("Lent").map(|profile| profile.values.clone()), Some(settings.general.clone()), "a copy of the general profile");
    assert_eq!(settings.add_profile("Lent", None), Err(NameError::Taken));
    assert_eq!(settings.add_profile(" ", None), Err(NameError::Empty));
    for general in ["Général", "general", " GÉNÉRAL "] {
        assert_eq!(settings.add_profile(general, None), Err(NameError::Reserved), "{general}");
    }
    assert_eq!(settings.profiles.len(), 1);
}

#[test]
fn a_duplicated_profile_takes_every_value_of_the_other() {
    let mut settings = with_device();
    settings.edit_tab_profile(0, |profile| {
        profile.baud_rate = 9600;
        profile.line_ending = LineEnding::CrLf;
    });
    assert_eq!(settings.add_profile("Sonde", Some("Carte")), Ok("Sonde".to_owned()));
    let (carte, sonde) = (settings.profile(Some("Carte")), settings.profile(Some("Sonde")));
    assert_eq!(sonde, carte, "every value");
    assert_eq!((sonde.baud_rate, sonde.line_ending), (9600, LineEnding::CrLf));
    // A copy: what one changes later stays its own.
    let mut slower = sonde.clone();
    slower.baud_rate = 4800;
    settings.set_profile(Some("Sonde"), slower);
    assert_eq!((settings.profile(Some("Carte")).baud_rate, settings.profile(Some("Sonde")).baud_rate), (9600, 4800));
    assert_eq!(settings.add_profile("Autre", Some("Inconnue")), Err(NameError::Unknown));
    assert_eq!(settings.profiles.len(), 2, "nothing made");
}

#[test]
fn a_renamed_profile_stays_with_its_devices_and_tabs() {
    let mut settings = with_device();
    let bare = settings.add_tab();
    settings.tabs[bare].port = "COM4".into();
    settings.choose_profile(bare, Some("Carte"));
    assert_eq!(settings.rename_profile("Carte", " Lent "), Ok("Lent".to_owned()));
    assert_eq!((settings.devices[0].profile.as_deref(), settings.tabs[bare].profile.as_deref()), (Some("Lent"), Some("Lent")));
    assert_eq!((settings.profile_name_of(0), settings.profile_name_of(bare)), (Some("Lent"), Some("Lent")));
    settings.add_profile("Rapide", None).unwrap();
    assert_eq!(settings.rename_profile("Lent", "Rapide"), Err(NameError::Taken));
    assert_eq!(settings.rename_profile("Lent", "General"), Err(NameError::Reserved));
    assert_eq!(settings.rename_profile("Inconnu", "X"), Err(NameError::Unknown));
    assert!(settings.find_profile("Lent").is_some(), "unchanged");
}

#[test]
fn a_deleted_profile_leaves_the_general_one_to_those_that_used_it() {
    let mut settings = with_device();
    settings.edit_tab_profile(0, |profile| profile.baud_rate = 9600);
    let bare = settings.add_tab();
    settings.tabs[bare].port = "COM4".into();
    settings.choose_profile(bare, Some("Carte"));
    // A tab on the device counts with it, not on its own.
    let following = settings.add_tab();
    settings.choose_device(following, "Carte");
    assert_eq!(settings.users_of("Carte"), (vec!["Carte"], 1));
    settings.delete_profile("Carte");
    assert!(settings.find_profile("Carte").is_none());
    assert_eq!((settings.devices[0].profile.clone(), settings.tabs[bare].profile.clone()), (None, None));
    assert_eq!((settings.profile_of(0), settings.profile_of(bare)), (settings.general.clone(), settings.general.clone()));
    assert_eq!(settings.users_of("Carte"), (Vec::<&str>::new(), 0));
}

#[test]
fn the_general_profile_is_chosen_again_from_the_list() {
    let mut settings = with_device();
    settings.edit_tab_profile(0, |profile| profile.hex_view = true);
    settings.choose_profile(0, None);
    assert_eq!((settings.devices[0].profile.clone(), settings.profile_of(0)), (None, settings.general.clone()));
    assert!(settings.profile(Some("Carte")).hex_view, "the profile stays in the library, with its values");
    settings.choose_profile(0, Some("Carte"));
    settings.choose_profile(0, Some("Inconnue"));
    assert_eq!(settings.profile_name_of(0), None, "a name the library does not know: the general profile");
}

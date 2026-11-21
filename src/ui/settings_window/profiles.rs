//! The section of the profiles in the settings, the library: the general profile first, a new
//! profile under a name, then a card for each profile, which says who uses it, and changes,
//! duplicates, renames or deletes it.

use eframe::egui::{self, Align, Layout, RichText};

use super::{CHANGE_PROFILE_WIDTH, CONTROL_HEIGHT, KEEP_WIDTH, NAME_WIDTH, Palette, SHORT, Section, Tone, Window, icons, link_words, name_problem, theme, widgets};
use super::rows::{card, plain_row, problem_line};
use crate::settings::{NamedProfile, Settings};

/// What a card of a profile asks for, done once every card is drawn.
enum CardAsked {
    Open(Option<String>),
    Duplicate(String),
    Rename(String, String),
    Delete(String),
    ConfirmDelete(String),
}

/// Who uses a profile of the library, in words: "utilisé par Carte n°3, Carte n°4 et 1 onglet".
pub(in crate::ui) fn users_words(settings: &Settings, name: &str) -> String {
    let (devices, tabs) = settings.users_of(name);
    let tabs = match tabs {
        0 => None,
        1 => Some(t!("1 onglet", "1 tab").to_owned()),
        count => Some(tr!("{count} onglets", "{count} tabs")),
    };
    let mut users: Vec<String> = devices.into_iter().map(str::to_owned).collect();
    users.extend(tabs);
    match users.as_slice() {
        [] => t!("utilisé par aucun appareil ni onglet", "used by no device nor tab").to_owned(),
        [one] => tr!("utilisé par {one}", "used by {one}"),
        [first @ .., last] => {
            let first = first.join(", ");
            tr!("utilisé par {first} et {last}", "used by {first} and {last}")
        }
    }
}

pub(super) fn profiles(ui: &mut egui::Ui, window: &mut Window, settings: &mut Settings) {
    let muted = Palette::of(ui.visuals().dark_mode).muted;
    let mut asked = None;
    general_card(ui, settings, &mut asked);
    ui.add_space(8.0);
    new_profile(ui, window, settings);
    problem_line(ui, window.problem.as_deref());
    ui.add_space(8.0);
    if settings.profiles.is_empty() {
        ui.label(RichText::new(t!(
            "Aucun profil dans la bibliothèque pour l'instant : les ports et les appareils utilisent le profil général.",
            "No profile in the library yet: the ports and the devices use the general profile."
        ))
        .color(muted));
    }
    for profile in &settings.profiles {
        if let Some(card_asked) = profile_card(ui, window, settings, profile) {
            asked = Some(card_asked);
        }
        ui.add_space(6.0);
    }
    match asked {
        Some(CardAsked::Open(name)) => {
            window.profile = name;
            window.section = Section::Link;
        }
        Some(CardAsked::Duplicate(name)) => {
            let copy = copy_name(settings, &name);
            match settings.add_profile(&copy, Some(&name)) {
                // Its name, to be changed at once.
                Ok(copy) => {
                    window.renaming_profile = Some((copy.clone(), copy));
                    window.problem = None;
                }
                Err(error) => window.problem = Some(name_problem(error)),
            }
        }
        Some(CardAsked::Rename(old, new)) => match settings.rename_profile(&old, &new) {
            Ok(new) => {
                if window.profile.as_deref() == Some(old.as_str()) {
                    window.profile = Some(new);
                }
                window.renaming_profile = None;
                window.problem = None;
            }
            Err(error) => window.problem = Some(name_problem(error)),
        },
        // Deleted at once when nothing uses it; else confirmed first, in its card.
        Some(CardAsked::Delete(name)) => {
            let (devices, tabs) = settings.users_of(&name);
            if devices.is_empty() && tabs == 0 {
                delete(window, settings, &name);
            } else {
                window.deleting = Some(name);
            }
        }
        Some(CardAsked::ConfirmDelete(name)) => delete(window, settings, &name),
        None => {}
    }
}

fn delete(window: &mut Window, settings: &mut Settings, name: &str) {
    settings.delete_profile(name);
    if window.profile.as_deref() == Some(name) {
        window.profile = None;
    }
    window.deleting = None;
    window.problem = None;
}

/// The name a duplicate of `name` takes first: "Lent (copie)", then "Lent (copie 2)".
fn copy_name(settings: &Settings, name: &str) -> String {
    let first = tr!("{name} (copie)", "{name} (copy)");
    if settings.find_profile(&first).is_none() {
        return first;
    }
    (2..).map(|n| tr!("{name} (copie {n})", "{name} (copy {n})")).find(|copy| settings.find_profile(copy).is_none()).unwrap_or(first)
}

/// The general profile's card: the choices by default; changed, never renamed nor deleted.
fn general_card(ui: &mut egui::Ui, settings: &Settings, asked: &mut Option<CardAsked>) {
    let muted = Palette::of(ui.visuals().dark_mode).muted;
    card(ui, |ui| {
        ui.horizontal(|ui| {
            ui.label(RichText::new(t!("Général", "General")).strong());
            ui.label(RichText::new(t!("par défaut : pour les ports et les appareils sans profil", "by default: for the ports and the devices without a profile")).color(muted));
        });
        plain_row(ui, t!("Liaison", "Link"), t!("Comment le port est ouvert, et ce qui suit chaque ligne envoyée.", "How the port is opened, and what follows each line sent."), |ui| {
            words_then_button(ui, &link_words(&settings.general), None, asked);
        });
    });
}

/// The words of a card's row, then the button that changes the profile: the general one for none.
fn words_then_button(ui: &mut egui::Ui, words: &str, profile: Option<&str>, asked: &mut Option<CardAsked>) {
    let muted = Palette::of(ui.visuals().dark_mode).muted;
    // The words on as many rows as they need, never cut short (WCAG 1.4.4). Of a fixed width: the
    // button stands at the same place on every card.
    ui.scope(|ui| {
        let width = (ui.available_width() - CHANGE_PROFILE_WIDTH).max(80.0);
        ui.set_min_width(width);
        ui.set_max_width(width);
        ui.add(egui::Label::new(RichText::new(words).color(muted)).wrap());
    });
    let hint = t!("Choisir ce profil en haut, et ouvrir la Liaison", "Choose this profile at the top, and open the Link");
    if theme::secondary_button(ui, t!("Modifier", "Change"), true).on_hover_text(hint).clicked() {
        *asked = Some(CardAsked::Open(profile.map(str::to_owned)));
    }
}

/// A new profile of the library: its name, then a copy of the general profile.
fn new_profile(ui: &mut egui::Ui, window: &mut Window, settings: &mut Settings) {
    plain_row(
        ui,
        t!("Nouveau profil", "New profile"),
        t!("Une copie du profil général, sous ce nom. « Dupliquer », sur une carte, copie un autre profil.", "A copy of the general profile, under this name. \"Duplicate\", on a card, copies another profile."),
        |ui| {
            let make = t!("Créer le profil", "Make the profile");
            let field = (ui.available_width() - KEEP_WIDTH - ui.spacing().item_spacing.x).clamp(SHORT, NAME_WIDTH);
            let response = ui.add(
                egui::TextEdit::singleline(&mut window.profile_name)
                    .hint_text(t!("ESP32 921600", "ESP32 921600"))
                    .desired_width(field)
                    .min_size(egui::vec2(field, CONTROL_HEIGHT))
                    .vertical_align(Align::Center),
            );
            widgets::name(&response, t!("Nom du nouveau profil", "Name of the new profile"));
            let enter = response.lost_focus() && ui.input(|input| input.key_pressed(egui::Key::Enter));
            let named = !window.profile_name.trim().is_empty();
            let clicked = theme::primary_button(ui, make, KEEP_WIDTH, named).on_hover_text(t!("Une copie du profil général, à modifier ensuite", "A copy of the general profile, to be changed next")).clicked();
            if (clicked || enter) && named {
                match settings.add_profile(&window.profile_name, None) {
                    Ok(_) => {
                        window.profile_name.clear();
                        window.problem = None;
                    }
                    Err(error) => window.problem = Some(name_problem(error)),
                }
            }
        },
    );
}

/// A profile's card: its name, its link in words, who uses it, and what one asks of it.
fn profile_card(ui: &mut egui::Ui, window: &mut Window, settings: &Settings, profile: &NamedProfile) -> Option<CardAsked> {
    let dark = ui.visuals().dark_mode;
    let palette = Palette::of(dark);
    let mut asked = None;
    card(ui, |ui| {
        ui.horizontal(|ui| {
            let renaming = matches!(&window.renaming_profile, Some((old, _)) if *old == profile.name);
            if renaming {
                if let Some((_, new)) = &mut window.renaming_profile {
                    let field = ui.add(egui::TextEdit::singleline(new).desired_width(NAME_WIDTH));
                    widgets::name(&field, t!("Nouveau nom du profil", "New name of the profile"));
                    if field.lost_focus() && ui.input(|input| input.key_pressed(egui::Key::Enter)) {
                        asked = window.renaming_profile.clone().map(|(old, new)| CardAsked::Rename(old, new));
                    }
                }
                if theme::secondary_button(ui, t!("Renommer", "Rename"), true).clicked() {
                    asked = window.renaming_profile.clone().map(|(old, new)| CardAsked::Rename(old, new));
                }
                if theme::secondary_button(ui, t!("Annuler", "Cancel"), true).clicked() {
                    window.renaming_profile = None;
                }
            } else {
                ui.label(RichText::new(&profile.name).strong());
            }
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                let hint = t!("Supprimer ce profil : ceux qui l'utilisent prennent le profil général", "Delete this profile: those that use it take the general profile");
                if widgets::icon_button(ui, icons::TRASH, t!("Supprimer", "Delete"), hint, true).clicked() {
                    asked = Some(CardAsked::Delete(profile.name.clone()));
                }
                if widgets::icon_button(ui, icons::COPY, t!("Dupliquer", "Duplicate"), t!("Un nouveau profil, copie de celui-ci", "A new profile, a copy of this one"), true).clicked() {
                    asked = Some(CardAsked::Duplicate(profile.name.clone()));
                }
                if !renaming && widgets::icon_button(ui, icons::PENCIL, t!("Renommer", "Rename"), t!("Changer son nom", "Change its name"), true).clicked() {
                    window.renaming_profile = Some((profile.name.clone(), profile.name.clone()));
                }
            });
        });
        plain_row(ui, t!("Liaison", "Link"), t!("Comment le port est ouvert, et ce qui suit chaque ligne envoyée.", "How the port is opened, and what follows each line sent."), |ui| {
            words_then_button(ui, &link_words(&profile.values), Some(&profile.name), &mut asked);
        });
        let users = users_words(settings, &profile.name);
        plain_row(ui, t!("Utilisation", "Use"), t!("Les appareils nommés qui ont ce profil, et les onglets sur un port sans appareil nommé.", "The named devices given this profile, and the tabs on a port without a named device."), |ui| {
            ui.add(egui::Label::new(RichText::new(upper_first(&users)).color(palette.muted)).wrap());
        });
        if window.deleting.as_deref() == Some(profile.name.as_str()) {
            let name = &profile.name;
            let question = tr!("Supprimer « {name} » ? Il est {users} : ils prendront le profil général.", "Delete \"{name}\"? It is {users}: they will take the general profile.");
            ui.label(RichText::new(question).color(palette.tone(Tone::Bad, dark)));
            ui.horizontal(|ui| {
                if theme::primary_button(ui, t!("Supprimer", "Delete"), SHORT, true).clicked() {
                    asked = Some(CardAsked::ConfirmDelete(profile.name.clone()));
                }
                if theme::secondary_button(ui, t!("Annuler", "Cancel"), true).clicked() {
                    window.deleting = None;
                }
            });
        }
    });
    asked
}

/// A sentence's first letter in upper case.
fn upper_first(text: &str) -> String {
    let mut chars = text.chars();
    chars.next().map(|first| first.to_uppercase().chain(chars).collect()).unwrap_or_default()
}

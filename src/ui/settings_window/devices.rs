//! The section of the named devices in the settings: naming the device on the tab's port, and a
//! card for each named device, with the profile it is given.

use eframe::egui::{self, Align, Layout, RichText};

use super::{CONTROL_HEIGHT, KEEP_WIDTH, NAME_WIDTH, PROFILE_WIDTH, Palette, SHORT, Tone, Window, icons, name_problem, theme, widgets};
use super::rows::{card, plain_row, problem_line};
use crate::ports::PortInfo;
use crate::settings::{Device, Settings};

/// What a card of a device asks for, done once every card is drawn.
enum CardAsked {
    Forget(String),
    Rename(String, String),
    Give(String, Option<String>),
}

pub(super) fn devices(ui: &mut egui::Ui, window: &mut Window, settings: &mut Settings, active: usize, ports: &[PortInfo]) {
    name_device(ui, window, settings, active, ports);
    problem_line(ui, window.problem.as_deref());
    ui.add_space(8.0);
    if settings.devices.is_empty() {
        let muted = Palette::of(ui.visuals().dark_mode).muted;
        ui.label(RichText::new(t!("Aucun appareil nommé pour l'instant.", "No named device yet.")).color(muted));
        return;
    }
    let mut asked = None;
    let profiles: Vec<String> = settings.profiles.iter().map(|profile| profile.name.clone()).collect();
    for device in &settings.devices {
        if let Some(card_asked) = device_card(ui, window, device, &profiles, ports) {
            asked = Some(card_asked);
        }
        ui.add_space(6.0);
    }
    match asked {
        Some(CardAsked::Forget(name)) => settings.forget_device(&name),
        Some(CardAsked::Rename(old, new)) => match settings.rename_device(&old, &new) {
            Ok(()) => {
                window.renaming_device = None;
                window.problem = None;
            }
            Err(error) => window.problem = Some(name_problem(error)),
        },
        Some(CardAsked::Give(device, profile)) => settings.give_profile(&device, profile.as_deref()),
        None => {}
    }
}

/// The device of the tab: to be named, or already named.
fn name_device(ui: &mut egui::Ui, window: &mut Window, settings: &mut Settings, active: usize, ports: &[PortInfo]) {
    let muted = Palette::of(ui.visuals().dark_mode).muted;
    let tab = settings.tabs[active].clone();
    let device = ports.iter().find(|port| port.name == tab.port).and_then(|port| port.usb.clone());
    match (&tab.device, device) {
        (Some(name), _) => {
            ui.label(RichText::new(tr!("Cet onglet suit « {name} ».", "This tab follows \"{name}\".")).color(muted));
        }
        (None, Some(usb)) if settings.device_of(&usb).is_none() => {
            let hint = t!(
                "L'appareil USB branché sur le port de l'onglet. Nommé, RSC le reconnaît sur tout port, avec le profil que l'onglet utilise.",
                "The USB device on the tab's port. Named, RSC knows it on any port, with the profile the tab uses."
            );
            plain_row(ui, t!("Appareil de l'onglet", "Device of the tab"), hint, |ui| {
                let make = t!("Nommer cet appareil", "Name this device");
                // The name's field leaves the button the room its words take.
                let font = egui::TextStyle::Button.resolve(ui.style());
                let words = ui.ctx().fonts_mut(|fonts| fonts.layout_no_wrap(make.to_owned(), font, egui::Color32::PLACEHOLDER).size().x);
                let button = (words + 2.0 * ui.spacing().button_padding.x).max(KEEP_WIDTH);
                let field = (ui.available_width() - button - ui.spacing().item_spacing.x).clamp(SHORT, NAME_WIDTH);
                let response = ui.add(
                    egui::TextEdit::singleline(&mut window.device_name)
                        .hint_text(t!("Carte ESP32 n°3", "ESP32 board no. 3"))
                        .desired_width(field)
                        .min_size(egui::vec2(field, CONTROL_HEIGHT))
                        .vertical_align(Align::Center),
                );
                widgets::name(&response, t!("Nom de l'appareil", "Name of the device"));
                if std::mem::take(&mut window.focus_name) {
                    response.request_focus();
                }
                let enter = response.lost_focus() && ui.input(|input| input.key_pressed(egui::Key::Enter));
                let clicked = theme::primary_button(ui, make, KEEP_WIDTH, !window.device_name.trim().is_empty())
                    .on_hover_text(tr!("{} · {} : reconnu sur tout port, sous ce nom", "{} · {}: known on any port, under this name", tab.port, usb.label()))
                    .clicked();
                if clicked || enter {
                    match settings.name_device(active, &window.device_name, usb.clone()) {
                        Ok(()) => {
                            window.device_name.clear();
                            window.problem = None;
                        }
                        Err(error) => window.problem = Some(name_problem(error)),
                    }
                }
            });
        }
        (None, Some(_)) => {}
        (None, None) => {
            ui.label(RichText::new(t!("L'onglet n'est pas sur un port USB : rien à nommer.", "The tab is not on a USB port: nothing to name.")).color(muted));
        }
    }
}

/// A device's card: whether it is plugged in, its USB identity, its profile, and what one asks of it.
fn device_card(ui: &mut egui::Ui, window: &mut Window, device: &Device, profiles: &[String], ports: &[PortInfo]) -> Option<CardAsked> {
    let dark = ui.visuals().dark_mode;
    let palette = Palette::of(dark);
    let present = super::top::port_of(device, ports);
    let mut asked = None;
    card(ui, |ui| {
        ui.horizontal(|ui| {
            let tone = if present.is_some() { Tone::Good } else { Tone::Neutral };
            theme::mark(ui, tone, palette.tone(tone, dark));
            let renaming = matches!(&window.renaming_device, Some((old, _)) if *old == device.name);
            if renaming {
                if let Some((_, new)) = &mut window.renaming_device {
                    let field = ui.add(egui::TextEdit::singleline(new).desired_width(NAME_WIDTH));
                    widgets::name(&field, t!("Nouveau nom de l'appareil", "New name of the device"));
                    if field.lost_focus() && ui.input(|input| input.key_pressed(egui::Key::Enter)) {
                        asked = window.renaming_device.clone().map(|(old, new)| CardAsked::Rename(old, new));
                    }
                }
                if theme::secondary_button(ui, t!("Renommer", "Rename"), true).clicked() {
                    asked = window.renaming_device.clone().map(|(old, new)| CardAsked::Rename(old, new));
                }
                if theme::secondary_button(ui, t!("Annuler", "Cancel"), true).clicked() {
                    window.renaming_device = None;
                }
            } else {
                ui.label(RichText::new(&device.name).strong());
                let state = match &present {
                    Some(port) => tr!("branché sur {port}", "plugged in {port}"),
                    None if device.last_port.is_empty() => t!("absent", "absent").to_owned(),
                    None => tr!("absent · vu sur {}", "absent · last on {}", device.last_port),
                };
                ui.label(RichText::new(state).color(palette.muted));
            }
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                let hint = t!("Oublier cet appareil : ses onglets restent sur leur port, avec le profil général", "Forget this device: its tabs stay on their port, with the general profile");
                if widgets::icon_button(ui, icons::TRASH, t!("Oublier", "Forget"), hint, true).clicked() {
                    asked = Some(CardAsked::Forget(device.name.clone()));
                }
                if !renaming && widgets::icon_button(ui, icons::PENCIL, t!("Renommer", "Rename"), t!("Changer son nom", "Change its name"), true).clicked() {
                    window.renaming_device = Some((device.name.clone(), device.name.clone()));
                }
            });
        });
        plain_row(ui, "USB", t!("Ce qui reconnaît l'appareil : fabricant, produit, numéro de série.", "What tells the device: maker, product, serial number."), |ui| {
            ui.label(RichText::new(device.usb.label()).monospace());
        });
        let hint = t!("Le profil que RSC prend pour cet appareil, sur tout port. Général : les choix par défaut.", "The profile RSC takes for this device, on any port. General: the choices by default.");
        plain_row(ui, t!("Profil", "Profile"), hint, |ui| {
            let general = t!("Général", "General");
            let mut chosen = device.profile.clone();
            let list = egui::ComboBox::from_id_salt(("device-profile", &device.name))
                .width(PROFILE_WIDTH)
                .selected_text(chosen.clone().unwrap_or_else(|| general.to_owned()))
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut chosen, None, general);
                    for name in profiles {
                        ui.selectable_value(&mut chosen, Some(name.clone()), name);
                    }
                });
            widgets::labelled(list.response);
            if chosen != device.profile {
                asked = Some(CardAsked::Give(device.name.clone(), chosen));
            }
        });
    });
    asked
}

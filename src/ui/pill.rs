//! The profile's pill in the connection bar, and its menu: a profile of the library chosen, given to
//! the tab's named device or kept by the tab; a new profile from the one in use; the settings.

use eframe::egui::{self, Align, CornerRadius, Key, Layout, RichText};

use crate::ports::PortInfo;
use crate::settings::Settings;
use super::App;
use super::icons;
use super::settings_window;
use super::theme::{self, CONTROL_HEIGHT, Palette, Tone};
use super::widgets;

const PROFILE_PILL_WIDTH: f32 = 230.0;
/// A profile's name in the profile's pill, cut beyond.
const PILL_NAME_CHARS: usize = 16;
/// The width of the pill's menu: its sentences wrap within.
const PILL_MENU_WIDTH: f32 = 340.0;
/// The pill's width before it is drawn once.
pub(super) const PILL_GUESS: f32 = 220.0;

/// A name cut to `chars` characters at most, an ellipsis showing where.
pub(super) fn short_name(name: &str, chars: usize) -> String {
    if name.chars().count() <= chars { name.to_owned() } else { name.chars().take(chars.saturating_sub(1)).collect::<String>() + "…" }
}

/// The profile tab `tab` uses, as its tooltips name it.
pub(super) fn profile_label(settings: &Settings, tab: usize) -> String {
    match settings.profile_name_of(tab) {
        Some(name) => tr!("« {name} »", "\"{name}\""),
        None => t!("général", "general").to_owned(),
    }
}

/// What the pill's menu asks for.
enum PillAsked {
    /// A profile of the library, or the general one for none.
    Choose(Option<String>),
    /// A new profile under the name typed, a copy of the one in use, then chosen.
    Create,
    /// The settings, on the tab's profile.
    Settings,
    /// The settings, to name the device on the tab's port.
    NameDevice,
}

/// The profile's pill: its icon, then what it says.
fn pill_button<'a>(ui: &egui::Ui, text: &str) -> egui::Button<'a> {
    let palette = Palette::of(ui.visuals().dark_mode);
    egui::Button::new((icons::SLIDERS.text(15.0).color(palette.accent), RichText::new(text).color(palette.text)))
        .corner_radius(CornerRadius::same(8))
        .min_size(egui::vec2(PROFILE_PILL_WIDTH, CONTROL_HEIGHT))
}

/// What the profile's pill says: cut to fit the bar, whole for screen readers, and its tooltip.
pub(super) struct PillWords {
    shown: String,
    whole: String,
    hint: String,
}

/// Where the pill of tab `i` keeps the width it took, for the next frame to lay its row out by.
/// Laid out where nothing shows, it would stay in the accessibility tree: a second button of the
/// same name (WCAG 4.1.2).
pub(super) fn pill_width_id(i: usize) -> egui::Id {
    egui::Id::new(("pill-width", i))
}

/// A sentence of a menu, aligned to the left: a menu's justified layout would stretch its spaces.
fn menu_note(ui: &mut egui::Ui, text: RichText) {
    ui.with_layout(Layout::top_down(Align::Min), |ui| ui.label(text));
}

/// An item of a menu: its icon, then its text, which names it.
fn menu_item(ui: &mut egui::Ui, icon: icons::Icon, text: &str, enabled: bool) -> egui::Response {
    let muted = Palette::of(ui.visuals().dark_mode).muted;
    let response = ui.add_enabled(enabled, egui::Button::new((icon.text(14.0).color(muted), "  ", text)));
    response.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, enabled, text));
    response
}

impl App {
    /// What the pill says, whole and cut to fit, and its tooltip: the profile the tab uses, and who
    /// it is given to.
    pub(super) fn pill_words(&self, i: usize) -> PillWords {
        let device = self.settings.tabs[i].device.as_deref().filter(|name| self.settings.find_device(name).is_some());
        match (self.settings.profile_name_of(i), device) {
            (None, device) => {
                let text = t!("Profil général", "General profile").to_owned();
                let hint = match device {
                    Some(device) => tr!(
                        "« {device} » utilise le profil général, les choix par défaut. Cliquer pour lui choisir un profil.",
                        "\"{device}\" uses the general profile, the choices by default. Click to choose it a profile."
                    ),
                    None => t!(
                        "Ce port utilise le profil général, les choix par défaut. Cliquer pour choisir un profil.",
                        "This port uses the general profile, the choices by default. Click to choose a profile."
                    )
                    .to_owned(),
                };
                PillWords { shown: text.clone(), whole: text, hint }
            }
            (Some(name), device) => {
                let says = |name: &str| tr!("Profil {name}", "Profile {name}");
                let hint = match device {
                    Some(device) => tr!(
                        "« {device} » utilise le profil « {name} », sur tout port. Cliquer pour en choisir un autre ou le modifier.",
                        "\"{device}\" uses the profile \"{name}\", on any port. Click to choose another or change it."
                    ),
                    None => tr!(
                        "Cet onglet utilise le profil « {name} ». Cliquer pour en choisir un autre ou le modifier.",
                        "This tab uses the profile \"{name}\". Click to choose another or change it."
                    ),
                };
                PillWords { shown: says(&short_name(name, PILL_NAME_CHARS)), whole: says(name), hint }
            }
        }
    }

    /// Which profile the tab uses, at a glance, as `pill_words` says it: cut to fit the bar, whole for
    /// screen readers (WCAG 4.1.2). A click opens its menu: a profile of the library, a new one, the
    /// settings.
    pub(super) fn profile_pill(&mut self, ui: &mut egui::Ui, i: usize, words: PillWords) {
        let palette = Palette::of(ui.visuals().dark_mode);
        let response = theme::filled_button(ui, pill_button(ui, &words.shown), palette.field, true);
        response.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true, &words.whole));
        widgets::measure(ui, pill_width_id(i), response.rect.width());
        let response = response.on_hover_text(words.hint);
        if response.clicked() {
            self.pill_problem = None;
        }
        // Clicks inside leave it open, for the name to be typed: it closes once done.
        let menu = egui::Popup::menu(&response).close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside).width(PILL_MENU_WIDTH).show(|ui| {
            ui.set_max_width(PILL_MENU_WIDTH);
            let asked = self.pill_menu(ui, i);
            if asked.is_some_and(|asked| self.do_asked(i, asked)) {
                ui.close();
            }
        });
        if let Some(menu) = menu {
            widgets::name(&menu.response, t!("Menu du profil", "Menu of the profile"));
        }
    }

    /// The pill's menu: who the choice goes to, the profiles of the library, a new profile from the one
    /// in use, the settings; on a USB port without a name, the way to name its device.
    fn pill_menu(&mut self, ui: &mut egui::Ui, i: usize) -> Option<PillAsked> {
        let dark = ui.visuals().dark_mode;
        let palette = Palette::of(dark);
        let mut asked = None;
        let device = self.settings.tabs[i].device.clone().filter(|name| self.settings.find_device(name).is_some());
        let unnamed = self.unnamed_device(i);
        let goes_to = match (&device, unnamed) {
            (Some(device), _) => tr!("Le profil choisi va à « {device} », sur tout port.", "The profile chosen goes to \"{device}\", on any port."),
            (None, true) => t!(
                "Le profil choisi reste à cet onglet. Nommez l'appareil pour qu'il le garde sur tout port.",
                "The profile chosen stays with this tab. Name the device for it to keep it on any port."
            )
            .to_owned(),
            (None, false) => t!("Le profil choisi reste à cet onglet.", "The profile chosen stays with this tab.").to_owned(),
        };
        menu_note(ui, RichText::new(goes_to).color(palette.muted));
        ui.separator();
        // The profiles, the one in use marked.
        let current = self.settings.profile_name_of(i).map(str::to_owned);
        if ui.selectable_label(current.is_none(), t!("Général", "General")).on_hover_text(t!("Les choix par défaut", "The choices by default")).clicked() {
            asked = Some(PillAsked::Choose(None));
        }
        for profile in &self.settings.profiles {
            let chosen = current.as_deref() == Some(profile.name.as_str());
            if ui.selectable_label(chosen, &profile.name).on_hover_text(settings_window::link_words(&profile.values)).clicked() {
                asked = Some(PillAsked::Choose(Some(profile.name.clone())));
            }
        }
        ui.separator();
        // A new profile: a copy of the one in use, under the name typed.
        menu_note(ui, RichText::new(t!("Nouveau profil, à partir de celui-ci", "New profile, from this one")).small().color(palette.muted));
        let field = ui.add(
            egui::TextEdit::singleline(&mut self.pill_name)
                .hint_text(t!("ESP32 921600", "ESP32 921600"))
                .desired_width(f32::INFINITY)
                .min_size(egui::vec2(0.0, CONTROL_HEIGHT))
                .vertical_align(Align::Center),
        );
        widgets::name(&field, t!("Nom du nouveau profil", "Name of the new profile"));
        let named = !self.pill_name.trim().is_empty();
        let enter = field.lost_focus() && ui.input(|input| input.key_pressed(Key::Enter));
        let create = menu_item(ui, icons::PLUS, t!("Créer et choisir ce profil", "Make and choose this profile"), named)
            .on_hover_text(t!("Une copie du profil utilisé, sous ce nom, aussitôt choisie", "A copy of the profile in use, under this name, chosen at once"))
            .on_disabled_hover_text(t!("Il faut d'abord un nom.", "A name is needed first."));
        if (create.clicked() || enter) && named {
            asked = Some(PillAsked::Create);
        }
        if let Some(problem) = &self.pill_problem {
            menu_note(ui, RichText::new(problem).color(palette.tone(Tone::Bad, dark)));
        }
        ui.separator();
        if menu_item(ui, icons::SETTINGS, t!("Modifier dans les réglages…", "Change in the settings…"), true).clicked() {
            asked = Some(PillAsked::Settings);
        }
        if unnamed && menu_item(ui, icons::TAG, t!("Nommer cet appareil…", "Name this device…"), true).clicked() {
            asked = Some(PillAsked::NameDevice);
        }
        asked
    }

    /// Whether the tab's port carries a USB device that has no name yet.
    pub(super) fn unnamed_device(&self, i: usize) -> bool {
        let tab = &self.settings.tabs[i];
        tab.device.is_none() && self.ports.iter().any(|known| known.name == tab.port && known.usb.as_ref().is_some_and(|usb| self.settings.device_of(usb).is_none()))
    }

    /// The product on the tab's port, as the system names it: the name a device is first offered.
    pub(super) fn product_on(&self, i: usize) -> String {
        let port = &self.settings.tabs[i].port;
        self.ports.iter().find(|known: &&PortInfo| known.name == *port).map(|known| known.description.clone()).unwrap_or_default()
    }

    /// Does what the pill's menu asked for: false when it could not, and says why in the menu.
    fn do_asked(&mut self, i: usize, asked: PillAsked) -> bool {
        match asked {
            PillAsked::Choose(name) => self.settings.choose_profile(i, name.as_deref()),
            PillAsked::Create => {
                let from = self.settings.profile_name_of(i).map(str::to_owned);
                match self.settings.add_profile(&self.pill_name, from.as_deref()) {
                    Ok(name) => {
                        self.settings.choose_profile(i, Some(&name));
                        self.pill_name.clear();
                        self.pill_problem = None;
                    }
                    Err(error) => {
                        self.pill_problem = Some(settings_window::name_problem(error));
                        return false;
                    }
                }
            }
            PillAsked::Settings => self.window = Some(settings_window::Window::for_tab(&self.settings, i)),
            PillAsked::NameDevice => self.window = Some(settings_window::Window::for_device(self.product_on(i))),
        }
        true
    }
}

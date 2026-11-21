//! The settings window. At its top, always, the profile being changed: the general one, the choices
//! by default, or a profile of the library, and who uses it. At its left, the sections: those of the
//! profile, then the library of profiles, the named devices and the look, which are the
//! application's. A change takes effect at once and is kept; each label explains itself.

mod application;
mod devices;
mod profile;
mod profiles;
mod rows;

use std::path::Path;

use eframe::egui::{self, Align, Align2, CornerRadius, Layout, Margin, RichText};

use super::files;
use super::icons::{self, Icon};
use super::theme::{self, CONTROL_HEIGHT, Palette, Tone};
use super::top;
use super::widgets;
use crate::ports::PortInfo;
use crate::settings::{NameError, Profile, Settings};
use application::{about, look};
use devices::devices;
use profile::edit_profile;
use profiles::profiles;
pub(super) use profiles::users_words;

/// The width of the navigation, and of the labels' column, the same in every section.
const NAV_WIDTH: f32 = 196.0;
const LABEL_WIDTH: f32 = 196.0;
/// The width of a number or a short choice, and of a name.
const SHORT: f32 = 128.0;
const NAME_WIDTH: f32 = 200.0;
/// The width of the button that keeps a name, and of the profile's selector.
const KEEP_WIDTH: f32 = 200.0;
const PROFILE_WIDTH: f32 = 240.0;
/// The room the "Change" button takes after the words of a profile's card.
const CHANGE_PROFILE_WIDTH: f32 = 190.0;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Section {
    #[default]
    Link,
    Console,
    Curves,
    Sending,
    Connection,
    Journal,
    Profiles,
    Devices,
    Look,
    About,
}

impl Section {
    /// The sections of the profile being changed.
    const PROFILE: [Section; 6] = [Section::Link, Section::Console, Section::Curves, Section::Sending, Section::Connection, Section::Journal];
    /// The sections of the application, the same whatever the profile.
    const APPLICATION: [Section; 4] = [Section::Profiles, Section::Devices, Section::Look, Section::About];
    #[cfg(test)]
    pub const ALL: [Section; 10] = [
        Section::Link,
        Section::Console,
        Section::Curves,
        Section::Sending,
        Section::Connection,
        Section::Journal,
        Section::Profiles,
        Section::Devices,
        Section::Look,
        Section::About,
    ];

    fn icon(self) -> Icon {
        match self {
            Section::Link => icons::PLUG,
            Section::Console => icons::TERMINAL,
            Section::Curves => icons::CHART_LINE,
            Section::Sending => icons::SEND,
            Section::Connection => icons::REFRESH,
            Section::Journal => icons::FILE_TEXT,
            Section::Profiles => icons::SLIDERS,
            Section::Devices => icons::CPU,
            Section::Look => icons::PALETTE,
            Section::About => icons::INFO,
        }
    }

    pub fn title(self) -> &'static str {
        match self {
            Section::Link => t!("Liaison", "Link"),
            Section::Console => t!("Console", "Console"),
            Section::Curves => t!("Courbes", "Curves"),
            Section::Sending => t!("Envoi", "Sending"),
            Section::Connection => t!("Connexion", "Connection"),
            Section::Journal => t!("Journal", "Journal"),
            Section::Profiles => t!("Profils", "Profiles"),
            Section::Devices => t!("Appareils", "Devices"),
            Section::Look => t!("Apparence", "Look"),
            Section::About => t!("À propos", "About"),
        }
    }

    fn hint(self) -> &'static str {
        match self {
            Section::Link => t!("Comment le port est ouvert. Une liaison ouverte avec ce profil est rouverte aussitôt.", "How the port is opened. A link open with this profile is opened again at once."),
            Section::Console => t!("Ce que la console montre et garde.", "What the console shows and keeps."),
            Section::Curves => t!(
                "Une ligne faite seulement de valeurs devient des points : « 21.5 40 » ou « temp=21.5 hum=40 ».",
                "A line made only of values becomes points: \"21.5 40\" or \"temp=21.5 hum=40\"."
            ),
            Section::Sending => t!("Comment la ligne d'envoi est lue.", "How the send line is read."),
            Section::Connection => t!("Ce que fait un onglet quand son port disparaît.", "What a tab does when its port goes away."),
            Section::Journal => t!(
                "Tout ce qui arrive, écrit dans un fichier au fil de l'eau, au-delà des lignes gardées.",
                "Everything that comes, written to a file as it goes, beyond the lines kept."
            ),
            Section::Profiles => t!(
                "La bibliothèque : des profils nommés, chacun avec toutes ses valeurs, à donner à un appareil ou à un onglet.",
                "The library: named profiles, each with all its values, to give to a device or a tab."
            ),
            Section::Devices => t!(
                "Un appareil USB que vous nommez : RSC le reconnaît sur n'importe quel port et lui donne son profil.",
                "A USB device you name: RSC knows it on whatever port and gives it its profile."
            ),
            Section::Look => t!("Commune à tous les profils : le thème et la langue de la fenêtre.", "Common to every profile: the window's theme and language."),
            Section::About => t!("RSC, sa version, sa licence, et celles des composants dont il est fait.", "RSC, its version, its licence, and those of the components it is made of."),
        }
    }
}

#[derive(Default)]
pub struct Window {
    pub(super) section: Section,
    /// The profile being changed: a profile of the library, by its name, or the general one.
    pub(super) profile: Option<String>,
    /// The name typed for a new profile of the library, and for the device of the tab.
    pub(super) profile_name: String,
    pub(super) device_name: String,
    /// The profile and the device being renamed, each with its new name typed.
    pub(super) renaming_profile: Option<(String, String)>,
    pub(super) renaming_device: Option<(String, String)>,
    /// The profile whose deletion waits to be confirmed: it is in use.
    pub(super) deleting: Option<String>,
    /// The name field of the device takes the focus at the next frame.
    focus_name: bool,
    /// Why the last name was refused.
    problem: Option<String>,
    /// "About": the components searched for, the one whose licence is shown, and where the
    /// notices were saved or why they were not (then true).
    pub(super) about_search: String,
    about_chosen: Option<String>,
    about_saved: Option<(String, bool)>,
    /// A component was just chosen: the section scrolls down to its licence.
    about_reveal: bool,
    /// The section whose row the list of sections brought into view: a section chosen since, or a
    /// window just opened, is brought once, then the list scrolls freely.
    section_shown: Option<Section>,
}

impl Window {
    /// Opened on the profile of the tab in front.
    pub fn for_tab(settings: &Settings, tab: usize) -> Self {
        Self { profile: settings.profile_name_of(tab).map(str::to_owned), ..Self::default() }
    }

    /// Opened on a section, on a profile.
    #[cfg(test)]
    pub fn at(section: Section, profile: Option<&str>) -> Self {
        Self { section, profile: profile.map(str::to_owned), ..Self::default() }
    }

    /// Opened to name the device of the tab, the name `name` offered.
    pub fn for_device(name: String) -> Self {
        Self { section: Section::Devices, device_name: name, focus_name: true, ..Self::default() }
    }
}

/// The profile being changed in a frame: its values, drawn on a copy.
struct Editor {
    profile: Profile,
}

/// What a profile's link is, in a few words: "115200 8N1 · LF".
pub(super) fn link_words(profile: &Profile) -> String {
    format!("{} · {}", profile.link().summary(), profile.describe(crate::settings::Field::LineEnding))
}

pub(super) fn name_problem(error: NameError) -> String {
    match error {
        NameError::Empty => t!("Il faut un nom.", "A name is needed.").to_owned(),
        NameError::Taken => t!("Ce nom est déjà pris.", "This name is already taken.").to_owned(),
        NameError::Reserved => t!("« Général » est le nom du profil par défaut.", "\"General\" is the name of the default profile.").to_owned(),
        NameError::Known(name) => tr!("Cet appareil s'appelle déjà « {name} ».", "This device is already called \"{name}\"."),
        NameError::Unknown => t!("Introuvable.", "Not found.").to_owned(),
    }
}

/// Shows the window over the tab `active`, with the ports of the machine and the file the settings
/// are kept in: false once it is closed.
pub fn show(ctx: &egui::Context, window: &mut Window, settings: &mut Settings, active: usize, ports: &[PortInfo], path: Option<&Path>) -> bool {
    let mut close = false;
    if window.profile.as_deref().is_some_and(|name| settings.find_profile(name).is_none()) {
        window.profile = None;
    }
    let screen = ctx.content_rect();
    // A margin above and below; small on a low window, the text at 200 % for instance: the
    // settings are modal, the window behind has nothing to show meanwhile.
    let margin = if screen.height() < 600.0 { 24.0 } else { 120.0 };
    let height = (screen.height() - margin).clamp(320.0, 720.0);
    // The window behind does not answer the pointer while the settings are open: a backdrop takes
    // its clicks, as egui's modals do, but clear, without a veil; it is nothing to screen readers.
    egui::Area::new(egui::Id::new("settings-backdrop")).order(egui::Order::Background).fixed_pos(screen.min).show(ctx, |ui| {
        widgets::hidden(&ui.allocate_response(screen.size(), egui::Sense::CLICK | egui::Sense::DRAG));
    });
    let shown = egui::Window::new(t!("Réglages", "Settings"))
        .id(egui::Id::new("settings"))
        .title_bar(false)
        .collapsible(false)
        .resizable([true, true])
        .default_size(egui::vec2(880.0, height))
        .min_size(egui::vec2(720.0, 320.0))
        .pivot(Align2::CENTER_CENTER)
        .default_pos(screen.center())
        .frame(egui::Frame::window(&ctx.global_style()).inner_margin(Margin::ZERO))
        .show(ctx, |ui| {
            let palette = Palette::of(ui.visuals().dark_mode);
            // No stroke of their own: a rectangle's square corners would cross the window's rounded
            // ones. The panels draw their separating line themselves.
            egui::Panel::top("settings-header").frame(egui::Frame::new().inner_margin(Margin { left: 18, right: 16, top: 12, bottom: 10 })).show(ui, |ui| header(ui, window, settings, &mut close));
            // The footer's room is taken first, for the content to end above it; the footer is
            // filled last, for Tab to reach "Close" after the content (WCAG 2.4.3).
            let footer = egui::Panel::bottom("settings-footer").exact_size(CONTROL_HEIGHT + 20.0).frame(egui::Frame::new()).show(ui, |_| ()).response.rect;
            egui::Panel::left("settings-nav")
                .exact_size(NAV_WIDTH)
                .resizable(false)
                .frame(egui::Frame::new().fill(palette.page).inner_margin(Margin::symmetric(10, 12)))
                // Scrolled when the window is low (WCAG 1.4.4): no section falls under the footer.
                .show(ui, |ui| egui::ScrollArea::vertical().id_salt("settings-nav").auto_shrink([false, false]).show(ui, |ui| navigation(ui, window)));
            egui::Frame::new().inner_margin(Margin::symmetric(20, 14)).show(ui, |ui| {
                egui::ScrollArea::vertical().id_salt(window.section).auto_shrink([false, false]).show(ui, |ui| {
                    ui.label(RichText::new(window.section.title()).strong().size(17.0));
                    ui.label(RichText::new(window.section.hint()).color(palette.muted));
                    ui.add_space(10.0);
                    match window.section {
                        Section::Profiles => profiles(ui, window, settings),
                        Section::Devices => devices(ui, window, settings, active, ports),
                        Section::Look => look(ui, settings),
                        Section::About => about(ui, window),
                        section => edit_profile(ui, window, settings, section),
                    }
                });
            });
            let inside = footer.shrink2(egui::vec2(18.0, 10.0));
            ui.scope_builder(egui::UiBuilder::new().max_rect(inside).layout(Layout::left_to_right(Align::Center)), |ui| {
                ui.label(RichText::new(format!("RSC {}", env!("CARGO_PKG_VERSION"))).weak());
                if let Some(path) = path {
                    let shown = path.display().to_string();
                    ui.add(egui::Label::new(RichText::new(tr!("· gardés dans {shown}", "· kept in {shown}")).weak()).truncate())
                        .on_hover_text(tr!("Le fichier des réglages, écrit à chaque changement :\n{shown}", "The settings file, written at each change:\n{shown}"));
                }
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if theme::primary_button(ui, t!("Fermer", "Close"), SHORT, true).clicked() {
                        close = true;
                    }
                });
            });
        });
    // The settings keep the keyboard and the pointer while they are open (WCAG 2.4.11): Tab never
    // goes under them. A focus left in the window behind them (its layer is the background) is
    // given up, for the next Tab to come into them.
    if let Some(shown) = &shown {
        ctx.memory_mut(|memory| memory.set_modal_layer(shown.response.layer_id));
        if let Some(behind) = ctx.memory(|memory| memory.focused()).and_then(|id| ctx.read_response(id)).filter(|focused| focused.layer_id.order == egui::Order::Background) {
            ctx.memory_mut(|memory| memory.surrender_focus(behind.id));
        }
    }
    !close
}

/// The title, the profile being changed and what it means, the cross that closes.
fn header(ui: &mut egui::Ui, window: &mut Window, settings: &Settings, close: &mut bool) {
    let palette = Palette::of(ui.visuals().dark_mode);
    ui.horizontal(|ui| {
        ui.label(RichText::new(t!("Réglages", "Settings")).strong().size(17.0));
        ui.add_space(20.0);
        widgets::hidden(&ui.label(icons::SLIDERS.text(15.0).color(palette.muted)));
        let changing = ui.label(t!("Vous modifiez", "You are changing"));
        let general = t!("Général", "General");
        let selected = window.profile.clone().unwrap_or_else(|| general.to_owned());
        egui::ComboBox::from_id_salt("settings-profile")
            .width(PROFILE_WIDTH)
            .selected_text(selected)
            .show_ui(ui, |ui| {
                if ui.selectable_label(window.profile.is_none(), general).on_hover_text(t!("Les choix par défaut", "The choices by default")).clicked() {
                    window.profile = None;
                }
                for profile in &settings.profiles {
                    let hint = link_words(&profile.values);
                    if ui.selectable_label(window.profile.as_deref() == Some(profile.name.as_str()), &profile.name).on_hover_text(hint).clicked() {
                        window.profile = Some(profile.name.clone());
                    }
                }
            })
            .response
            .labelled_by(changing.id)
            .on_hover_text(t!("Le profil que les sections Liaison à Journal modifient", "The profile the sections Link to Journal change"));
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            if widgets::bare_icon_button(ui, icons::X, t!("Fermer les réglages", "Close the settings"), t!("Fermer les réglages ; tout est déjà gardé", "Close the settings; everything is already kept")).clicked() {
                *close = true;
            }
        });
    });
    let meaning = match window.profile.as_deref().filter(|name| settings.find_profile(name).is_some()) {
        Some(name) => {
            let users = users_words(settings, name);
            tr!("« {name} », {users}. Un réglage changé ici change ce profil seulement.", "\"{name}\", {users}. A setting changed here changes this profile only.")
        }
        None => t!(
            "Le profil général : les choix par défaut, pour les ports et les appareils qui n'ont pas de profil de la bibliothèque.",
            "The general profile: the choices by default, for the ports and the devices that have no profile of the library."
        )
        .to_owned(),
    };
    ui.label(RichText::new(meaning).color(palette.muted));
}

fn navigation(ui: &mut egui::Ui, window: &mut Window) {
    let palette = Palette::of(ui.visuals().dark_mode);
    let muted = palette.muted;
    ui.with_layout(Layout::top_down_justified(Align::LEFT), |ui| {
        ui.spacing_mut().button_padding = egui::vec2(10.0, 7.0);
        // Ten sections whole at the window's size by default: 4 points between two, not 8.
        ui.spacing_mut().item_spacing.y = 4.0;
        for (group, sections) in [(t!("Profil", "Profile"), &Section::PROFILE[..]), (t!("Application", "Application"), &Section::APPLICATION[..])] {
            ui.add_space(4.0);
            ui.label(RichText::new(group).small().color(muted));
            for &section in sections {
                let chosen = window.section == section;
                let response = ui.selectable_label(chosen, (section.icon().text(15.0), "  ", section.title()));
                let title = section.title();
                response.widget_info(|| egui::WidgetInfo::selected(egui::WidgetType::SelectableLabel, true, chosen, title));
                if chosen {
                    // Marked by more than its fill (WCAG 1.4.11): a bar of the accent at its left.
                    let bar = egui::Rect::from_min_size(response.rect.left_top() + egui::vec2(0.0, 6.0), egui::vec2(3.0, response.rect.height() - 12.0));
                    ui.painter().rect_filled(bar, CornerRadius::same(2), palette.accent);
                    // In view at any height (WCAG 1.4.10), once: the window opened on it, or it was
                    // just chosen.
                    if window.section_shown != Some(section) {
                        response.scroll_to_me(None);
                        window.section_shown = Some(section);
                    }
                }
                if response.clicked() && !chosen {
                    window.section = section;
                    window.renaming_profile = None;
                    window.renaming_device = None;
                    window.deleting = None;
                    window.problem = None;
                }
            }
            ui.add_space(8.0);
        }
    });
}

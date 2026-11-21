//! The application's sections of the settings: "Look" and "About", with the licences.

use eframe::egui::{self, Align, RichText};

use super::{CONTROL_HEIGHT, NAME_WIDTH, PROFILE_WIDTH, Palette, Tone, Window, theme, widgets};
use super::rows::plain_row;
use crate::licences;
use crate::settings::{Language, Settings, Theme};

/// RSC, its version, and the licences of what it is made of: each component, the texts of its
/// licence at a click, and the whole record written to a file. The record is read the first time
/// this section shows.
pub(super) fn about(ui: &mut egui::Ui, window: &mut Window) {
    let dark = ui.visuals().dark_mode;
    let palette = Palette::of(dark);
    let notices = licences::bundled();
    let count = notices.components.len();
    ui.label(RichText::new("RSC · Rust Serial Console").strong());
    ui.label(RichText::new(tr!("Version {}", "Version {}", env!("CARGO_PKG_VERSION"))).color(palette.muted));
    // RSC's own licence, its text carried as the components' are.
    ui.label(format!("© {} {} · Eclipse Public License 2.0 (EPL-2.0)", licences::YEAR, licences::HOLDER));
    // Where its source is, said as each component's is.
    ui.label(RichText::new(tr!("Source : {}", "Source: {}", licences::SOURCE)).color(palette.muted));
    egui::CollapsingHeader::new(t!("Lire la licence de RSC", "Read RSC's licence")).id_salt("about-own-licence").show(ui, |ui| {
        egui::ScrollArea::both().id_salt("about-own-text").max_height(320.0).auto_shrink([false, true]).show(ui, |ui| {
            ui.add(egui::Label::new(RichText::new(licences::own_licence()).monospace().size(12.0)).extend());
        });
    });
    ui.add_space(8.0);
    ui.label(tr!(
        "RSC est fait de {count} composants libres. Leurs licences demandent que leurs mentions accompagnent chaque copie : les voici, avec leurs textes.",
        "RSC is made of {count} open components. Their licences ask that their notices go with every copy: here they are, with their texts."
    ));
    ui.add_space(6.0);
    ui.horizontal(|ui| {
        let save = theme::secondary_button(ui, t!("Enregistrer les mentions…", "Save the notices…"), true)
            .on_hover_text(t!(
                "La licence de RSC et toutes les mentions de ses composants, dans un fichier texte à joindre à une copie de RSC",
                "RSC's licence and every notice of its components, in a text file to go with a copy of RSC"
            ));
        if save.clicked() {
            window.about_saved = save_notices().or(window.about_saved.take());
        }
        if let Some((said, failed)) = &window.about_saved {
            ui.label(RichText::new(said).color(if *failed { palette.tone(Tone::Bad, dark) } else { palette.muted }));
        }
    });
    ui.add_space(12.0);
    // A label that stays when a name is typed (WCAG 3.3.2), the first word of the field's name
    // (2.5.3).
    ui.horizontal(|ui| {
        ui.label(t!("Chercher", "Find"));
        let search = ui.add(
            egui::TextEdit::singleline(&mut window.about_search)
                .hint_text(t!("Un composant ou une licence", "A component or a licence"))
                .desired_width(PROFILE_WIDTH + NAME_WIDTH)
                .min_size(egui::vec2(0.0, CONTROL_HEIGHT))
                .vertical_align(Align::Center),
        );
        widgets::name(&search, t!("Chercher un composant ou une licence", "Find a component or a licence"));
    });
    let wanted = window.about_search.trim().to_lowercase();
    let shown: Vec<&licences::Component> =
        notices.components.iter().filter(|component| wanted.is_empty() || component.name.to_lowercase().contains(&wanted) || component.licence.to_lowercase().contains(&wanted)).collect();
    ui.label(RichText::new(tr!("{} sur {count}", "{} of {count}", shown.len())).small().color(palette.muted));
    egui::ScrollArea::vertical().id_salt("about-components").max_height(220.0).auto_shrink([false, true]).show(ui, |ui| {
        for component in shown {
            let label = component.label();
            let chosen = window.about_chosen.as_deref() == Some(label.as_str());
            if ui.selectable_label(chosen, format!("{label} · {}", component.licence)).clicked() {
                window.about_reveal = !chosen;
                window.about_chosen = (!chosen).then_some(label);
            }
        }
    });
    let Some(component) = window.about_chosen.as_deref().and_then(|label| notices.components.iter().find(|component| component.label() == label)) else {
        return;
    };
    ui.add_space(12.0);
    let title = ui.label(RichText::new(format!("{} · {}", component.label(), component.licence)).strong());
    if std::mem::take(&mut window.about_reveal) {
        title.scroll_to_me(Some(Align::TOP));
    }
    // Plain text, not links: RSC opens nothing.
    ui.label(tr!("Source : {}", "Source: {}", component.source));
    if let Some(repository) = &component.repository {
        ui.label(tr!("Dépôt : {repository}", "Repository: {repository}"));
    }
    ui.label(RichText::new(tr!("Dans le programme pour : {}", "In the program for: {}", component.systems.join(", "))).color(palette.muted));
    ui.add_space(6.0);
    // Each line whole, as the licence lays it out: the texts scroll sideways too.
    egui::ScrollArea::both().id_salt("about-texts").max_height(320.0).auto_shrink([false, true]).show(ui, |ui| {
        for text in notices.texts_of(component) {
            ui.label(RichText::new(&text.title).small().color(palette.muted));
            ui.add(egui::Label::new(RichText::new(&text.body).monospace().size(12.0)).extend());
            ui.add_space(8.0);
        }
    });
}

/// RSC's licence and the whole record of its components' written where one asks: where it went, or
/// why it did not (then true).
fn save_notices() -> Option<(String, bool)> {
    let path = super::files::save_dialog(t!("Enregistrer les mentions de licences", "Save the licence notices"), "RSC-LICENSES.txt", super::files::Saved::Notices)?;
    Some(match std::fs::write(&path, licences::everything()) {
        Ok(()) => (tr!("Écrites dans {}", "Written to {}", path.display()), false),
        Err(error) => (tr!("Pas écrites : {error}", "Not written: {error}"), true),
    })
}

pub(super) fn look(ui: &mut egui::Ui, settings: &mut Settings) {
    plain_row(ui, t!("Thème", "Theme"), t!("Celui du système suit ses changements ; la console reste sombre.", "The system's follows its changes; the console stays dark."), |ui| {
        widgets::segmented(
            ui,
            &mut settings.theme,
            &[(Theme::System, t!("Système", "System")), (Theme::Light, t!("Clair", "Light")), (Theme::Dark, t!("Sombre", "Dark"))],
            t!("Le thème de la fenêtre", "The window's theme"),
        );
    });
    plain_row(ui, t!("Langue", "Language"), t!("La langue de la fenêtre.", "The window's language."), |ui| {
        widgets::segmented(ui, &mut settings.language, &[(Language::French, "Français"), (Language::English, "English")], t!("Français ou anglais", "French or English"));
    });
}

//! The sections of a profile in the settings: the link, the console, the curves, the sending, the
//! connection and the journal, drawn on a copy of the profile.

use eframe::egui::{self, Align, RichText};

use super::{CONTROL_HEIGHT, Editor, Palette, SHORT, Section, Window, theme, widgets};
use super::rows::row;
use crate::plot::XAxis;
use crate::settings::{BAUD_RATES, FONT_SIZE, Field, FlowControl, LineEnding, PLOT_POINTS, Parity, SCROLLBACK, Settings, StopBits};

/// A section of the profile: drawn on a copy, what changed kept in the profile at the end.
pub(super) fn edit_profile(ui: &mut egui::Ui, window: &Window, settings: &mut Settings, section: Section) {
    let which = window.profile.clone();
    let before = settings.profile(which.as_deref());
    let mut editor = Editor { profile: before.clone() };
    match section {
        Section::Link => link(ui, &mut editor),
        Section::Console => console(ui, &mut editor),
        Section::Curves => curves(ui, &mut editor),
        Section::Sending => sending(ui, &mut editor),
        Section::Connection => connection(ui, &mut editor),
        _ => journal(ui, &mut editor),
    }
    if editor.profile != before {
        settings.set_profile(which.as_deref(), editor.profile);
    }
}

fn link(ui: &mut egui::Ui, editor: &mut Editor) {
    row(ui, editor, t!("Débit (bauds)", "Baud rate"), t!("Les bits par seconde : le même des deux côtés.", "The bits per second: the same at both ends."), |ui, profile| {
        let list = egui::ComboBox::from_id_salt("settings-baud").width(SHORT).selected_text(profile.baud_rate.to_string()).show_ui(ui, |ui| {
            for value in BAUD_RATES {
                ui.selectable_value(&mut profile.baud_rate, value, value.to_string());
            }
        });
        widgets::labelled(list.response);
        // Typed, not dragged: each value dragged through would open the port again.
        widgets::labelled(ui.add_sized(egui::vec2(SHORT, CONTROL_HEIGHT), egui::DragValue::new(&mut profile.baud_rate).range(50..=12_000_000).speed(0.0)))
            .on_hover_text(t!("Une autre valeur : cliquer, taper, Entrée", "Another value: click, type, Enter"));
    });
    row(ui, editor, t!("Bits de données", "Data bits"), t!("8 presque toujours ; 7 pour de vieux appareils.", "8 nearly always; 7 for old devices."), |ui, profile| {
        widgets::segmented(ui, &mut profile.data_bits, &[(5, "5"), (6, "6"), (7, "7"), (8, "8")], t!("Les bits de chaque caractère", "The bits of each character"));
    });
    row(ui, editor, t!("Parité", "Parity"), t!("Un bit de contrôle après les données ; aucun le plus souvent.", "A check bit after the data; none most of the time."), |ui, profile| {
        widgets::segmented(
            ui,
            &mut profile.parity,
            &[(Parity::None, t!("Aucune", "None")), (Parity::Even, t!("Paire", "Even")), (Parity::Odd, t!("Impaire", "Odd"))],
            t!("N, E ou O dans « 8N1 »", "N, E or O in \"8N1\""),
        );
    });
    row(ui, editor, t!("Bits de stop", "Stop bits"), t!("La fin de chaque caractère ; 1 le plus souvent.", "The end of each character; 1 most of the time."), |ui, profile| {
        widgets::segmented(ui, &mut profile.stop_bits, &[(StopBits::One, "1"), (StopBits::Two, "2")], t!("Le 1 de « 8N1 »", "The 1 of \"8N1\""));
    });
    row(ui, editor, t!("Contrôle de flux", "Flow control"), t!("Qui dit à l'autre d'attendre : personne, des caractères, ou des fils.", "Who tells the other to wait: no one, characters, or wires."), |ui, profile| {
        widgets::segmented(
            ui,
            &mut profile.flow_control,
            &[(FlowControl::None, t!("Aucun", "None")), (FlowControl::Software, "XON/XOFF"), (FlowControl::Hardware, "RTS/CTS")],
            t!("XON/XOFF : par des caractères ; RTS/CTS : par deux fils", "XON/XOFF: by characters; RTS/CTS: by two wires"),
        );
    });
    row(ui, editor, "DTR", t!("Une ligne de commande du port. Certaines cartes (ESP32, Arduino) redémarrent quand elle change.", "A control line of the port. Some boards (ESP32, Arduino) restart when it changes."), |ui, profile| {
        widgets::switch(ui, &mut profile.dtr, t!("Active une fois le port ouvert", "Set once the port is open"), t!("Coupée, la carte ne redémarre pas à l'ouverture", "Off, the board does not restart when opened"));
    });
    row(ui, editor, "RTS", t!("L'autre ligne de commande ; le contrôle RTS/CTS la pilote lui-même.", "The other control line; the RTS/CTS flow control drives it."), |ui, profile| {
        let hardware = profile.flow_control == FlowControl::Hardware;
        ui.add_enabled_ui(!hardware, |ui| {
            widgets::switch(ui, &mut profile.rts, t!("Active une fois le port ouvert", "Set once the port is open"), t!("Sans effet avec le contrôle RTS/CTS", "No effect with the RTS/CTS flow control"));
        });
    });
    row(ui, editor, t!("Fin de ligne", "Line ending"), t!("Ce qui suit chaque ligne envoyée.", "What follows each line sent."), |ui, profile| {
        widgets::segmented(
            ui,
            &mut profile.line_ending,
            &[(LineEnding::None, t!("Rien", "None")), (LineEnding::Lf, "LF"), (LineEnding::Cr, "CR"), (LineEnding::CrLf, "CR LF")],
            t!("LF le plus souvent ; CR LF pour les modems et les commandes AT", "LF most of the time; CR LF for modems and AT commands"),
        );
    });
}

fn console(ui: &mut egui::Ui, editor: &mut Editor) {
    row(
        ui,
        editor,
        t!("Mode terminal", "Terminal mode"),
        t!(
            "Les touches vont à l'appareil une à une, et son écran se dessine comme dans un terminal : vi, top, less, menus. Ctrl+C copie la sélection, sinon interrompt ; Ctrl+V colle. Les réglages ci-dessous sont ceux de la console en mode ligne.",
            "The keys go to the device one by one, and its screen is drawn as in a terminal: vi, top, less, menus. Ctrl+C copies the selection, else interrupts; Ctrl+V pastes. The settings below are the line console's."
        ),
        |ui, profile| {
            widgets::switch(ui, &mut profile.terminal, t!("L'écran d'un terminal", "A terminal's screen"), t!("Sinon, la console en mode ligne", "Otherwise, the line console"));
        },
    );
    row(ui, editor, t!("Horodatage", "Timestamps"), t!("L'heure à laquelle chaque ligne est arrivée.", "The time each line came."), |ui, profile| {
        widgets::switch(ui, &mut profile.timestamps, t!("Montrer l'heure de chaque ligne", "Show the time of each line"), t!("À la milliseconde", "To the millisecond"));
    });
    row(ui, editor, t!("Couleurs", "Colours"), t!("Les codes de couleur ANSI que l'appareil envoie, comme les journaux ESP-IDF.", "The ANSI colour codes the device sends, as ESP-IDF logs do."), |ui, profile| {
        widgets::switch(ui, &mut profile.ansi_colors, t!("Les couleurs de l'appareil", "The device's colours"), t!("Coupées, tout est d'une couleur", "Off, everything is of one colour"));
    });
    row(ui, editor, t!("Hexadécimal", "Hexadecimal"), t!("Les octets reçus et envoyés, tels quels, en hexadécimal.", "The bytes received and sent, as they are, in hexadecimal."), |ui, profile| {
        widgets::switch(ui, &mut profile.hex_view, t!("Afficher en hexadécimal", "Show in hexadecimal"), t!("Pour un appareil qui parle en octets", "For a device that speaks bytes"));
    });
    row(ui, editor, t!("Minimap", "Minimap"), t!("Toute la console en miniature, à sa droite ; un clic y déplace la vue.", "The whole console in miniature, at its right; a click moves the view there."), |ui, profile| {
        widgets::switch(ui, &mut profile.minimap, t!("Montrer la minimap", "Show the minimap"), t!("À partir d'une console assez large", "From a console wide enough"));
    });
    row(ui, editor, t!("Écho", "Echo"), t!("Les lignes envoyées, dans la console, après « > ».", "The lines sent, in the console, after \"> \"."), |ui, profile| {
        widgets::switch(ui, &mut profile.local_echo, t!("Montrer ce qui est envoyé", "Show what is sent"), t!("En bleu, après « > »", "In blue, after \"> \""));
    });
    row(ui, editor, t!("Lignes gardées", "Lines kept"), t!("Au-delà, les plus anciennes s'en vont ; le journal, lui, garde tout.", "Beyond, the oldest go; the journal keeps everything."), |ui, profile| {
        widgets::labelled(ui.add_sized(egui::vec2(SHORT, CONTROL_HEIGHT), egui::DragValue::new(&mut profile.scrollback).range(SCROLLBACK).speed(100))).on_hover_text(t!("Par onglet", "Per tab"));
    });
    row(ui, editor, t!("Taille du texte", "Text size"), t!("La taille des caractères de la console.", "The size of the console's characters."), |ui, profile| {
        widgets::labelled(ui.add_sized(egui::vec2(SHORT, CONTROL_HEIGHT), egui::DragValue::new(&mut profile.font_size).range(FONT_SIZE).speed(0.1).fixed_decimals(1).suffix(" pt")))
            .on_hover_text(t!("Glisser, ou cliquer et taper", "Drag, or click and type"));
    });
}

fn curves(ui: &mut egui::Ui, editor: &mut Editor) {
    row(ui, editor, t!("Courbes", "Curves"), t!("Le panneau des courbes, sous la console.", "The curves' panel, under the console."), |ui, profile| {
        widgets::switch(ui, &mut profile.plot, t!("Montrer les courbes", "Show the curves"), t!("Comme l'interrupteur « Courbes » de la fenêtre", "As the window's \"Curves\" switch"));
    });
    row(
        ui,
        editor,
        t!("Préfixe", "Prefix"),
        t!("Seules les lignes qui commencent par lui font des courbes, sans lui. Vide : toutes les lignes de valeurs.", "Only the lines that begin with it make curves, without it. Empty: every line of values."),
        |ui, profile| {
            widgets::labelled(ui.add(egui::TextEdit::singleline(&mut profile.plot_prefix).hint_text("PLOT").desired_width(SHORT).min_size(egui::vec2(SHORT, CONTROL_HEIGHT)).vertical_align(Align::Center)))
                .on_hover_text(t!("Par exemple PLOT, pour « PLOT 21.5 40 »", "For instance PLOT, for \"PLOT 21.5 40\""));
        },
    );
    row(ui, editor, t!("Points gardés", "Points kept"), t!("Par courbe ; les plus anciens s'en vont.", "Per curve; the oldest go."), |ui, profile| {
        widgets::labelled(ui.add_sized(egui::vec2(SHORT, CONTROL_HEIGHT), egui::DragValue::new(&mut profile.plot_points).range(PLOT_POINTS).speed(1000)))
            .on_hover_text(t!("Glisser, ou cliquer et taper", "Drag, or click and type"));
    });
    row(
        ui,
        editor,
        t!("Axe horizontal", "Horizontal axis"),
        t!(
            "Le temps, ou le numéro de la mesure : 1 pour la première ligne de mesures, 2 pour la suivante. Le numéro étale les mesures qu'un appareil envoie par paquets.",
            "The time, or the number of the measure: 1 for the first line of measures, 2 for the next. The number spreads the measures a device sends in bursts."
        ),
        |ui, profile| {
            widgets::segmented(
                ui,
                &mut profile.plot_x,
                &[(XAxis::Time, t!("Temps", "Time")), (XAxis::Number, t!("Numéro de la mesure", "Number of the measure"))],
                t!("Comme le choix « Temps | N° » des courbes", "As the curves' \"Time | No.\" choice"),
            );
        },
    );
    row(
        ui,
        editor,
        t!("Courbes du bas", "Curves below"),
        t!(
            "Les courbes du graphique du bas, chacun des deux graphiques avec son échelle. Elles se choisissent dans les courbes, par « Haut ou bas ».",
            "The curves of the chart below, each of the two charts with its own scale. They are chosen in the curves, with \"Above or below\"."
        ),
        |ui, profile| {
            ui.label(profile.describe(Field::PlotBelow));
            let empty = profile.plot_below.is_empty();
            let back = ui.add_enabled(!empty, egui::Button::new(t!("Tout remettre en haut", "All back above")).min_size(egui::vec2(0.0, CONTROL_HEIGHT)));
            if back.on_hover_text(t!("Un seul graphique, avec toutes les courbes", "A single chart, with every curve")).clicked() {
                profile.plot_below.clear();
            }
        },
    );
}

fn sending(ui: &mut egui::Ui, editor: &mut Editor) {
    row(ui, editor, t!("Saisie hex", "Hex input"), t!("La ligne d'envoi lue comme des octets : 48 65 6C, 0x48…", "The send line read as bytes: 48 65 6C, 0x48…"), |ui, profile| {
        widgets::switch(ui, &mut profile.hex_input, t!("Lire la ligne en hexadécimal", "Read the line in hexadecimal"), t!("Sans fin de ligne ajoutée", "With no line ending added"));
    });
    row(
        ui,
        editor,
        t!("Séquences", "Escapes"),
        t!("\\n, \\r, \\t, \\0, \\e, \\\\ et \\xNN lus comme l'octet qu'ils nomment : \\x03 envoie un Ctrl+C.", "\\n, \\r, \\t, \\0, \\e, \\\\ and \\xNN read as the byte they name: \\x03 sends a Ctrl+C."),
        |ui, profile| {
            widgets::switch(ui, &mut profile.escapes, t!("Lire les séquences dans la ligne", "Read the escapes in the line"), t!("Coupé, une barre oblique inverse part telle quelle", "Off, a backslash goes as it is"));
        },
    );
}

fn connection(ui: &mut egui::Ui, editor: &mut Editor) {
    row(
        ui,
        editor,
        t!("Port perdu", "Port lost"),
        t!("Un câble débranché, une carte qui redémarre : l'onglet attend son retour.", "A cable pulled out, a board that restarts: the tab waits for it to come back."),
        |ui, profile| {
            widgets::switch(ui, &mut profile.reconnect, t!("Le rouvrir dès son retour", "Open it again as soon as it is back"), t!("Un appareil qui a un profil est retrouvé même sur un autre port", "A device with a profile is found even on another port"));
        },
    );
}

fn journal(ui: &mut egui::Ui, editor: &mut Editor) {
    let muted = Palette::of(ui.visuals().dark_mode).muted;
    row(ui, editor, t!("Dossier", "Folder"), t!("Où les journaux sont créés ; sans dossier, le fichier est demandé à chaque fois.", "Where the journals are made; without a folder, the file is asked for each time."), |ui, profile| {
        if theme::secondary_button(ui, t!("Choisir…", "Choose…"), true).on_hover_text(t!("Choisir le dossier des journaux", "Choose the journals' folder")).clicked()
            && let Some(folder) = rfd::FileDialog::new().set_title(t!("Dossier des journaux", "Journals' folder")).pick_folder()
        {
            profile.journal_folder = folder.display().to_string();
        }
        if theme::secondary_button(ui, t!("Retirer", "Remove"), profile.journal_folder().is_some()).on_hover_text(t!("Demander le fichier à chaque journal", "Ask for the file at each journal")).clicked() {
            profile.journal_folder.clear();
        }
        let text = if profile.journal_folder.is_empty() { t!("Aucun", "None").to_owned() } else { profile.journal_folder.clone() };
        ui.label(RichText::new(text).color(muted));
    });
    row(ui, editor, t!("Démarrage", "Start"), t!("Un journal par connexion, dans le dossier, sans rien demander.", "A journal at each connection, in the folder, without asking."), |ui, profile| {
        let folder = profile.journal_folder().is_some();
        ui.add_enabled_ui(folder, |ui| {
            widgets::switch(ui, &mut profile.journal_on_connect, t!("À chaque connexion", "At each connection"), t!("Il faut d'abord un dossier", "A folder first"));
        });
    });
    row(ui, editor, t!("Horodatage", "Timestamps"), t!("L'heure de chaque ligne, dans le journal.", "The time of each line, in the journal."), |ui, profile| {
        widgets::switch(ui, &mut profile.journal_timestamps, t!("L'heure de chaque ligne", "The time of each line"), t!("À la milliseconde", "To the millisecond"));
    });
}

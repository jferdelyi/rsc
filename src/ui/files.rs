//! What goes to and from files: the system's save window and the kinds it offers, the console
//! exported or copied, the journal, the curves as CSV, a file sent to the device.

use std::fs;
use std::path::PathBuf;

use eframe::egui;

use crate::console::Time;
use crate::journal::Journal;
use crate::link::Command;
use super::App;
use super::top::tab_name;

/// The largest file sent at once, and the pieces it is sent in.
const LARGEST_FILE: u64 = 16 * 1024 * 1024;
const FILE_PIECE: usize = 4096;
/// What a file is saved as.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Saved {
    /// The console, or a journal.
    Journal,
    Curves,
    /// The licences' notices.
    Notices,
}

/// The kinds of file a save window offers: the file's own kind, then every file. Each named with
/// its pattern, as the system's window shows the name only; the first one gives the extension a
/// name typed without one takes.
pub(super) fn save_filters(saved: Saved) -> [(&'static str, &'static str); 2] {
    let own = match saved {
        Saved::Journal => (t!("Journal (*.log)", "Log (*.log)"), "log"),
        Saved::Curves => ("CSV (*.csv)", "csv"),
        Saved::Notices => (t!("Texte (*.txt)", "Text (*.txt)"), "txt"),
    };
    [own, (t!("Tous les fichiers (*.*)", "All files (*.*)"), "*")]
}

/// The system's window to save a file of that kind, `name` proposed: the path chosen, if one is.
pub(super) fn save_dialog(title: &str, name: &str, saved: Saved) -> Option<PathBuf> {
    save_filters(saved).into_iter().fold(rfd::FileDialog::new().set_title(title).set_file_name(name), |dialog, (kind, extension)| dialog.add_filter(kind, &[extension])).save_file()
}

/// A name a file can bear: "/dev/ttyUSB0" gives "ttyUSB0", "Carte Nano" gives "Carte-Nano".
pub(super) fn file_stem(name: &str) -> String {
    // A port's path, not a named device's name with a slash in it: its last part.
    let last = if name.starts_with(['/', '\\']) { name.rsplit(['/', '\\']).next().unwrap_or(name) } else { name };
    let stem: String = last.chars().map(|c| if c.is_alphanumeric() || c == '-' || c == '_' { c } else { '-' }).collect();
    let stem = stem.trim_matches('-');
    if stem.is_empty() { "rsc".to_owned() } else { stem.to_owned() }
}

impl App {
    pub(super) fn send_file(&mut self, i: usize, now: Time) {
        let Some(path) = rfd::FileDialog::new().set_title(t!("Envoyer un fichier", "Send a file")).pick_file() else {
            return;
        };
        let name = path.file_name().map_or_else(|| path.display().to_string(), |name| name.to_string_lossy().into_owned());
        let session = &mut self.sessions[i];
        let bytes = match fs::metadata(&path) {
            Ok(meta) if meta.len() > LARGEST_FILE => Err(tr!("{name} dépasse 16 Mo.", "{name} is over 16 MB.")),
            _ => fs::read(&path).map_err(|error| tr!("{name} ne se lit pas : {error}", "{name} cannot be read: {error}")),
        };
        match bytes {
            Ok(bytes) => {
                for piece in bytes.chunks(FILE_PIECE) {
                    session.link.send(Command::Send(piece.to_vec()));
                }
                session.console.notice(&tr!("Fichier envoyé : {name} ({} octets)", "File sent: {name} ({} bytes)", bytes.len()), now);
            }
            Err(problem) => session.problem = Some(problem),
        }
    }

    pub(super) fn copy(&self, ctx: &egui::Context, i: usize) {
        let session = &self.sessions[i];
        if let Some(terminal) = &session.terminal {
            ctx.copy_text(session.terminal_view.selected_text(terminal).unwrap_or_else(|| terminal.screen().contents()));
            return;
        }
        let shown = self.shown(i);
        ctx.copy_text(session.view.selected_text(&session.console, shown).unwrap_or_else(|| session.console.text(shown)));
    }

    pub(super) fn save_console(&mut self, i: usize, now: Time) {
        let name = format!("rsc-{}-{}.log", file_stem(&tab_name(&self.settings.tabs[i])), now.format("%Y%m%d-%H%M%S"));
        let Some(path) = save_dialog(t!("Enregistrer la console", "Save the console"), &name, Saved::Journal) else {
            return;
        };
        let text = self.sessions[i].console.text(self.shown(i));
        let session = &mut self.sessions[i];
        match fs::write(&path, text) {
            Ok(()) => session.console.notice(&tr!("Console enregistrée dans {}", "Console saved in {}", path.display()), now),
            Err(error) => session.problem = Some(tr!("{} ne s'écrit pas : {error}", "{} cannot be written: {error}", path.display())),
        }
    }

    /// A journal in the journals' folder, or in a file asked for when there is none.
    pub(super) fn start_journal(&mut self, i: usize, now: Time) {
        let name = format!("rsc-{}-{}.log", file_stem(&tab_name(&self.settings.tabs[i])), now.format("%Y%m%d-%H%M%S"));
        let path = match self.settings.profile_of(i).journal_folder() {
            Some(folder) => folder.join(name),
            None => match save_dialog(t!("Journal", "Journal"), &name, Saved::Journal) {
                Some(path) => path,
                None => return,
            },
        };
        let session = &mut self.sessions[i];
        let started = path.parent().map_or(Ok(()), fs::create_dir_all).and_then(|()| Journal::start(&path));
        match started {
            Ok(journal) => {
                session.console.follow(true);
                session.console.notice(&tr!("Journal : {}", "Journal: {}", path.display()), now);
                session.journal = Some(journal);
            }
            Err(error) => session.problem = Some(tr!("Journal impossible dans {} : {error}", "No journal in {}: {error}", path.display())),
        }
    }

    pub(super) fn stop_journal(&mut self, i: usize, now: Time) {
        // What ended before the stop goes in.
        self.hand_lines(i);
        let session = &mut self.sessions[i];
        if session.journal.take().is_some() {
            session.console.notice(t!("Journal arrêté", "Journal stopped"), now);
        }
    }

    pub(super) fn export_curves(&mut self, i: usize, now: Time) {
        let name = format!("rsc-{}-{}-courbes.csv", file_stem(&tab_name(&self.settings.tabs[i])), now.format("%Y%m%d-%H%M%S"));
        let Some(path) = save_dialog(t!("Exporter les courbes", "Export the curves"), &name, Saved::Curves) else {
            return;
        };
        let session = &mut self.sessions[i];
        match fs::write(&path, session.plot.csv(t!("temps (s)", "time (s)"))) {
            Ok(()) => session.console.notice(&tr!("Courbes exportées dans {}", "Curves exported to {}", path.display()), now),
            Err(error) => session.problem = Some(tr!("{} ne s'écrit pas : {error}", "{} cannot be written: {error}", path.display())),
        }
    }
}

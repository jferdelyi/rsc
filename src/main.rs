//! RSC, the executable: the window of `rsc::ui`.

// No console behind the window on Windows.
#![windows_subsystem = "windows"]
// No unsafe code, as in the library.
#![forbid(unsafe_code)]

use eframe::egui;
use rsc::ui;

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title(ui::TITLE)
            .with_app_id(ui::APP_ID)
            .with_icon(ui::icon())
            .with_inner_size([1100.0, 720.0])
            .with_min_inner_size(ui::MIN_SIZE),
        ..Default::default()
    };
    eframe::run_native(ui::APP_ID, options, Box::new(|cc| Ok(Box::new(ui::App::system(cc)))))
}

//! French or English: every text of the window is written in both, side by side, where it is used.

#[cfg(not(test))]
static ENGLISH: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

// The tests run side by side, each its window in its language: one choice per thread.
#[cfg(test)]
thread_local! {
    static ENGLISH: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

/// The language of the window: French unless English is asked for.
pub fn set_english(on: bool) {
    #[cfg(not(test))]
    ENGLISH.store(on, std::sync::atomic::Ordering::Relaxed);
    #[cfg(test)]
    ENGLISH.with(|english| english.set(on));
}

pub fn english() -> bool {
    #[cfg(not(test))]
    return ENGLISH.load(std::sync::atomic::Ordering::Relaxed);
    #[cfg(test)]
    return ENGLISH.with(std::cell::Cell::get);
}

/// A text in the window's language: `t!("Envoyer", "Send")`.
#[macro_export]
macro_rules! t {
    ($french:expr, $english:expr) => {
        if $crate::i18n::english() { $english } else { $french }
    };
}

/// A text with values in it, in the window's language: `tr!("Connecté à {port}", "Connected to {port}")`.
#[macro_export]
macro_rules! tr {
    ($french:literal, $english:literal $(, $($arguments:tt)*)?) => {
        if $crate::i18n::english() { format!($english $(, $($arguments)*)?) } else { format!($french $(, $($arguments)*)?) }
    };
}

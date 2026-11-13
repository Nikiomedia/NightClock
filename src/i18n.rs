//! Zweisprachigkeit (Deutsch / Englisch).
//!
//! Die aktuelle Sprache liegt global, damit Texte überall einfach mit
//! `t("Deutsch", "English")` gewählt werden können.

use std::sync::atomic::{AtomicBool, Ordering};

static ENGLISH: AtomicBool = AtomicBool::new(false);

pub fn set_english(on: bool) {
    ENGLISH.store(on, Ordering::Relaxed);
}

pub fn is_en() -> bool {
    ENGLISH.load(Ordering::Relaxed)
}

/// Text in der aktuellen Sprache.
pub fn t(de: &'static str, en: &'static str) -> &'static str {
    if is_en() {
        en
    } else {
        de
    }
}

const WEEKDAYS_DE: [&str; 7] = [
    "Montag",
    "Dienstag",
    "Mittwoch",
    "Donnerstag",
    "Freitag",
    "Samstag",
    "Sonntag",
];
const WEEKDAYS_EN: [&str; 7] = [
    "Monday",
    "Tuesday",
    "Wednesday",
    "Thursday",
    "Friday",
    "Saturday",
    "Sunday",
];
const MONTHS_DE: [&str; 12] = [
    "Januar",
    "Februar",
    "März",
    "April",
    "Mai",
    "Juni",
    "Juli",
    "August",
    "September",
    "Oktober",
    "November",
    "Dezember",
];
const MONTHS_EN: [&str; 12] = [
    "January",
    "February",
    "March",
    "April",
    "May",
    "June",
    "July",
    "August",
    "September",
    "October",
    "November",
    "December",
];
const DAY_SHORT_DE: [&str; 7] = ["Mo", "Di", "Mi", "Do", "Fr", "Sa", "So"];
const DAY_SHORT_EN: [&str; 7] = ["Mo", "Tu", "We", "Th", "Fr", "Sa", "Su"];

/// Wochentag, 0 = Montag
pub fn weekday(i: usize) -> &'static str {
    let i = i.min(6);
    if is_en() {
        WEEKDAYS_EN[i]
    } else {
        WEEKDAYS_DE[i]
    }
}

/// Monat, 1 = Januar
pub fn month(m: u32) -> &'static str {
    let i = (m as usize).saturating_sub(1).min(11);
    if is_en() {
        MONTHS_EN[i]
    } else {
        MONTHS_DE[i]
    }
}

/// Zweibuchstabige Tageskürzel Montag … Sonntag
pub fn day_short_all() -> &'static [&'static str; 7] {
    if is_en() {
        &DAY_SHORT_EN
    } else {
        &DAY_SHORT_DE
    }
}

/// Ist die Windows-Oberfläche auf Deutsch eingestellt?
#[cfg(windows)]
pub fn system_prefers_german() -> bool {
    const LANG_GERMAN: u16 = 0x07;
    let id = unsafe { windows_sys::Win32::Globalization::GetUserDefaultUILanguage() };
    id & 0x3ff == LANG_GERMAN
}

#[cfg(not(windows))]
pub fn system_prefers_german() -> bool {
    std::env::var("LANG").map(|l| l.starts_with("de")).unwrap_or(false)
}

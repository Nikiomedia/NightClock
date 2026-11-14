//! System-Schriftarten finden und in egui laden.

use egui::{FontData, FontDefinitions, FontFamily};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

pub const CLOCK_FAMILY: &str = "clock";
pub const UI_BOLD_FAMILY: &str = "ui-bold";
pub const DATE_FAMILY: &str = "date";

#[derive(Clone, Debug)]
pub struct FontEntry {
    pub name: String,
    pub path: PathBuf,
    pub index: u32,
    pub recommended: bool,
}

/// Schriften, die sich besonders schön als Uhr machen (falls installiert).
pub const RECOMMENDED: &[&str] = &[
    "Segoe UI Light",
    "Segoe UI Semilight",
    "Segoe UI Variable",
    "Bahnschrift",
    "Bahnschrift Light",
    "Cascadia Mono",
    "Cascadia Code",
    "Consolas",
    "Calibri Light",
    "Corbel Light",
    "Candara Light",
    "Georgia",
    "Sitka Text",
    "Ink Free",
    "Gabriola",
    "Segoe Print",
    "Lucida Console",
    "Trebuchet MS",
    "Segoe UI Black",
];

pub fn clock_family() -> FontFamily {
    FontFamily::Name(CLOCK_FAMILY.into())
}

/// Schrift für das Datum (fällt auf die Uhrschrift zurück).
pub fn date_family() -> FontFamily {
    FontFamily::Name(DATE_FAMILY.into())
}

pub fn ui_bold_family() -> FontFamily {
    FontFamily::Name(UI_BOLD_FAMILY.into())
}

fn windows_fonts_dir() -> PathBuf {
    let win = std::env::var_os("WINDIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("C:\\Windows"));
    win.join("Fonts")
}

fn font_dirs() -> Vec<PathBuf> {
    let mut v = vec![windows_fonts_dir()];
    if let Some(local) = std::env::var_os("LOCALAPPDATA") {
        v.push(PathBuf::from(local).join("Microsoft\\Windows\\Fonts"));
    }
    v
}

/// Standard-Uhrschrift beim ersten Start.
pub fn default_clock_font() -> Option<(String, PathBuf, u32)> {
    let dir = windows_fonts_dir();
    for (name, file) in [
        ("Segoe UI Light", "segoeuil.ttf"),
        ("Bahnschrift", "bahnschrift.ttf"),
        ("Segoe UI", "segoeui.ttf"),
    ] {
        let p = dir.join(file);
        if p.exists() {
            return Some((name.to_string(), p, 0));
        }
    }
    None
}

fn best_name(face: &ttf_parser::Face) -> Option<String> {
    let mut full_en = None;
    let mut full_any = None;
    let mut fam_en = None;
    for n in face.names() {
        if n.name_id == ttf_parser::name_id::FULL_NAME || n.name_id == ttf_parser::name_id::FAMILY
        {
            if let Some(s) = n.to_string() {
                let en = n.language_id == 0x0409 || n.platform_id == ttf_parser::PlatformId::Unicode;
                if n.name_id == ttf_parser::name_id::FULL_NAME {
                    if en && full_en.is_none() {
                        full_en = Some(s);
                    } else if full_any.is_none() {
                        full_any = Some(s);
                    }
                } else if en && fam_en.is_none() {
                    fam_en = Some(s);
                }
            }
        }
    }
    full_en.or(full_any).or(fam_en).map(|s| s.trim().to_string())
}

/// Durchsucht die Windows-Schriftordner (läuft im Hintergrund-Thread).
pub fn scan_system_fonts() -> Vec<FontEntry> {
    let mut list: Vec<FontEntry> = Vec::new();
    for dir in font_dirs() {
        let Ok(rd) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in rd.flatten() {
            let path = entry.path();
            let ext = path
                .extension()
                .and_then(|e| e.to_str())
                .map(|e| e.to_ascii_lowercase())
                .unwrap_or_default();
            if !matches!(ext.as_str(), "ttf" | "otf" | "ttc") {
                continue;
            }
            let Ok(data) = std::fs::read(&path) else {
                continue;
            };
            let count = ttf_parser::fonts_in_collection(&data).unwrap_or(1);
            for index in 0..count {
                let Ok(face) = ttf_parser::Face::parse(&data, index) else {
                    continue;
                };
                // Nur Schriften mit Ziffern und Doppelpunkt sind als Uhr brauchbar.
                if face.glyph_index('0').is_none()
                    || face.glyph_index('9').is_none()
                    || face.glyph_index(':').is_none()
                {
                    continue;
                }
                let Some(name) = best_name(&face) else {
                    continue;
                };
                if name.is_empty() || list.iter().any(|f| f.name == name) {
                    continue;
                }
                let recommended = RECOMMENDED.iter().any(|r| r.eq_ignore_ascii_case(&name));
                list.push(FontEntry {
                    name,
                    path: path.clone(),
                    index,
                    recommended,
                });
            }
        }
    }
    list.sort_by_key(|a| a.name.to_lowercase());
    list
}

pub type SharedFontList = Arc<Mutex<Option<Vec<FontEntry>>>>;

pub fn spawn_font_scan() -> SharedFontList {
    let shared: SharedFontList = Arc::new(Mutex::new(None));
    let s2 = shared.clone();
    std::thread::spawn(move || {
        let list = scan_system_fonts();
        if let Ok(mut g) = s2.lock() {
            *g = Some(list);
        }
    });
    shared
}

/// Baut die egui-Schriftdefinitionen: Segoe UI für die Oberfläche,
/// die gewählte Schrift für die Uhr und optional eine eigene für das Datum –
/// jeweils mit egui-Standardschriften als Fallback.
pub fn build_font_definitions(
    clock_font: Option<(&Path, u32)>,
    date_font: Option<(&Path, u32)>,
) -> FontDefinitions {
    let mut defs = FontDefinitions::default();
    let dir = windows_fonts_dir();
    let default_prop = defs
        .families
        .get(&FontFamily::Proportional)
        .cloned()
        .unwrap_or_default();

    // Oberfläche: Segoe UI
    let mut ui_fonts: Vec<String> = Vec::new();
    if let Ok(bytes) = std::fs::read(dir.join("segoeui.ttf")) {
        defs.font_data
            .insert("segoe".into(), Arc::new(FontData::from_owned(bytes)));
        ui_fonts.push("segoe".into());
    }
    let mut prop = ui_fonts.clone();
    prop.extend(default_prop.iter().cloned());
    defs.families.insert(FontFamily::Proportional, prop.clone());

    // Fett für Überschriften
    let mut bold = Vec::new();
    if let Ok(bytes) = std::fs::read(dir.join("seguisb.ttf")) {
        defs.font_data
            .insert("segoe-sb".into(), Arc::new(FontData::from_owned(bytes)));
        bold.push("segoe-sb".to_string());
    }
    bold.extend(prop.iter().cloned());
    defs.families.insert(ui_bold_family(), bold);

    // Uhr
    let mut clock = Vec::new();
    if let Some((path, index)) = clock_font {
        if let Ok(bytes) = std::fs::read(path) {
            let mut fd = FontData::from_owned(bytes);
            fd.index = index;
            defs.font_data.insert("clock".into(), Arc::new(fd));
            clock.push("clock".to_string());
        }
    }
    clock.extend(prop.iter().cloned());

    // Datum: eigene Schrift, sonst wie die Uhr
    let mut date = Vec::new();
    if let Some((path, index)) = date_font {
        if let Ok(bytes) = std::fs::read(path) {
            let mut fd = FontData::from_owned(bytes);
            fd.index = index;
            defs.font_data.insert("date".into(), Arc::new(fd));
            date.push("date".to_string());
        }
    }
    date.extend(clock.iter().cloned());
    defs.families.insert(date_family(), date);
    defs.families.insert(clock_family(), clock);

    defs
}

#[cfg(test)]
mod tests {
    #[test]
    fn finds_system_fonts() {
        let list = super::scan_system_fonts();
        let rec: Vec<_> = list.iter().filter(|f| f.recommended).map(|f| f.name.clone()).collect();
        println!("{} Schriften, empfohlen: {:?}", list.len(), rec);
        assert!(!list.is_empty());
    }
}

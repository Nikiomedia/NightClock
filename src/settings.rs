//! Einstellungen – werden als JSON unter %APPDATA%\NightClock\settings.json gespeichert.

use crate::i18n::t;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Debug)]
pub enum ClockStyle {
    Text,
    Segment,
    Analog,
}

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum AmbientMode {
    Off,
    Plasma,
    Lava,
    Aurora,
    Stars,
    Fireflies,
    Breath,
}

impl AmbientMode {
    pub const ALL: [AmbientMode; 7] = [
        AmbientMode::Off,
        AmbientMode::Stars,
        AmbientMode::Aurora,
        AmbientMode::Fireflies,
        AmbientMode::Lava,
        AmbientMode::Plasma,
        AmbientMode::Breath,
    ];

    pub fn label(self) -> &'static str {
        match self {
            AmbientMode::Off => t("Aus", "Off"),
            AmbientMode::Plasma => t("Plasma", "Plasma"),
            AmbientMode::Lava => t("Lavalampe", "Lava lamp"),
            AmbientMode::Aurora => t("Polarlicht", "Aurora"),
            AmbientMode::Stars => t("Sternenhimmel", "Starry sky"),
            AmbientMode::Fireflies => t("Glühwürmchen", "Fireflies"),
            AmbientMode::Breath => t("Atem-Licht", "Breathing light"),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Debug)]
pub enum Language {
    De,
    En,
}

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Debug)]
pub enum LayoutMode {
    Auto,
    Single,
    Stacked,
}

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Debug)]
pub enum SecondsMode {
    Off,
    Digits,
    Line,
}

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Debug)]
pub enum AlarmSound {
    Bell,
    Morning,
    Classic,
    Custom,
}

impl AlarmSound {
    pub const ALL: [AlarmSound; 4] = [
        AlarmSound::Bell,
        AlarmSound::Morning,
        AlarmSound::Classic,
        AlarmSound::Custom,
    ];

    pub fn label(self) -> &'static str {
        match self {
            AlarmSound::Bell => t("Sanfte Glocke", "Gentle bell"),
            AlarmSound::Morning => t("Morgenmelodie", "Morning melody"),
            AlarmSound::Classic => t("Klassischer Wecker", "Classic alarm"),
            AlarmSound::Custom => t("Eigene Audiodatei", "Custom audio file"),
        }
    }
}

pub const ALARM_NAME_MAX: usize = 40;

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
#[serde(default)]
pub struct Alarm {
    pub enabled: bool,
    /// Frei wählbarer Name, z. B. "Arbeit" (leer = kein Name)
    pub name: String,
    pub hour: u32,
    pub minute: u32,
    /// Montag … Sonntag. Kein Tag gewählt = einmaliger Wecker.
    pub days: [bool; 7],
}

impl Default for Alarm {
    fn default() -> Self {
        Self {
            enabled: true,
            name: String::new(),
            hour: 7,
            minute: 0,
            days: [true, true, true, true, true, false, false],
        }
    }
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
#[serde(default)]
pub struct Settings {
    pub language: Language,

    // ── Darstellung ─────────────────────────────
    pub style: ClockStyle,
    pub font_name: String,
    pub font_path: String,
    pub font_index: u32,
    /// Schriftstärke bei variablen Schriften (0 = Standard)
    pub font_weight: f32,
    pub color: [u8; 3],
    pub background: [u8; 3],
    pub brightness: f32,
    pub glow: f32,
    pub scale: f32,
    /// Position der Uhr: -1 = ganz links bzw. oben, 0 = Mitte, 1 = ganz rechts bzw. unten
    pub clock_x: f32,
    pub clock_y: f32,

    // ── Stimmung (bewegter Hintergrund) ─────────
    pub ambient: AmbientMode,
    pub ambient_intensity: f32,
    pub ambient_speed: f32,
    pub ambient_tint: bool,
    /// Minuten ohne Bedienung bis zum Ausblenden (0 = nie)
    pub ambient_timeout: u32,

    // ── Anzeige ─────────────────────────────────
    pub use_24h: bool,
    pub layout: LayoutMode,
    pub leading_zero: bool,
    pub seconds: SecondsMode,
    pub show_date: bool,
    /// Eigene Schrift für das Datum (leerer Pfad = wie die Uhrzeit)
    pub date_font_name: String,
    pub date_font_path: String,
    pub date_font_index: u32,
    /// Größe des Datums relativ zur Standardgröße
    pub date_scale: f32,
    /// Helligkeit des Datums relativ zur Uhrzeit
    pub date_brightness: f32,
    pub show_next_alarm: bool,
    pub blink_colon: bool,
    pub ghost_segments: bool,
    pub slanted_segments: bool,

    // ── Nachtmodus ──────────────────────────────
    pub night_dim: bool,
    /// Minuten nach Mitternacht
    pub night_start: u32,
    pub night_end: u32,
    pub night_brightness: f32,

    // ── Bildschirm ──────────────────────────────
    pub monitor_device: String,
    pub start_fullscreen: bool,
    pub keep_awake: bool,
    pub burn_in_shift: bool,
    pub hide_cursor: bool,
    pub always_on_top: bool,
    /// Sparmodus für schwache Grafik (weniger Bilder pro Sekunde, einfacheres Leuchten)
    pub eco_mode: bool,
    pub window_rect: Option<[i32; 4]>,
    /// War das Fenster beim Beenden maximiert?
    pub window_maximized: bool,
    /// War beim Beenden das Vollbild aktiv?
    pub was_fullscreen: bool,

    // ── Wecker ──────────────────────────────────
    pub alarms: Vec<Alarm>,
    pub sound: AlarmSound,
    pub custom_sound_path: String,
    pub volume: f32,
    pub fade_in_secs: f32,
    pub snooze_minutes: u32,
    pub sunrise: bool,
    pub sunrise_minutes: u32,
    pub auto_stop_minutes: u32,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            language: if crate::i18n::system_prefers_german() {
                Language::De
            } else {
                Language::En
            },
            style: ClockStyle::Text,
            font_name: String::new(),
            font_path: String::new(),
            font_index: 0,
            font_weight: 0.0,
            color: [255, 179, 92],
            background: [0, 0, 0],
            brightness: 0.6,
            glow: 0.55,
            scale: 0.85,
            clock_x: 0.0,
            clock_y: 0.0,

            ambient: AmbientMode::Off,
            ambient_intensity: 0.6,
            ambient_speed: 1.0,
            ambient_tint: false,
            ambient_timeout: 0,

            use_24h: true,
            layout: LayoutMode::Auto,
            leading_zero: true,
            seconds: SecondsMode::Off,
            show_date: true,
            date_font_name: String::new(),
            date_font_path: String::new(),
            date_font_index: 0,
            date_scale: 1.0,
            date_brightness: 0.5,
            show_next_alarm: true,
            blink_colon: true,
            ghost_segments: true,
            slanted_segments: true,

            night_dim: false,
            night_start: 23 * 60,
            night_end: 6 * 60 + 30,
            night_brightness: 0.2,

            monitor_device: String::new(),
            start_fullscreen: false,
            keep_awake: true,
            burn_in_shift: true,
            hide_cursor: true,
            always_on_top: false,
            eco_mode: false,
            window_rect: None,
            window_maximized: false,
            was_fullscreen: false,

            alarms: vec![Alarm {
                enabled: false,
                ..Alarm::default()
            }],
            sound: AlarmSound::Bell,
            custom_sound_path: String::new(),
            volume: 0.7,
            fade_in_secs: 45.0,
            snooze_minutes: 9,
            sunrise: true,
            sunrise_minutes: 15,
            auto_stop_minutes: 15,
        }
    }
}

pub fn config_dir() -> PathBuf {
    let base = std::env::var_os("APPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."));
    base.join("NightClock")
}

fn config_path() -> PathBuf {
    config_dir().join("settings.json")
}

impl Settings {
    pub fn load() -> Self {
        let mut s: Settings = std::fs::read_to_string(config_path())
            .ok()
            .and_then(|txt| serde_json::from_str(txt.trim_start_matches('\u{feff}')).ok())
            .unwrap_or_default();
        s.sanitize();
        s
    }

    pub fn save(&self) {
        let dir = config_dir();
        if std::fs::create_dir_all(&dir).is_err() {
            return;
        }
        if let Ok(txt) = serde_json::to_string_pretty(self) {
            let tmp = dir.join("settings.json.tmp");
            if std::fs::write(&tmp, txt).is_ok() {
                let _ = std::fs::rename(&tmp, config_path());
            }
        }
    }

    fn sanitize(&mut self) {
        self.brightness = self.brightness.clamp(0.02, 1.0);
        self.night_brightness = self.night_brightness.clamp(0.02, 1.0);
        self.glow = self.glow.clamp(0.0, 1.0);
        self.scale = self.scale.clamp(0.3, 1.0);
        self.clock_x = self.clock_x.clamp(-1.0, 1.0);
        self.clock_y = self.clock_y.clamp(-1.0, 1.0);
        self.date_scale = self.date_scale.clamp(0.5, 2.5);
        self.date_brightness = self.date_brightness.clamp(0.05, 1.0);
        self.ambient_intensity = self.ambient_intensity.clamp(0.05, 1.0);
        self.ambient_speed = self.ambient_speed.clamp(0.2, 2.5);
        self.ambient_timeout = self.ambient_timeout.min(240);
        self.volume = self.volume.clamp(0.0, 1.0);
        self.fade_in_secs = self.fade_in_secs.clamp(0.0, 300.0);
        self.snooze_minutes = self.snooze_minutes.clamp(1, 60);
        self.sunrise_minutes = self.sunrise_minutes.clamp(1, 60);
        self.auto_stop_minutes = self.auto_stop_minutes.clamp(1, 120);
        self.night_start %= 24 * 60;
        self.night_end %= 24 * 60;
        for a in &mut self.alarms {
            a.hour %= 24;
            a.minute %= 60;
            if a.name.chars().count() > ALARM_NAME_MAX {
                a.name = a.name.chars().take(ALARM_NAME_MAX).collect();
            }
        }
    }
}

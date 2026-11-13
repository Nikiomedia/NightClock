//! Hauptlogik der App: Zustand, Eingaben, Vollbild, Wecker, Zeichnen.

use crate::alarm;
use crate::audio::AlarmPlayer;
use crate::clock::{self, falloff, ClockTime, Palette};
use crate::fonts;
use crate::platform::{self, MonitorInfo};
use crate::ambient::AmbientParams;
use crate::settings::{AmbientMode, ClockStyle, SecondsMode, Settings};
use crate::i18n::t;
use crate::ui;
use crate::vfont::VectorFont;
use chrono::{DateTime, Local, Timelike};
use egui::{pos2, vec2, Color32, CursorIcon, Key, Mesh, Rect, Sense, Shape, ViewportCommand};
use std::path::PathBuf;
use std::time::{Duration, Instant};

pub struct Ringing {
    pub hour: u32,
    pub minute: u32,
    pub started: Instant,
    pub player: Option<AlarmPlayer>,
}

enum Pending {
    /// Fensterrechteck setzen; mehrere Durchgänge, weil Windows die Größe beim
    /// Wechsel auf einen Monitor mit anderer Skalierung (DPI) mitskaliert.
    RestoreRect([i32; 4], u8),
    EnterFullscreen,
}

pub struct NightClockApp {
    pub s: Settings,
    saved: Settings,
    last_save_check: Instant,
    loaded_font: Option<(String, u32)>,
    loaded_vfont: (String, u32, u32),
    pub vfont: VectorFont,
    pub ambient: crate::ambient::Ambient,
    pub font_list: fonts::SharedFontList,
    font_scan_started: bool,
    pub font_search: String,
    pub monitors: Vec<MonitorInfo>,
    pub settings_open: bool,
    pub last_activity: Instant,
    hwnd: Option<isize>,
    frame_count: u64,
    force_fullscreen: bool,
    windowed_rect: Option<[i32; 4]>,
    windowed_maximized: bool,
    /// Nach dem Wiederherstellen der Position maximieren
    restore_maximized: bool,
    pending: Option<(Pending, u32)>,
    pending_tries: u32,
    pub toast: Option<(String, Instant)>,
    last_tick: DateTime<Local>,
    pub ringing: Option<Ringing>,
    pub snooze_until: Option<DateTime<Local>>,
    pub preview: Option<AlarmPlayer>,
    awake_state: Option<(bool, bool)>,
    applied_on_top: Option<bool>,
    accent_applied: Option<[u8; 3]>,
    start: Instant,
    scroll_accum: f32,
    pub is_fullscreen: bool,
    /// Nur für automatische Testbilder (Debug-Build)
    no_save: bool,
    dev_sunrise: Option<f32>,
    pub dev_scroll: Option<f32>,
    #[cfg(debug_assertions)]
    devshot: Option<crate::devshot::DevShot>,
}

impl NightClockApp {
    pub fn new(cc: &eframe::CreationContext<'_>, mut s: Settings, force_fullscreen: bool) -> Self {
        // Testbilder (nur Debug): gespeicherten Fenster-/Vollbild-Zustand ignorieren
        #[cfg(debug_assertions)]
        if std::env::var_os("NIGHTCLOCK_SHOT").is_some() {
            s.was_fullscreen = false;
            s.start_fullscreen = false;
            s.window_rect = None;
        }
        // Schrift prüfen / Standard wählen
        let font_ok = !s.font_path.is_empty() && PathBuf::from(&s.font_path).exists();
        if !font_ok {
            if let Some((name, path, index)) = fonts::default_clock_font() {
                s.font_name = name;
                s.font_path = path.to_string_lossy().into_owned();
                s.font_index = index;
            } else {
                s.font_name = "Standard".into();
                s.font_path.clear();
                s.font_index = 0;
            }
        }
        let path = PathBuf::from(&s.font_path);
        cc.egui_ctx.set_fonts(fonts::build_font_definitions(
            (!s.font_path.is_empty()).then_some((path.as_path(), s.font_index)),
        ));
        ui::apply_style(&cc.egui_ctx, s.color);

        let weight = (s.font_weight > 0.0).then_some(s.font_weight);
        let vfont = VectorFont::load(&s.font_path, s.font_index, weight);
        Self {
            loaded_font: Some((s.font_path.clone(), s.font_index)),
            loaded_vfont: (s.font_path.clone(), s.font_index, s.font_weight.to_bits()),
            vfont,
            ambient: crate::ambient::Ambient::new(),
            saved: s.clone(),
            s,
            last_save_check: Instant::now(),
            font_list: Default::default(),
            font_scan_started: false,
            font_search: String::new(),
            monitors: platform::monitors(),
            settings_open: false,
            last_activity: Instant::now(),
            hwnd: platform::hwnd_of(cc),
            frame_count: 0,
            force_fullscreen,
            windowed_rect: None,
            windowed_maximized: false,
            restore_maximized: false,
            pending: None,
            pending_tries: 0,
            toast: None,
            last_tick: Local::now(),
            ringing: None,
            snooze_until: None,
            preview: None,
            awake_state: None,
            applied_on_top: None,
            accent_applied: None,
            start: Instant::now(),
            scroll_accum: 0.0,
            is_fullscreen: false,
            no_save: false,
            dev_sunrise: None,
            dev_scroll: None,
            #[cfg(debug_assertions)]
            devshot: crate::devshot::DevShot::from_env(),
        }
    }

    pub fn show_toast(&mut self, text: impl Into<String>) {
        self.toast = Some((text.into(), Instant::now()));
    }

    // ───────────────────────── Vollbild & Monitore ─────────────────────────

    pub fn target_monitor(&self) -> Option<MonitorInfo> {
        if let Some(m) = self.monitors.iter().find(|m| m.device == self.s.monitor_device) {
            return Some(m.clone());
        }
        // Standard: der zweite (nicht primäre) Bildschirm, falls vorhanden
        self.monitors
            .iter()
            .find(|m| !m.primary)
            .or(self.monitors.first())
            .cloned()
    }

    pub fn refresh_monitors(&mut self) {
        self.monitors = platform::monitors();
    }

    pub fn toggle_fullscreen(&mut self, ctx: &egui::Context) {
        if self.is_fullscreen {
            self.exit_fullscreen(ctx);
        } else {
            self.enter_fullscreen(ctx);
        }
    }

    pub fn enter_fullscreen(&mut self, ctx: &egui::Context) {
        self.enter_fullscreen_inner(ctx, true);
    }

    fn enter_fullscreen_inner(&mut self, ctx: &egui::Context, remember: bool) {
        self.refresh_monitors();
        let target = self.target_monitor();
        if let Some(hwnd) = self.hwnd {
            if remember && !self.is_fullscreen {
                if let Some((r, maxed)) = platform::window_state(hwnd) {
                    self.windowed_rect = Some(r);
                    self.windowed_maximized = maxed;
                }
            }
            if let Some(m) = &target {
                platform::move_to_monitor(hwnd, m);
            }
        }
        if let Some(m) = &target {
            self.s.monitor_device = m.device.clone();
        }
        ctx.send_viewport_cmd(ViewportCommand::Fullscreen(true));
        let name = target
            .as_ref()
            .and_then(|m| m.number())
            .map(|n| format!("{} {n}", t("Bildschirm", "Display")))
            .unwrap_or_else(|| t("Vollbild", "Fullscreen").into());
        self.show_toast(format!("{name}  ·  {}", t("Esc oder F11 beendet das Vollbild", "Esc or F11 exits fullscreen")));
    }

    pub fn exit_fullscreen(&mut self, ctx: &egui::Context) {
        ctx.send_viewport_cmd(ViewportCommand::Fullscreen(false));
        if let Some(r) = self.windowed_rect {
            self.restore_maximized = self.windowed_maximized;
            self.pending = Some((Pending::RestoreRect(r, 2), 3));
        }
    }

    /// Bildschirm für das Vollbild wechseln (auch während des Vollbilds).
    pub fn switch_monitor(&mut self, ctx: &egui::Context, device: String) {
        self.s.monitor_device = device;
        if self.is_fullscreen {
            ctx.send_viewport_cmd(ViewportCommand::Fullscreen(false));
            self.pending = Some((Pending::EnterFullscreen, 3));
        }
    }

    pub fn next_monitor(&mut self, ctx: &egui::Context) {
        self.refresh_monitors();
        if self.monitors.len() < 2 {
            self.show_toast(t("Nur ein Bildschirm gefunden", "Only one display found"));
            return;
        }
        let cur = self.target_monitor().map(|m| m.device).unwrap_or_default();
        let idx = self.monitors.iter().position(|m| m.device == cur).unwrap_or(0);
        let next = self.monitors[(idx + 1) % self.monitors.len()].clone();
        self.switch_monitor(ctx, next.device.clone());
        if !self.is_fullscreen {
            self.show_toast(format!("{}: {}", t("Vollbild-Ziel", "Fullscreen target"), next.label(idx + 1)));
        }
    }

    fn process_pending(&mut self, ctx: &egui::Context) {
        let Some((action, wait)) = self.pending.take() else {
            return;
        };
        ctx.request_repaint();
        if wait > 0 {
            self.pending = Some((action, wait - 1));
            return;
        }
        // warten, bis das Vollbild wirklich verlassen wurde (max. ~2 s)
        if self.is_fullscreen {
            if self.pending_tries < 60 {
                self.pending_tries += 1;
                self.pending = Some((action, 1));
            } else {
                // immer noch Vollbild: Fensterposition nicht anfassen
                self.pending_tries = 0;
            }
            return;
        }
        self.pending_tries = 0;
        match action {
            Pending::RestoreRect(r, passes) => {
                if let Some(h) = self.hwnd {
                    if platform::rect_visible(r) {
                        platform::set_window_rect(h, r);
                        if passes > 1 {
                            self.pending = Some((Pending::RestoreRect(r, passes - 1), 4));
                            return;
                        }
                    }
                    if self.restore_maximized {
                        platform::maximize(h);
                    }
                }
                self.restore_maximized = false;
            }
            Pending::EnterFullscreen => self.enter_fullscreen_inner(ctx, false),
        }
    }

    /// Letzte Fensterposition, -größe und letzten Zustand wiederherstellen.
    fn on_first_frame(&mut self, ctx: &egui::Context) {
        let go_fullscreen =
            self.force_fullscreen || self.s.start_fullscreen || self.s.was_fullscreen;
        let rect = self.s.window_rect.filter(|r| platform::rect_visible(*r));
        if let (Some(h), Some(r)) = (self.hwnd, rect) {
            // 1. Durchgang verschiebt auf den richtigen Monitor (Windows skaliert dabei
            //    evtl. die Größe), der 2. setzt die exakte Größe.
            platform::set_window_rect(h, r);
            self.windowed_rect = Some(r);
            self.windowed_maximized = self.s.window_maximized;
            if !go_fullscreen {
                self.restore_maximized = self.s.window_maximized;
                self.pending = Some((Pending::RestoreRect(r, 1), 4));
            }
        }
        if go_fullscreen {
            self.enter_fullscreen_inner(ctx, rect.is_none());
        }
    }

    /// Aktuellen Fensterzustand in die Einstellungen übernehmen.
    fn capture_window_state(&mut self) {
        // nicht während Übergängen oder direkt nach dem Start
        if self.pending.is_some() || self.start.elapsed() < Duration::from_secs(2) {
            return;
        }
        self.s.was_fullscreen = self.is_fullscreen;
        if self.is_fullscreen {
            return;
        }
        if let Some(h) = self.hwnd {
            if let Some((r, maxed)) = platform::window_state(h) {
                self.s.window_rect = Some(r);
                self.s.window_maximized = maxed;
            }
        }
    }

    // ───────────────────────── Wecker ─────────────────────────

    fn check_alarms(&mut self, now: DateTime<Local>) {
        let last = self.last_tick;
        self.last_tick = now;

        if self.ringing.is_none() {
            if let Some(i) = alarm::due_alarm(&self.s.alarms, last, now) {
                let a = self.s.alarms[i].clone();
                if alarm::alarm_is_one_shot(&a) {
                    self.s.alarms[i].enabled = false;
                }
                self.snooze_until = None;
                self.start_ringing(a.hour, a.minute);
            } else if let Some(su) = self.snooze_until {
                if now >= su {
                    self.snooze_until = None;
                    self.start_ringing(su.hour(), su.minute());
                }
            }
        }

        let mut auto_stop = false;
        if let Some(r) = &self.ringing {
            if let Some(p) = &r.player {
                p.update();
            }
            if r.started.elapsed() > Duration::from_secs(self.s.auto_stop_minutes as u64 * 60) {
                auto_stop = true;
            }
        }
        if auto_stop {
            self.ringing = None;
        }

        let mut stop_preview = false;
        if let Some(p) = &mut self.preview {
            p.set_volume(self.s.volume);
            p.update();
            stop_preview = p.elapsed() > Duration::from_secs(10);
        }
        if stop_preview {
            self.preview = None;
        }
    }

    fn start_ringing(&mut self, hour: u32, minute: u32) {
        self.preview = None;
        let player = match AlarmPlayer::start(
            self.s.sound,
            &self.s.custom_sound_path,
            self.s.volume,
            self.s.fade_in_secs,
        ) {
            Ok(p) => Some(p),
            Err(e) => {
                self.show_toast(e);
                None
            }
        };
        self.ringing = Some(Ringing {
            hour,
            minute,
            started: Instant::now(),
            player,
        });
        self.settings_open = false;
        self.last_activity = Instant::now();
    }

    pub fn snooze(&mut self) {
        if self.ringing.take().is_some() {
            let until = Local::now() + chrono::Duration::minutes(self.s.snooze_minutes as i64);
            self.snooze_until = Some(until);
            self.show_toast(format!(
                "{} {:02}:{:02}",
                t("Schlummern bis", "Snoozing until"),
                until.hour(),
                until.minute()
            ));
        }
    }

    pub fn dismiss(&mut self) {
        self.ringing = None;
        self.snooze_until = None;
        self.show_toast(t("Wecker aus  ·  Guten Morgen!", "Alarm off  ·  Good morning!"));
    }

    pub fn toggle_preview(&mut self) {
        if self.preview.is_some() {
            self.preview = None;
        } else {
            match AlarmPlayer::start(self.s.sound, &self.s.custom_sound_path, self.s.volume, 0.0) {
                Ok(p) => self.preview = Some(p),
                Err(e) => self.show_toast(e),
            }
        }
    }

    /// 0..1 – wie weit der Lichtwecker (Sonnenaufgang) fortgeschritten ist.
    fn sunrise_progress(&self, now: DateTime<Local>) -> f32 {
        if let Some(v) = self.dev_sunrise {
            return v;
        }
        if self.ringing.is_some() {
            return if self.s.sunrise { 1.0 } else { 0.0 };
        }
        if !self.s.sunrise {
            return 0.0;
        }
        if self.snooze_until.is_some() {
            return 1.0;
        }
        let Some((t, _)) = alarm::next_alarm(&self.s.alarms, now) else {
            return 0.0;
        };
        let total = self.s.sunrise_minutes as f32 * 60.0;
        let remaining = (t - now).num_milliseconds() as f32 / 1000.0;
        if remaining > total || remaining < 0.0 {
            0.0
        } else {
            1.0 - remaining / total
        }
    }

    fn in_night_window(&self, now: DateTime<Local>) -> bool {
        let m = now.hour() * 60 + now.minute();
        let (a, b) = (self.s.night_start, self.s.night_end);
        if a <= b {
            m >= a && m < b
        } else {
            m >= a || m < b
        }
    }

    fn effective_brightness(&self, now: DateTime<Local>) -> f32 {
        let mut b = self.s.brightness;
        if self.s.night_dim && self.in_night_window(now) {
            b = b.min(self.s.night_brightness);
        }
        let sun = self.sunrise_progress(now);
        if sun > 0.0 {
            b += (1.0 - b) * sun * sun * 0.85;
        }
        if self.ringing.is_some() {
            b = b.max(0.9);
        }
        b.clamp(0.02, 1.0)
    }

    // ───────────────────────── Eingaben ─────────────────────────

    fn change_brightness(&mut self, delta: f32) {
        self.s.brightness = (self.s.brightness + delta).clamp(0.02, 1.0);
        self.show_toast(format!(
            "{}  {:.0} %",
            t("Helligkeit", "Brightness"),
            (self.s.brightness * 100.0).round()
        ));
    }

    fn handle_input(&mut self, ctx: &egui::Context) {
        let (moved, pressed_any, scroll) = ctx.input(|i| {
            (
                i.pointer.delta() != egui::Vec2::ZERO,
                i.pointer.any_pressed() || !i.keys_down.is_empty(),
                i.raw_scroll_delta.y,
            )
        });
        if moved || pressed_any {
            self.last_activity = Instant::now();
        }

        // Mausrad = Helligkeit (nicht über dem Einstellungsmenü)
        if scroll != 0.0 && !ctx.is_pointer_over_area() && self.ringing.is_none() {
            self.scroll_accum += scroll;
            let steps = (self.scroll_accum / 50.0).trunc();
            if steps != 0.0 {
                self.scroll_accum -= steps * 50.0;
                self.change_brightness(steps * 0.05);
            }
        }

        if ctx.wants_keyboard_input() {
            return;
        }
        let key = |k: Key| ctx.input(|i| i.key_pressed(k));

        if self.ringing.is_some() {
            if key(Key::Space) || key(Key::S) {
                self.snooze();
            } else if key(Key::Enter) || key(Key::Escape) {
                self.dismiss();
            }
            return;
        }

        if key(Key::F11) || key(Key::F) {
            self.toggle_fullscreen(ctx);
        }
        if key(Key::Escape) {
            if self.settings_open {
                self.settings_open = false;
            } else if self.is_fullscreen {
                self.exit_fullscreen(ctx);
            }
        }
        if key(Key::S) || key(Key::E) {
            self.settings_open = !self.settings_open;
        }
        if key(Key::ArrowUp) || key(Key::Plus) {
            self.change_brightness(0.05);
        }
        if key(Key::ArrowDown) || key(Key::Minus) {
            self.change_brightness(-0.05);
        }
        if key(Key::M) {
            self.next_monitor(ctx);
        }
        if key(Key::Num1) {
            self.s.style = ClockStyle::Text;
        }
        if key(Key::Num2) {
            self.s.style = ClockStyle::Segment;
        }
        if key(Key::Num3) {
            self.s.style = ClockStyle::Analog;
        }
        if key(Key::Z) && self.s.style != ClockStyle::Analog {
            let now_stacked = clock::stacked(ctx.screen_rect(), &self.s);
            self.s.layout = if now_stacked {
                crate::settings::LayoutMode::Single
            } else {
                crate::settings::LayoutMode::Stacked
            };
            self.show_toast(if now_stacked {
                t("Anordnung: Einzeilig", "Layout: single line")
            } else {
                t("Anordnung: Zweizeilig", "Layout: two lines")
            });
        }
    }

    // ───────────────────────── Systemzustand ─────────────────────────

    fn apply_font_if_changed(&mut self, ctx: &egui::Context) {
        let key = (self.s.font_path.clone(), self.s.font_index);
        if self.loaded_font.as_ref() != Some(&key) {
            let path = PathBuf::from(&key.0);
            ctx.set_fonts(fonts::build_font_definitions(
                (!key.0.is_empty()).then_some((path.as_path(), key.1)),
            ));
            self.loaded_font = Some(key);
            // neue Schrift -> Standardstärke
            self.s.font_weight = 0.0;
        }
        let vkey = (self.s.font_path.clone(), self.s.font_index, self.s.font_weight.to_bits());
        if self.loaded_vfont != vkey {
            let weight = (self.s.font_weight > 0.0).then_some(self.s.font_weight);
            self.vfont = VectorFont::load(&self.s.font_path, self.s.font_index, weight);
            self.loaded_vfont = vkey;
        }
    }

    fn apply_system_state(&mut self, ctx: &egui::Context) {
        if self.applied_on_top != Some(self.s.always_on_top) {
            ctx.send_viewport_cmd(ViewportCommand::WindowLevel(if self.s.always_on_top {
                egui::WindowLevel::AlwaysOnTop
            } else {
                egui::WindowLevel::Normal
            }));
            self.applied_on_top = Some(self.s.always_on_top);
        }
        // System wach halten, solange ein Wecker aktiv ist; Bildschirm nach Einstellung
        let any_alarm = self.s.alarms.iter().any(|a| a.enabled) || self.snooze_until.is_some();
        let want = (any_alarm, self.s.keep_awake);
        if self.awake_state != Some(want) {
            platform::set_awake(want.0, want.1);
            self.awake_state = Some(want);
        }
        if self.accent_applied != Some(self.s.color) {
            ui::apply_style(ctx, self.s.color);
            self.accent_applied = Some(self.s.color);
        }
    }

    #[cfg(debug_assertions)]
    fn devshot_tick(&mut self, ctx: &egui::Context) {
        let Some(mut ds) = self.devshot.take() else {
            return;
        };
        if self.frame_count == 2 {
            self.no_save = true;
            let state = ds.state.clone();
            for st in &state {
                let (k, v) = st.split_once('=').unwrap_or((st.as_str(), ""));
                match k {
                    "settings" => self.settings_open = true,
                    "fullscreen" => self.enter_fullscreen(ctx),
                    "en" => self.s.language = crate::settings::Language::En,
                    "tint" => self.s.ambient_tint = true,
                    "eco" => self.s.eco_mode = true,
                    "stacked" => self.s.layout = crate::settings::LayoutMode::Stacked,
                    "single" => self.s.layout = crate::settings::LayoutMode::Single,
                    "clean" => self.last_activity = Instant::now() - Duration::from_secs(30),
                    "ambient" => {
                        self.s.ambient = match v {
                            "plasma" => AmbientMode::Plasma,
                            "lava" => AmbientMode::Lava,
                            "aurora" => AmbientMode::Aurora,
                            "stars" => AmbientMode::Stars,
                            "fireflies" => AmbientMode::Fireflies,
                            "breath" => AmbientMode::Breath,
                            _ => AmbientMode::Off,
                        };
                        self.ambient.time = 37.0;
                    }
                    "ambient_k" => self.s.ambient_intensity = v.parse().unwrap_or(0.6),
                    "atime" => self.ambient.time = v.parse().unwrap_or(0.0),
                    "de" => self.s.language = crate::settings::Language::De,
                    "text" => self.s.style = ClockStyle::Text,
                    "segment" => self.s.style = ClockStyle::Segment,
                    "analog" => self.s.style = ClockStyle::Analog,
                    "line" => self.s.seconds = SecondsMode::Line,
                    "digits" => self.s.seconds = SecondsMode::Digits,
                    "nocolon" => self.s.blink_colon = false,
                    "hover" => self.last_activity = Instant::now(),
                    "alarm" => {
                        self.s.alarms = vec![crate::settings::Alarm::default()];
                    }
                    "ringing" => {
                        self.ringing = Some(Ringing {
                            hour: 7,
                            minute: 0,
                            started: Instant::now(),
                            player: None,
                        })
                    }
                    "toast" => self.show_toast("Bildschirm 2  ·  Esc oder F11 beendet das Vollbild"),
                    "sunrise" => self.dev_sunrise = v.parse().ok(),
                    "scroll" => self.dev_scroll = v.parse().ok(),
                    "bright" => self.s.brightness = v.parse().unwrap_or(1.0),
                    "glow" => self.s.glow = v.parse().unwrap_or(0.5),
                    "color" => {
                        let c: Vec<u8> = v.split('/').filter_map(|x| x.parse().ok()).collect();
                        if c.len() == 3 {
                            self.s.color = [c[0], c[1], c[2]];
                        }
                    }
                    "font" => {
                        self.s.font_path = format!("C:\\Windows\\Fonts\\{v}");
                        self.s.font_name = v.into();
                        self.s.font_index = 0;
                    }
                    "size" => {
                        if let Some((w, h)) = v.split_once('x') {
                            if let (Ok(w), Ok(h)) = (w.parse::<f32>(), h.parse::<f32>()) {
                                ctx.send_viewport_cmd(ViewportCommand::InnerSize(vec2(w, h)));
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
        if (self.settings_open || self.ringing.is_some() || ds.has("hover")) && !ds.has("clean") {
            self.last_activity = Instant::now();
        }
        if self.frame_count == 45 && !ds.requested {
            ds.requested = true;
            ctx.send_viewport_cmd(ViewportCommand::Screenshot(Default::default()));
        }
        let shot = ctx.input(|i| {
            i.raw.events.iter().find_map(|e| match e {
                egui::Event::Screenshot { image, .. } => Some(image.clone()),
                _ => None,
            })
        });
        if let Some(img) = shot {
            crate::devshot::save_png(&ds.path, img.width(), img.height(), img.as_raw());
            ctx.send_viewport_cmd(ViewportCommand::Close);
        }
        ctx.request_repaint();
        self.devshot = Some(ds);
    }

    fn autosave(&mut self) {
        if self.no_save {
            return;
        }
        if self.last_save_check.elapsed() < Duration::from_secs(1) {
            return;
        }
        self.last_save_check = Instant::now();
        self.capture_window_state();
        if self.s != self.saved {
            self.s.save();
            self.saved = self.s.clone();
        }
    }

    // ───────────────────────── Zeichnen ─────────────────────────

    fn burn_in_offset(&self, rect: Rect) -> egui::Vec2 {
        if !self.s.burn_in_shift {
            return egui::Vec2::ZERO;
        }
        let t = self.start.elapsed().as_secs_f32();
        let amp = rect.width().min(rect.height()) * 0.025;
        vec2(
            (t * std::f32::consts::TAU / 173.0).sin() * amp,
            (t * std::f32::consts::TAU / 241.0 + 1.3).sin() * amp * 0.7,
        )
    }

    fn draw_sunrise(painter: &egui::Painter, rect: Rect, k: f32) {
        let k = k.clamp(0.0, 1.0);
        let warm = |r: f32, g: f32, b: f32, i: f32| {
            Color32::from_rgb(
                (r * i).clamp(0.0, 255.0) as u8,
                (g * i).clamp(0.0, 255.0) as u8,
                (b * i).clamp(0.0, 255.0) as u8,
            )
            .additive()
        };
        let mut mesh = Mesh::default();
        let stops = [
            (0.0, warm(255.0, 105.0, 35.0, 0.62 * k)),
            (0.25, warm(255.0, 130.0, 55.0, 0.32 * k)),
            (0.6, warm(255.0, 165.0, 95.0, 0.08 * k)),
            (1.0, Color32::TRANSPARENT),
        ];
        for (j, &(f, c)) in stops.iter().enumerate() {
            let y = rect.bottom() - rect.height() * f;
            mesh.colored_vertex(pos2(rect.left(), y), c);
            mesh.colored_vertex(pos2(rect.right(), y), c);
            if j > 0 {
                let b = (j as u32 - 1) * 2;
                mesh.add_triangle(b, b + 1, b + 2);
                mesh.add_triangle(b + 1, b + 3, b + 2);
            }
        }
        painter.add(Shape::mesh(mesh));
        // Sonne hinter dem Horizont
        clock::radial_glow(
            painter,
            pos2(rect.center().x, rect.bottom() + rect.height() * 0.12),
            vec2(rect.width() * 0.55, rect.height() * 0.55),
            |t| warm(255.0, 150.0, 70.0, 0.5 * k * k * falloff(t)),
        );
    }

    fn draw_status_line(&self, painter: &egui::Painter, rect: Rect, shift: egui::Vec2, pal: &Palette, now: DateTime<Local>) {
        if !self.s.show_next_alarm || self.ringing.is_some() {
            return;
        }
        let text = if let Some(su) = self.snooze_until {
            format!("{} {:02}:{:02}", t("Schlummern bis", "Snoozing until"), su.hour(), su.minute())
        } else if let Some((t, _)) = alarm::next_alarm(&self.s.alarms, now) {
            format!(
                "{}  {:02}:{:02}",
                alarm::relative_day(t, now),
                t.hour(),
                t.minute()
            )
        } else {
            return;
        };
        let size = (rect.height() * 0.026).clamp(12.0, 28.0);
        let col = pal.col(0.42);
        let g = painter.ctx().fonts(|f| {
            f.layout_no_wrap(text, egui::FontId::new(size, fonts::clock_family()), col)
        });
        let icon = size * 0.95;
        let gap = size * 0.45;
        let total = icon + gap + g.size().x;
        let y = rect.bottom() - size * 2.4 + shift.y * 0.5;
        let x0 = rect.center().x - total * 0.5 + shift.x * 0.5;
        ui::bell_icon(painter, pos2(x0 + icon * 0.5, y), icon, col);
        painter.galley(pos2(x0 + icon + gap, y - g.size().y * 0.5), g, col);
    }

    fn draw_scene(&mut self, ui: &mut egui::Ui, ctx: &egui::Context, now: DateTime<Local>) {
        let rect = ui.max_rect();
        let resp = ui.interact(rect, ui.id().with("clock_bg"), Sense::click());
        if resp.double_clicked() {
            self.toggle_fullscreen(ctx);
        }
        if resp.secondary_clicked() && self.ringing.is_none() {
            self.settings_open = !self.settings_open;
        }

        let level = self.effective_brightness(now);
        let pal = Palette {
            base: self.s.color,
            level,
            glow: self.s.glow,
            eco: self.s.eco_mode,
        };
        let painter = ui.painter();

        // Hintergrund
        let bk = 0.25 + 0.75 * level;
        let bg = Color32::from_rgb(
            (self.s.background[0] as f32 * bk) as u8,
            (self.s.background[1] as f32 * bk) as u8,
            (self.s.background[2] as f32 * bk) as u8,
        );
        painter.rect_filled(rect, 0.0, bg);

        // Stimmungs-Hintergrund
        let fade = self.ambient_fade();
        if self.s.ambient != AmbientMode::Off && fade > 0.0 {
            let params = AmbientParams {
                mode: self.s.ambient,
                k: self.s.ambient_intensity * level.powf(0.8) * fade,
                tint: self.s.ambient_tint.then_some(self.s.color),
                bg: [bg.r(), bg.g(), bg.b()],
                eco: self.s.eco_mode,
            };
            self.ambient.paint(ctx, painter, rect, params, "main", 200.0, 0);
        }

        // Lichtwecker
        let sun = self.sunrise_progress(now);
        if sun > 0.0 {
            Self::draw_sunrise(painter, rect, sun);
        }

        let shift = self.burn_in_offset(rect);
        let mut status_h = if self.s.show_next_alarm { rect.height() * 0.07 } else { 0.0 };
        if self.ringing.is_some() && rect.height() > 420.0 {
            // Platz für die Wecker-Karte unten freihalten
            status_h = status_h.max(230.0 + rect.height() * 0.07);
        }
        let clock_rect = Rect::from_min_max(rect.min, pos2(rect.right(), rect.bottom() - status_h))
            .translate(shift);

        // Pulsierender Schein, während der Wecker klingelt
        if let Some(r) = &self.ringing {
            let t = r.started.elapsed().as_secs_f32();
            let pulse = 0.5 + 0.5 * (t * std::f32::consts::TAU / 1.8).sin();
            let c = clock_rect.center();
            let rad = clock_rect.size().min_elem() * 0.75;
            clock::radial_glow(painter, c, vec2(rad * 1.6, rad), |k| {
                pal.add((0.10 + 0.22 * pulse) * falloff(k))
            });
        }

        let t = ClockTime::from(now);
        clock::draw_clock(painter, clock_rect, &t, &self.s, &pal, &mut self.vfont);
        self.draw_status_line(painter, rect, shift, &pal, now);
    }

    /// 1 = sichtbar; blendet nach der eingestellten Zeit ohne Bedienung aus.
    pub fn ambient_fade(&self) -> f32 {
        if self.s.ambient_timeout == 0 || self.settings_open || self.ringing.is_some() {
            return 1.0;
        }
        let idle = self.last_activity.elapsed().as_secs_f32();
        let limit = self.s.ambient_timeout as f32 * 60.0;
        (1.0 - (idle - limit) / 60.0).clamp(0.0, 1.0)
    }

    fn needs_animation(&self, now: DateTime<Local>) -> bool {
        (self.s.ambient != AmbientMode::Off && self.ambient_fade() > 0.0)
            || self.settings_open
            || self.s.blink_colon
            || self.s.seconds == SecondsMode::Line
            || (self.s.style == ClockStyle::Analog && self.s.seconds != SecondsMode::Off)
            || self.ringing.is_some()
            || self.preview.is_some()
            || self.pending.is_some()
            || self.last_activity.elapsed() < Duration::from_secs(4)
            || self
                .toast
                .as_ref()
                .is_some_and(|(_, t)| t.elapsed() < Duration::from_secs(4))
            || self.sunrise_progress(now) > 0.0
    }
}

impl eframe::App for NightClockApp {
    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        let c = Color32::from_rgb(self.s.background[0], self.s.background[1], self.s.background[2]);
        egui::Rgba::from(c).to_array()
    }

    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        let now = Local::now();
        crate::i18n::set_english(self.s.language == crate::settings::Language::En);
        self.frame_count += 1;
        if self.hwnd.is_none() {
            self.hwnd = platform::hwnd_of(frame);
        }
        self.is_fullscreen = ctx.input(|i| i.viewport().fullscreen.unwrap_or(false));

        if self.frame_count == 1 {
            self.on_first_frame(ctx);
        }
        self.process_pending(ctx);
        // Schriftliste erst laden, wenn das Menü geöffnet wird
        if self.settings_open && !self.font_scan_started {
            self.font_list = fonts::spawn_font_scan();
            self.font_scan_started = true;
        }
        self.apply_font_if_changed(ctx);
        self.apply_system_state(ctx);
        self.check_alarms(now);
        self.handle_input(ctx);
        self.ambient.advance(self.s.ambient_speed);
        #[cfg(debug_assertions)]
        self.devshot_tick(ctx);

        egui::CentralPanel::default()
            .frame(egui::Frame::new())
            .show(ctx, |ui| self.draw_scene(ui, ctx, now));

        ui::hover_controls(self, ctx);
        ui::settings_panel(self, ctx);
        ui::ringing_overlay(self, ctx);
        ui::toast(self, ctx);

        // Mauszeiger im Vollbild ausblenden
        if self.is_fullscreen
            && self.s.hide_cursor
            && !self.settings_open
            && self.ringing.is_none()
            && self.last_activity.elapsed() > Duration::from_secs(3)
        {
            ctx.set_cursor_icon(CursorIcon::None);
        }

        if ctx.input(|i| i.viewport().close_requested()) && !self.no_save {
            self.capture_window_state();
            self.s.save();
            self.saved = self.s.clone();
        }
        self.autosave();

        if self.needs_animation(now) {
            // langsame Bewegungen: 20 Bilder/s genügen, im Sparmodus 10
            let ms = if self.s.eco_mode { 100 } else { 50 };
            ctx.request_repaint_after(Duration::from_millis(ms));
        } else {
            let ms = 1000 - now.timestamp_subsec_millis().min(999) as u64;
            ctx.request_repaint_after(Duration::from_millis(ms + 5));
        }
    }
}

impl Drop for NightClockApp {
    fn drop(&mut self) {
        if !self.no_save {
            self.s.save();
        }
        platform::set_awake(false, false);
    }
}

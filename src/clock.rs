//! Zeichnet die Uhr: Schrift-, Segment- und Analog-Stil, inkl. Leuchteffekt.

use crate::fonts::clock_family;
use crate::i18n;
use crate::settings::{ClockStyle, LayoutMode, SecondsMode, Settings};
use crate::vfont::VectorFont;
use chrono::{DateTime, Datelike, Local, Timelike};
use egui::epaint::TextShape;
use egui::text::{LayoutJob, TextFormat};
use egui::{pos2, vec2, Color32, FontId, Galley, Mesh, Painter, Pos2, Rect, Shape, Stroke, Vec2};
use std::f32::consts::TAU;
use std::sync::Arc;

#[derive(Clone, Copy, Debug)]
pub struct ClockTime {
    pub h: u32,
    pub m: u32,
    pub s: u32,
    pub frac: f32,
    pub weekday: usize,
    pub day: u32,
    pub month: u32,
}

impl ClockTime {
    pub fn from(now: DateTime<Local>) -> Self {
        Self {
            h: now.hour(),
            m: now.minute(),
            s: now.second().min(59),
            frac: (now.nanosecond() % 1_000_000_000) as f32 / 1e9,
            weekday: now.weekday().num_days_from_monday() as usize,
            day: now.day(),
            month: now.month(),
        }
    }

    pub fn date_text(&self) -> String {
        let (wd, mo) = (i18n::weekday(self.weekday), i18n::month(self.month));
        if i18n::is_en() {
            format!("{wd}, {mo} {}", self.day)
        } else {
            format!("{wd}, {}. {mo}", self.day)
        }
    }

    /// Kurzform für das Zifferblatt, z. B. "FR · 13. NOVEMBER"
    pub fn short_date_text(&self) -> String {
        let (wd, mo) = (i18n::weekday(self.weekday), i18n::month(self.month));
        if i18n::is_en() {
            format!("{} · {mo} {}", &wd[..3], self.day)
        } else {
            format!("{} · {}. {mo}", &wd[..2], self.day)
        }
    }
}

/// Farbe der Uhr inkl. Helligkeit und Leuchtstärke.
#[derive(Clone, Copy, Debug)]
pub struct Palette {
    pub base: [u8; 3],
    pub level: f32,
    pub glow: f32,
    /// Sparmodus: weniger Kopien für das Leuchten (schwache Grafik)
    pub eco: bool,
}

impl Palette {
    /// Deckende Farbe mit relativer Intensität `k`.
    pub fn col(&self, k: f32) -> Color32 {
        let f = (self.level * k).max(0.0);
        let c = |v: u8| ((v as f32 * f).round()).clamp(0.0, 255.0) as u8;
        Color32::from_rgb(c(self.base[0]), c(self.base[1]), c(self.base[2]))
    }

    /// Additive Lichtfarbe (Alpha 0 = wird aufaddiert).
    pub fn add(&self, k: f32) -> Color32 {
        self.col(k).additive()
    }

    /// Hellere Variante für Akzente (Sekundenzeiger).
    pub fn accent(&self, k: f32) -> Color32 {
        let f = (self.level * k).max(0.0);
        let c = |v: u8| {
            let l = v as f32 + (255.0 - v as f32) * 0.45;
            (l * f).round().clamp(0.0, 255.0) as u8
        };
        Color32::from_rgb(c(self.base[0]), c(self.base[1]), c(self.base[2]))
    }
}

pub fn falloff(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    (-3.2 * t * t).exp() * (1.0 - t)
}

/// Weicher, elliptischer Lichtschein (additiv).
pub fn radial_glow(p: &Painter, center: Pos2, radius: Vec2, color_at: impl Fn(f32) -> Color32) {
    const SEGS: usize = 72;
    const RINGS: usize = 28;
    let mut mesh = Mesh::default();
    mesh.colored_vertex(center, color_at(0.0));
    for j in 1..=RINGS {
        let t = j as f32 / RINGS as f32;
        let col = color_at(t);
        for i in 0..SEGS {
            let a = i as f32 / SEGS as f32 * TAU;
            mesh.colored_vertex(
                center + vec2(a.cos() * radius.x * t, a.sin() * radius.y * t),
                col,
            );
        }
    }
    let ring_start = |j: usize| 1 + (j - 1) * SEGS;
    for i in 0..SEGS {
        let i2 = (i + 1) % SEGS;
        mesh.add_triangle(0, (ring_start(1) + i) as u32, (ring_start(1) + i2) as u32);
    }
    for j in 2..=RINGS {
        let a0 = ring_start(j - 1);
        let b0 = ring_start(j);
        for i in 0..SEGS {
            let i2 = (i + 1) % SEGS;
            mesh.add_triangle((a0 + i) as u32, (b0 + i) as u32, (b0 + i2) as u32);
            mesh.add_triangle((a0 + i) as u32, (b0 + i2) as u32, (a0 + i2) as u32);
        }
    }
    p.add(Shape::mesh(mesh));
}

fn layout(p: &Painter, text: &str, size: f32) -> Arc<Galley> {
    p.ctx().fonts(|f| {
        f.layout_no_wrap(
            text.to_string(),
            FontId::new(size, clock_family()),
            Color32::WHITE,
        )
    })
}

fn layout_spaced(p: &Painter, text: &str, size: f32, spacing: f32) -> Arc<Galley> {
    let mut job = LayoutJob::default();
    job.append(
        text,
        0.0,
        TextFormat {
            font_id: FontId::new(size, clock_family()),
            color: Color32::WHITE,
            extra_letter_spacing: spacing,
            ..Default::default()
        },
    );
    p.ctx().fonts(|f| f.layout_job(job))
}

fn text_at(p: &Painter, pos: Pos2, g: &Arc<Galley>, color: Color32) {
    p.add(TextShape::new(pos, g.clone(), color).with_override_text_color(color));
}

fn hour_string(h: u32, s: &Settings) -> (String, Option<&'static str>) {
    let (hh, ap) = if s.use_24h {
        (h, None)
    } else {
        let ap = if h < 12 { "AM" } else { "PM" };
        let h12 = if h % 12 == 0 { 12 } else { h % 12 };
        (h12, Some(ap))
    };
    let st = if s.leading_zero {
        format!("{hh:02}")
    } else {
        format!("{hh}")
    };
    (st, ap)
}

fn colon_level(s: &Settings, t: &ClockTime) -> f32 {
    if !s.blink_colon {
        return 1.0;
    }
    let c = 0.5 + 0.5 * (t.frac * TAU).cos();
    0.22 + 0.78 * c.powf(0.7)
}

// ───────────────────────── Datum & Sekundenlinie ─────────────────────────

const DATE_K: f32 = 0.1;

struct Extras {
    line: bool,
    line_gap: f32,
    line_th: f32,
    date: Option<Arc<Galley>>,
    date_gap: f32,
}

impl Extras {
    fn new(p: &Painter, s: &Settings, t: &ClockTime, size: f32, allow_line: bool, max_w: f32) -> Self {
        let line = allow_line && s.seconds == SecondsMode::Line;
        let date = s.show_date.then(|| {
            let txt = t.date_text().to_uppercase();
            let ds = (size * DATE_K).clamp(11.0, 84.0).round();
            let g = layout_spaced(p, &txt, ds, ds * 0.18);
            if g.size().x > max_w && g.size().x > 0.0 {
                // zu breit (z. B. Hochformat) -> verkleinern
                let ds = (ds * max_w / g.size().x).floor().max(9.0);
                layout_spaced(p, &txt, ds, ds * 0.18)
            } else {
                g
            }
        });
        Self {
            line,
            line_gap: size * 0.085,
            line_th: (size * 0.011).clamp(2.0, 6.0),
            date,
            date_gap: if line { size * 0.12 } else { size * 0.17 },
        }
    }

    fn height(&self) -> f32 {
        let mut h = 0.0;
        if self.line {
            h += self.line_gap + self.line_th;
        }
        if let Some(g) = &self.date {
            h += self.date_gap + g.mesh_bounds.height();
        }
        h
    }

    /// Höhe in Einheiten der Schriftgröße (für die Größenberechnung).
    fn units(s: &Settings, allow_line: bool) -> f32 {
        let mut u = 0.0;
        if allow_line && s.seconds == SecondsMode::Line {
            u += 0.1;
        }
        if s.show_date {
            u += 0.17 + DATE_K * 0.8;
        }
        u
    }

    fn draw(&self, p: &Painter, pal: &Palette, t: &ClockTime, left: f32, right: f32, mut y: f32, cx: f32) {
        if self.line {
            y += self.line_gap;
            let th = self.line_th;
            let track = Rect::from_min_max(pos2(left, y), pos2(right, y + th));
            p.rect_filled(track, th * 0.5, pal.add(0.09));
            let prog = ((t.s as f32 + t.frac) / 60.0).clamp(0.0, 1.0);
            let xe = left + (right - left) * prog;
            p.rect_filled(
                Rect::from_min_max(track.min, pos2(xe.max(left + th), track.max.y)),
                th * 0.5,
                pal.col(0.45),
            );
            let head = pos2(xe, track.center().y);
            radial_glow(p, head, vec2(th * 9.0, th * 9.0), |k| {
                pal.add((0.25 + pal.glow * 0.6) * falloff(k))
            });
            p.circle_filled(head, th * 1.15, pal.col(1.0));
            y += th;
        }
        if let Some(g) = &self.date {
            y += self.date_gap;
            let spacing = g.size().x - g.mesh_bounds.width();
            let pos = pos2(
                cx - g.size().x * 0.5 + spacing * 0.5,
                y - g.mesh_bounds.min.y,
            );
            text_at(p, pos, g, pal.col(0.5));
        }
    }
}

// ───────────────────────── Öffentliche Zeichenfunktion ─────────────────────────

/// Zweizeilig (Stunden über Minuten)?
pub fn stacked(rect: Rect, s: &Settings) -> bool {
    match s.layout {
        LayoutMode::Single => false,
        LayoutMode::Stacked => true,
        LayoutMode::Auto => rect.height() > rect.width() * 1.1,
    }
}

/// Zeichnet die Uhr in `rect`, gibt das Rechteck der Uhrzeit zurück.
pub fn draw_clock(
    p: &Painter,
    rect: Rect,
    t: &ClockTime,
    s: &Settings,
    pal: &Palette,
    vf: &mut VectorFont,
) -> Rect {
    match s.style {
        ClockStyle::Text => draw_text_style(p, rect, t, s, pal, vf),
        ClockStyle::Segment => draw_segment_style(p, rect, t, s, pal),
        ClockStyle::Analog => draw_analog_style(p, rect, t, s, pal),
    }
}

fn separator_dots(p: &Painter, pal: &Palette, centers: [Pos2; 2], r: f32, level: f32, ghost: bool) {
    for c in centers {
        if pal.glow > 0.01 {
            radial_glow(p, c, vec2(r * 4.0, r * 4.0), |k| {
                pal.add(pal.glow * 0.22 * level * falloff(k))
            });
        }
        if ghost {
            p.circle_filled(c, r, pal.add(0.06));
        }
        // durchscheinend statt dunkel, damit ein Hintergrund sichtbar bleibt
        p.circle_filled(c, r, pal.col(1.0).gamma_multiply(level.clamp(0.0, 1.0)));
    }
}

// ───────────────────────── Stil: Schrift (Vektor) ─────────────────────────

struct VMetrics {
    digit: f32,
    colon: f32,
    ink_top: f32,
    ink_bottom: f32,
}

fn vmetrics(vf: &mut VectorFont) -> VMetrics {
    let mut digit: f32 = 0.0;
    let mut top = f32::MAX;
    let mut bottom = f32::MIN;
    for c in '0'..='9' {
        if let Some(g) = vf.glyph(c) {
            digit = digit.max(g.adv);
            top = top.min(g.ink.min.y);
            bottom = bottom.max(g.ink.max.y);
        }
    }
    if digit <= 0.0 {
        digit = 0.6;
    }
    if top >= bottom {
        top = -0.7;
        bottom = 0.0;
    }
    let colon = vf
        .glyph(':')
        .map(|g| g.adv)
        .unwrap_or(digit * 0.4)
        .max(digit * 0.2);
    VMetrics {
        digit,
        colon,
        ink_top: top,
        ink_bottom: bottom,
    }
}

struct GlyphItem {
    c: char,
    origin: Pos2,
    size: f32,
    color: Color32,
    glow: f32,
}

fn draw_glyph_items(p: &Painter, vf: &mut VectorFont, pal: &Palette, items: &[GlyphItem]) {
    if pal.glow > 0.01 {
        for it in items {
            if let Some(g) = vf.glyph(it.c) {
                let k = pal.glow * it.glow;
                g.draw_glow(p, it.origin, it.size, pal.eco, |w| pal.add(k * w));
            }
        }
    }
    for it in items {
        if let Some(g) = vf.glyph(it.c) {
            g.draw(p, it.origin, it.size, it.color);
        }
    }
}

/// Legt eine Zeile Zeichen in festen Zellen an (nichts springt). Gibt die rechte Kante zurück.
#[allow(clippy::too_many_arguments)]
fn glyph_row(
    vf: &mut VectorFont,
    m: &VMetrics,
    items: &mut Vec<GlyphItem>,
    text: &str,
    x0: f32,
    ink_top_y: f32,
    size: f32,
    color: Color32,
    glow: f32,
) -> f32 {
    let colon_w = m.colon * 1.25;
    let mut x = x0;
    for c in text.chars() {
        let w = if c == ':' { colon_w * size } else { m.digit * size };
        let adv = vf.glyph(c).map(|g| g.adv * size).unwrap_or(w);
        items.push(GlyphItem {
            c,
            origin: pos2(x + (w - adv) * 0.5, ink_top_y - m.ink_top * size),
            size,
            color,
            glow,
        });
        x += w;
    }
    x
}

fn draw_text_style(
    p: &Painter,
    rect: Rect,
    t: &ClockTime,
    s: &Settings,
    pal: &Palette,
    vf: &mut VectorFont,
) -> Rect {
    const SEC_K: f32 = 0.36;
    const SEC_GAP: f32 = 0.07;
    const ROW_GAP: f32 = 0.3;
    const SEC_ROW_GAP: f32 = 0.16;
    let m = vmetrics(vf);
    let (hour_s, ampm) = hour_string(t.h, s);
    let min_s = format!("{:02}", t.m);
    let sec_s = format!("{:02}", t.s);
    let sec_digits = s.seconds == SecondsMode::Digits;
    let colon_w = m.colon * 1.25;
    let ink_h = (m.ink_bottom - m.ink_top).max(0.3);
    let stack = stacked(rect, s);
    let cl = colon_level(s, t);
    let main = pal.col(1.0);
    let avail_w = rect.width() * 0.9 * s.scale;
    let avail_h = rect.height() * 0.82 * s.scale;

    // Platz für Sekunden bzw. AM/PM rechts neben der Zeile
    let side = if sec_digits {
        SEC_GAP + 2.0 * m.digit * SEC_K
    } else if ampm.is_some() {
        SEC_GAP + 0.3
    } else {
        0.0
    };
    // zweizeilig: Sekunden als kleine dritte Zeile darunter
    let sec_row = if stack && sec_digits { SEC_ROW_GAP + ink_h * SEC_K } else { 0.0 };
    let (wu, hu) = if stack {
        let ap_side = if ampm.is_some() { 2.0 * (SEC_GAP + 0.3) } else { 0.0 };
        (2.0 * m.digit + ap_side, 2.0 * ink_h + ROW_GAP + sec_row + Extras::units(s, true))
    } else {
        (4.0 * m.digit + colon_w + side, ink_h + Extras::units(s, true))
    };
    let size = (avail_w / wu).min(avail_h / hu).max(8.0).floor();
    let sec_size = (size * SEC_K).floor();

    let extras = Extras::new(p, s, t, size, true, rect.width() * 0.86);
    let time_h = if stack {
        (2.0 * ink_h + ROW_GAP + sec_row) * size
    } else {
        ink_h * size
    };
    let total_h = time_h + extras.height();
    let top = rect.center().y - total_h * 0.5;
    let cx = rect.center().x;
    let n_hour = hour_s.chars().count() as f32;

    let mut items: Vec<GlyphItem> = Vec::new();
    let (left, right, ap_pos, sec_anchor);
    if stack {
        let hl = cx - n_hour * m.digit * size * 0.5;
        let hr = glyph_row(vf, &m, &mut items, &hour_s, hl, top, size, main, 1.0);
        let my = top + (ink_h + ROW_GAP) * size;
        let ml = cx - m.digit * size;
        let mr = glyph_row(vf, &m, &mut items, &min_s, ml, my, size, main, 1.0);
        // Trennpunkte zwischen den Zeilen
        let dy = top + (ink_h + ROW_GAP * 0.5) * size;
        let dx = m.digit * size * 0.3;
        separator_dots(
            p,
            pal,
            [pos2(cx - dx, dy), pos2(cx + dx, dy)],
            (size * 0.026).max(1.5),
            cl,
            false,
        );
        left = ml;
        right = mr;
        ap_pos = pos2(hr + SEC_GAP * size * 0.6, top);
        sec_anchor = (mr, my);
    } else {
        let mut width = (n_hour + 2.0) * m.digit * size + colon_w * size;
        if sec_digits {
            width += SEC_GAP * size + 2.0 * m.digit * sec_size;
        }
        let l = cx - width * 0.5;
        let x = glyph_row(vf, &m, &mut items, &hour_s, l, top, size, main, 1.0);
        let x = glyph_row(vf, &m, &mut items, ":", x, top, size, pal.col(1.0).gamma_multiply(cl), 0.7);
        let x = glyph_row(vf, &m, &mut items, &min_s, x, top, size, main, 1.0);
        left = l;
        right = x;
        ap_pos = pos2(x + SEC_GAP * size * if sec_digits { 1.0 } else { 0.6 }, top);
        sec_anchor = (x, top);
    }
    if sec_digits {
        let (sx, sy) = sec_anchor;
        if stack {
            let sec_top = sy + (ink_h + SEC_ROW_GAP) * size;
            let sl = cx - m.digit * sec_size;
            glyph_row(vf, &m, &mut items, &sec_s, sl, sec_top, sec_size, pal.col(0.55), 0.5);
        } else {
            let sec_top = sy + ink_h * size - ink_h * sec_size;
            glyph_row(
                vf,
                &m,
                &mut items,
                &sec_s,
                sx + SEC_GAP * size,
                sec_top,
                sec_size,
                pal.col(0.55),
                0.5,
            );
        }
    }

    // Lichtschein hinter der Uhr
    if pal.glow > 0.01 {
        let c = pos2(cx, top + time_h * 0.5);
        radial_glow(
            p,
            c,
            vec2((right - left) * 0.68, time_h * 0.95 + size * 0.15),
            |k| pal.add(pal.glow * 0.085 * falloff(k)),
        );
    }
    draw_glyph_items(p, vf, pal, &items);

    if let Some(ap) = ampm {
        let ap_size = (size * 0.15).round().max(9.0);
        let g = layout(p, ap, ap_size);
        text_at(p, pos2(ap_pos.x, ap_pos.y - g.mesh_bounds.min.y), &g, pal.col(0.6));
    }

    extras.draw(p, pal, t, left, right, top + time_h, cx);
    Rect::from_min_max(pos2(left, top), pos2(right, top + total_h))
}

// ───────────────────────── Stil: 7-Segment ─────────────────────────

/// Bitmasken a..g (Bit 0 = a)
const SEG_MASK: [u8; 10] = [0x3F, 0x06, 0x5B, 0x4F, 0x66, 0x6D, 0x7D, 0x07, 0x7F, 0x6F];
const SEG_DW: f32 = 0.54; // Ziffernbreite
const SEG_TH: f32 = 0.1; // Segmentstärke
const SEG_DS: f32 = 0.15; // Abstand innerhalb eines Paares

fn segment_polys(x0: f32, y0: f32, h: f32, w: f32, t: f32, slant: f32) -> [Vec<Pos2>; 7] {
    let g = t * 0.13;
    let ht = t * 0.5;
    let yc = y0 + h * 0.5;
    let sk = |x: f32, y: f32| pos2(x + (yc - y) * slant, y);
    let hseg = |y: f32| {
        let xa = x0 + ht + g;
        let xb = x0 + w - ht - g;
        vec![
            sk(xa, y),
            sk(xa + ht, y - ht),
            sk(xb - ht, y - ht),
            sk(xb, y),
            sk(xb - ht, y + ht),
            sk(xa + ht, y + ht),
        ]
    };
    let vseg = |x: f32, ya: f32, yb: f32| {
        vec![
            sk(x, ya),
            sk(x + ht, ya + ht),
            sk(x + ht, yb - ht),
            sk(x, yb),
            sk(x - ht, yb - ht),
            sk(x - ht, ya + ht),
        ]
    };
    let xl = x0 + ht;
    let xr = x0 + w - ht;
    let top = y0 + ht;
    let bot = y0 + h - ht;
    [
        hseg(top),
        vseg(xr, top + g, yc - g),
        vseg(xr, yc + g, bot - g),
        hseg(bot),
        vseg(xl, yc + g, bot - g),
        vseg(xl, top + g, yc - g),
        hseg(yc),
    ]
}

struct SegDigit {
    polys: [Vec<Pos2>; 7],
    mask: u8,
    th: f32,
    level: f32,
}

/// Zwei Ziffern nebeneinander; gibt die rechte Kante zurück.
fn seg_pair(
    out: &mut Vec<SegDigit>,
    x: f32,
    y: f32,
    h: f32,
    d: [Option<u8>; 2],
    slant: f32,
    level: f32,
) -> f32 {
    let dw = SEG_DW * h;
    let mut x = x;
    for (i, d) in d.iter().enumerate() {
        out.push(SegDigit {
            polys: segment_polys(x, y, h, dw, SEG_TH * h, slant),
            mask: d.map(|d| SEG_MASK[d as usize % 10]).unwrap_or(0),
            th: SEG_TH * h,
            level,
        });
        x += dw + if i == 0 { SEG_DS * h } else { 0.0 };
    }
    x
}

fn split_digits(n: u32) -> [Option<u8>; 2] {
    [Some((n / 10 % 10) as u8), Some((n % 10) as u8)]
}

fn draw_segment_style(p: &Painter, rect: Rect, t: &ClockTime, s: &Settings, pal: &Palette) -> Rect {
    const CW: f32 = 0.36; // Doppelpunkt-Zelle
    const SEC_K: f32 = 0.42;
    const SEC_GAP: f32 = 0.17;
    const ROW_GAP: f32 = 0.26;
    let slant = if s.slanted_segments { 0.085 } else { 0.0 };
    let sec_digits = s.seconds == SecondsMode::Digits;
    let stack = stacked(rect, s);
    let (hour_s, ampm) = hour_string(t.h, s);
    let pair_w = 2.0 * SEG_DW + SEG_DS;
    let side = if sec_digits {
        SEC_GAP + SEC_K * pair_w
    } else if ampm.is_some() {
        0.32
    } else {
        0.0
    };
    const SEC_ROW_GAP: f32 = 0.18;
    let sec_row = if stack && sec_digits { SEC_ROW_GAP + SEC_K } else { 0.0 };
    let (wu, hu) = if stack {
        let ap_side = if ampm.is_some() { 0.64 } else { 0.0 };
        (pair_w + slant + ap_side, 2.0 + ROW_GAP + sec_row + Extras::units(s, true))
    } else {
        (2.0 * pair_w + CW + slant + side, 1.0 + Extras::units(s, true))
    };
    let avail_w = rect.width() * 0.88 * s.scale;
    let avail_h = rect.height() * 0.8 * s.scale;
    let h = ((avail_w / wu).min(avail_h / hu)).max(10.0).floor();

    let extras = Extras::new(p, s, t, h, true, rect.width() * 0.86);
    let time_h = if stack { (2.0 + ROW_GAP + sec_row) * h } else { h };
    let total_h = time_h + extras.height();
    let top = rect.center().y - total_h * 0.5;
    let cx = rect.center().x;

    let hour_digits: [Option<u8>; 2] = if hour_s.len() == 1 {
        [None, hour_s.parse::<u8>().ok()]
    } else {
        let v: Vec<Option<u8>> = hour_s
            .chars()
            .map(|c| c.to_digit(10).map(|d| d as u8))
            .collect();
        [v[0], v[1]]
    };

    let mut digits: Vec<SegDigit> = Vec::new();
    let (left, right, colon_centers, last_row_y, ap_x);
    if stack {
        let l = cx - pair_w * h * 0.5;
        let hr = seg_pair(&mut digits, l, top, h, hour_digits, slant, 1.0);
        let my = top + (1.0 + ROW_GAP) * h;
        let mr = seg_pair(&mut digits, l, my, h, split_digits(t.m), slant, 1.0);
        let dy = top + (1.0 + ROW_GAP * 0.5) * h;
        let dx = SEG_DW * h * 0.32;
        colon_centers = [pos2(cx - dx, dy), pos2(cx + dx, dy)];
        left = l;
        right = mr;
        last_row_y = my;
        ap_x = hr + 0.1 * h;
    } else {
        let mut width = (2.0 * pair_w + CW) * h;
        if sec_digits {
            width += (SEC_GAP + SEC_K * pair_w) * h;
        }
        let l = cx - width * 0.5;
        let x = seg_pair(&mut digits, l, top, h, hour_digits, slant, 1.0);
        let colon_x = x + CW * h * 0.5;
        let x = seg_pair(&mut digits, x + CW * h, top, h, split_digits(t.m), slant, 1.0);
        let yc = top + h * 0.5;
        let c = |fy: f32| {
            let y = top + h * fy;
            pos2(colon_x + (yc - y) * slant, y)
        };
        colon_centers = [c(0.31), c(0.69)];
        left = l;
        right = x;
        last_row_y = top;
        ap_x = x + 0.1 * h;
    }
    if sec_digits && stack {
        let sh = SEC_K * h;
        let sy = last_row_y + h + SEC_ROW_GAP * h;
        let sl = cx - pair_w * sh * 0.5;
        seg_pair(&mut digits, sl, sy, sh, split_digits(t.s), slant, 0.6);
    } else if sec_digits {
        let sh = SEC_K * h;
        let sy = last_row_y + h - sh;
        // gleiche Neigungslinie wie die großen Ziffern
        let shift = (last_row_y + h * 0.5 - (sy + sh * 0.5)) * slant;
        seg_pair(
            &mut digits,
            right + SEC_GAP * h + shift,
            sy,
            sh,
            split_digits(t.s),
            slant,
            0.6,
        );
    }

    if pal.glow > 0.01 {
        radial_glow(
            p,
            pos2(cx, top + time_h * 0.5),
            vec2((right - left) * 0.68, time_h * 0.9 + h * 0.2),
            |k| pal.add(pal.glow * 0.085 * falloff(k)),
        );
    }

    // 1) inaktive Segmente
    if s.ghost_segments {
        let ghost = pal.add(0.06);
        for d in &digits {
            for (i, poly) in d.polys.iter().enumerate() {
                if d.mask & (1 << i) == 0 {
                    p.add(Shape::convex_polygon(poly.clone(), ghost, Stroke::NONE));
                }
            }
        }
    }
    // 2) Leuchten: leicht versetzte, additive Kopien der aktiven Segmente
    if pal.glow > 0.01 {
        let (rings, n): (&[(f32, f32)], usize) = if pal.eco {
            (&[(0.3, 0.6), (0.8, 0.4)], 6)
        } else {
            (&[(0.22, 0.5), (0.5, 0.3), (0.9, 0.2)], 8)
        };
        let per = 10.0 / n as f32;
        for d in &digits {
            for (ri, &(r, w)) in rings.iter().enumerate() {
                let col = pal.add(pal.glow * d.level * w * 0.15 * per);
                for k in 0..n {
                    let a = (k as f32 + 0.5 * ri as f32) / n as f32 * TAU;
                    let off = vec2(a.cos(), a.sin()) * (r * d.th).max(1.0);
                    for (i, poly) in d.polys.iter().enumerate() {
                        if d.mask & (1 << i) != 0 {
                            let pts = poly.iter().map(|q| *q + off).collect();
                            p.add(Shape::convex_polygon(pts, col, Stroke::NONE));
                        }
                    }
                }
            }
        }
    }
    // 3) aktive Segmente
    for d in &digits {
        let col = pal.col(d.level);
        for (i, poly) in d.polys.iter().enumerate() {
            if d.mask & (1 << i) != 0 {
                p.add(Shape::convex_polygon(poly.clone(), col, Stroke::NONE));
            }
        }
    }

    separator_dots(
        p,
        pal,
        colon_centers,
        SEG_TH * h * 0.6,
        colon_level(s, t),
        s.ghost_segments,
    );

    if let Some(ap) = ampm {
        let size = (h * 0.15).round().max(9.0);
        let g = layout(p, ap, size);
        text_at(p, pos2(ap_x, top - g.mesh_bounds.min.y), &g, pal.col(0.6));
    }

    extras.draw(p, pal, t, left, right, top + time_h, cx);
    Rect::from_min_max(pos2(left, top), pos2(right, top + total_h))
}

// ───────────────────────── Stil: Analog ─────────────────────────

fn hand(p: &Painter, c: Pos2, angle: f32, back: f32, len: f32, w0: f32, w1: f32, col: Color32, glow: Color32) {
    let dir = vec2(angle.sin(), -angle.cos());
    let nrm = vec2(-dir.y, dir.x);
    let a = c - dir * back;
    let b = c + dir * len;
    if glow != Color32::TRANSPARENT {
        p.line_segment([a, b], Stroke::new(w0 * 3.2, glow));
        p.line_segment([a, b], Stroke::new(w0 * 1.8, glow));
    }
    let pts = vec![
        a - nrm * (w0 * 0.5),
        b - nrm * (w1 * 0.5),
        b + nrm * (w1 * 0.5),
        a + nrm * (w0 * 0.5),
    ];
    p.add(Shape::convex_polygon(pts, col, Stroke::NONE));
    p.circle_filled(b, w1 * 0.5, col);
    p.circle_filled(a, w0 * 0.5, col);
}

fn draw_analog_style(p: &Painter, rect: Rect, t: &ClockTime, s: &Settings, pal: &Palette) -> Rect {
    let r = (rect.width().min(rect.height()) * 0.45 * s.scale).max(30.0);
    let c = rect.center();

    if pal.glow > 0.01 {
        radial_glow(p, c, vec2(r * 1.35, r * 1.35), |k| pal.add(pal.glow * 0.07 * falloff(k)));
    }

    // Zifferblatt
    p.circle_stroke(c, r, Stroke::new((r * 0.005).max(1.0), pal.col(0.13)));
    for i in 0..60 {
        let a = i as f32 / 60.0 * TAU;
        let dir = vec2(a.sin(), -a.cos());
        if i % 5 == 0 {
            let major = i % 15 == 0;
            let len = if major { 0.12 } else { 0.075 } * r;
            let w = if major { 0.024 } else { 0.014 } * r;
            let p2 = c + dir * (r * 0.92);
            let p1 = c + dir * (r * 0.92 - len);
            let col = pal.col(if major { 0.95 } else { 0.65 });
            if pal.glow > 0.01 {
                p.line_segment([p1, p2], Stroke::new(w * 3.0, pal.add(pal.glow * 0.06)));
            }
            p.line_segment([p1, p2], Stroke::new(w, col));
            p.circle_filled(p1, w * 0.5, col);
            p.circle_filled(p2, w * 0.5, col);
        } else {
            p.circle_filled(c + dir * (r * 0.905), (r * 0.0065).max(0.8), pal.col(0.3));
        }
    }

    // Datum im unteren Teil des Zifferblatts
    if s.show_date {
        let size = (r * 0.07).clamp(10.0, 60.0).round();
        let txt = t.short_date_text().to_uppercase();
        let g = layout_spaced(p, &txt, size, size * 0.16);
        let spacing = g.size().x - g.mesh_bounds.width();
        let pos = pos2(
            c.x - g.size().x * 0.5 + spacing * 0.5,
            c.y + r * 0.42 - g.mesh_bounds.center().y,
        );
        text_at(p, pos, &g, pal.col(0.5));
    }
    if !s.use_24h {
        let size = (r * 0.06).clamp(9.0, 50.0).round();
        let g = layout(p, if t.h < 12 { "AM" } else { "PM" }, size);
        let pos = pos2(c.x - g.size().x * 0.5, c.y - r * 0.4 - g.mesh_bounds.center().y);
        text_at(p, pos, &g, pal.col(0.45));
    }

    // Zeiger
    let sec = t.s as f32 + t.frac;
    let min = t.m as f32 + sec / 60.0;
    let hr = (t.h % 12) as f32 + min / 60.0;
    let glow_col = if pal.glow > 0.01 {
        pal.add(pal.glow * 0.07)
    } else {
        Color32::TRANSPARENT
    };
    let main = pal.col(1.0);
    hand(p, c, hr / 12.0 * TAU, r * 0.05, r * 0.5, r * 0.055, r * 0.036, main, glow_col);
    hand(p, c, min / 60.0 * TAU, r * 0.06, r * 0.8, r * 0.04, r * 0.022, main, glow_col);
    if s.seconds != SecondsMode::Off {
        let a = sec / 60.0 * TAU;
        let dir = vec2(a.sin(), -a.cos());
        let acc = pal.accent(1.0);
        p.line_segment([c - dir * (r * 0.2), c + dir * (r * 0.88)], Stroke::new((r * 0.009).max(1.0), acc));
        p.circle_filled(c - dir * (r * 0.16), r * 0.026, acc);
        p.circle_filled(c, r * 0.042, acc);
    } else {
        p.circle_filled(c, r * 0.042, main);
    }
    p.circle_filled(c, r * 0.016, Color32::from_rgb(
        (s.background[0] as f32 * pal.level) as u8,
        (s.background[1] as f32 * pal.level) as u8,
        (s.background[2] as f32 * pal.level) as u8,
    ));

    Rect::from_center_size(c, vec2(r * 2.0, r * 2.0))
}

//! Oberfläche: Stil, Einstellungsmenü, Schaltflächen, Wecker-Overlay, Hinweise.

use crate::app::NightClockApp;
use crate::fonts::ui_bold_family;
use crate::i18n::t;
use crate::ambient::AmbientParams;
use crate::settings::{Alarm, AlarmSound, AmbientMode, ClockStyle, Language, LayoutMode, SecondsMode};
use egui::{
    pos2, vec2, Align, Align2, Area, Button, Color32, ComboBox, Context, CornerRadius, DragValue,
    FontFamily, FontId, Frame, Id, Layout, Margin, Order, Painter, PopupCloseBehavior, Pos2, Rect,
    Response, RichText, ScrollArea, Sense, Shadow, Shape, Slider, Stroke, TextEdit, TextStyle, Ui,
    Visuals,
};
use std::f32::consts::TAU;
use std::ops::RangeInclusive;

pub const PRESETS: [([u8; 3], &str, &str); 8] = [
    ([255, 179, 92], "Bernstein", "Amber"),
    ([255, 92, 66], "Glut", "Ember"),
    ([255, 214, 150], "Kerzenlicht", "Candlelight"),
    ([214, 226, 255], "Mondlicht", "Moonlight"),
    ([110, 204, 255], "Eisblau", "Ice blue"),
    ([96, 240, 170], "Mint", "Mint"),
    ([184, 156, 255], "Lavendel", "Lavender"),
    ([255, 138, 186], "Rosé", "Rosé"),
];

fn accent(c: [u8; 3]) -> Color32 {
    Color32::from_rgb(c[0], c[1], c[2])
}

fn on_accent(c: [u8; 3]) -> Color32 {
    let lum = 0.2126 * c[0] as f32 + 0.7152 * c[1] as f32 + 0.0722 * c[2] as f32;
    if lum > 140.0 {
        Color32::from_rgb(18, 18, 22)
    } else {
        Color32::from_gray(250)
    }
}

fn lerp_col(a: Color32, b: Color32, t: f32) -> Color32 {
    let t = t.clamp(0.0, 1.0);
    let l = |x: u8, y: u8| (x as f32 + (y as f32 - x as f32) * t).round() as u8;
    Color32::from_rgba_premultiplied(l(a.r(), b.r()), l(a.g(), b.g()), l(a.b(), b.b()), l(a.a(), b.a()))
}

/// Dunkles, ruhiges Design mit der Uhrfarbe als Akzent.
pub fn apply_style(ctx: &Context, color: [u8; 3]) {
    let acc = accent(color);
    let mut style = (*ctx.style()).clone();
    let mut v = Visuals::dark();
    v.panel_fill = Color32::from_rgb(14, 15, 19);
    v.window_fill = Color32::from_rgb(20, 21, 27);
    v.extreme_bg_color = Color32::from_rgb(9, 10, 13);
    v.faint_bg_color = Color32::from_rgb(24, 25, 31);
    v.selection.bg_fill = acc.gamma_multiply(0.9);
    v.selection.stroke = Stroke::new(1.0_f32, on_accent(color));
    v.hyperlink_color = acc;
    v.slider_trailing_fill = true;
    v.window_corner_radius = CornerRadius::same(14);
    v.menu_corner_radius = CornerRadius::same(12);
    v.window_stroke = Stroke::new(1.0_f32, Color32::from_white_alpha(16));
    v.popup_shadow = Shadow {
        offset: [0, 6],
        blur: 24,
        spread: 0,
        color: Color32::from_black_alpha(150),
    };
    v.window_shadow = v.popup_shadow;
    let cr = CornerRadius::same(8);
    v.widgets.noninteractive.corner_radius = cr;
    v.widgets.noninteractive.bg_stroke = Stroke::new(1.0_f32, Color32::from_white_alpha(14));
    v.widgets.noninteractive.fg_stroke = Stroke::new(1.0_f32, Color32::from_gray(205));
    v.widgets.inactive.corner_radius = cr;
    v.widgets.inactive.bg_fill = Color32::from_rgb(38, 40, 48);
    v.widgets.inactive.weak_bg_fill = Color32::from_rgb(34, 36, 44);
    v.widgets.inactive.bg_stroke = Stroke::NONE;
    v.widgets.inactive.fg_stroke = Stroke::new(1.0_f32, Color32::from_gray(215));
    v.widgets.hovered.corner_radius = cr;
    v.widgets.hovered.bg_fill = Color32::from_rgb(50, 52, 62);
    v.widgets.hovered.weak_bg_fill = Color32::from_rgb(46, 48, 58);
    v.widgets.hovered.bg_stroke = Stroke::new(1.0_f32, acc.gamma_multiply(0.5));
    v.widgets.hovered.fg_stroke = Stroke::new(1.5_f32, Color32::from_gray(240));
    v.widgets.active.corner_radius = cr;
    v.widgets.active.bg_fill = acc;
    v.widgets.active.weak_bg_fill = acc.gamma_multiply(0.6);
    v.widgets.active.bg_stroke = Stroke::new(1.0_f32, acc);
    v.widgets.active.fg_stroke = Stroke::new(2.0_f32, Color32::WHITE);
    v.widgets.open.corner_radius = cr;
    v.widgets.open.weak_bg_fill = Color32::from_rgb(46, 48, 58);
    style.visuals = v;

    style.spacing.item_spacing = vec2(8.0, 7.0);
    style.spacing.button_padding = vec2(12.0, 5.0);
    style.spacing.interact_size.y = 26.0;
    style.spacing.slider_rail_height = 5.0;
    style.spacing.scroll.bar_width = 6.0;
    style.spacing.scroll.floating = true;

    use FontFamily::Proportional;
    style.text_styles = [
        (TextStyle::Heading, FontId::new(21.0, Proportional)),
        (TextStyle::Body, FontId::new(14.5, Proportional)),
        (TextStyle::Button, FontId::new(14.5, Proportional)),
        (TextStyle::Small, FontId::new(12.0, Proportional)),
        (TextStyle::Monospace, FontId::new(13.5, FontFamily::Monospace)),
    ]
    .into();
    ctx.set_style(style);
}

// ───────────────────────── Icons ─────────────────────────

pub fn bell_icon(p: &Painter, c: Pos2, s: f32, col: Color32) {
    let u = |x: f32, y: f32| c + vec2(x * s, y * s);
    p.circle_filled(u(0.0, -0.1), s * 0.3, col);
    p.add(Shape::convex_polygon(
        vec![u(-0.3, -0.1), u(0.3, -0.1), u(0.43, 0.27), u(-0.43, 0.27)],
        col,
        Stroke::NONE,
    ));
    p.rect_filled(Rect::from_min_max(u(-0.5, 0.22), u(0.5, 0.33)), s * 0.05, col);
    p.circle_filled(u(0.0, 0.43), s * 0.09, col);
    p.circle_filled(u(0.0, -0.44), s * 0.07, col);
}

fn gear_icon(p: &Painter, c: Pos2, s: f32, col: Color32) {
    let teeth = 8;
    let step = TAU / teeth as f32;
    let (ro, ri) = (s * 0.40, s * 0.29);
    let mut pts = Vec::new();
    for i in 0..teeth {
        let a0 = i as f32 * step;
        for (da, r) in [(-0.16, ro), (0.16, ro), (0.30, ri), (0.70, ri)] {
            let a = a0 + da * step;
            pts.push(c + vec2(a.cos(), a.sin()) * r);
        }
    }
    p.add(Shape::closed_line(pts, Stroke::new(s * 0.075, col)));
    p.circle_stroke(c, s * 0.12, Stroke::new(s * 0.075, col));
}

fn fullscreen_icon(p: &Painter, c: Pos2, s: f32, col: Color32, compress: bool) {
    let stroke = Stroke::new(s * 0.075, col);
    let l = s * 0.15;
    for (sx, sy) in [(-1.0, -1.0), (1.0, -1.0), (1.0, 1.0), (-1.0, 1.0)] {
        let (q, p1, p2) = if compress {
            let b = s * 0.1;
            let q = c + vec2(sx * b, sy * b);
            (q, q + vec2(sx * l, 0.0), q + vec2(0.0, sy * l))
        } else {
            let a = s * 0.3;
            let q = c + vec2(sx * a, sy * a);
            (q, q - vec2(sx * l, 0.0), q - vec2(0.0, sy * l))
        };
        p.add(Shape::line(vec![p1, q, p2], stroke));
    }
}

fn close_icon(p: &Painter, c: Pos2, s: f32, col: Color32) {
    let d = s * 0.2;
    let st = Stroke::new(s * 0.07, col);
    p.line_segment([c + vec2(-d, -d), c + vec2(d, d)], st);
    p.line_segment([c + vec2(-d, d), c + vec2(d, -d)], st);
}

fn round_button(ui: &mut Ui, size: f32, draw: impl FnOnce(&Painter, Pos2, Color32)) -> Response {
    let (rect, resp) = ui.allocate_exact_size(vec2(size, size), Sense::click());
    let hov = ui.ctx().animate_bool_with_time(resp.id, resp.hovered(), 0.12);
    let p = ui.painter();
    p.circle_filled(
        rect.center(),
        size * 0.5,
        Color32::from_white_alpha((12.0 + 22.0 * hov) as u8),
    );
    p.circle_stroke(
        rect.center(),
        size * 0.5 - 0.5,
        Stroke::new(1.0_f32, Color32::from_white_alpha(18)),
    );
    draw(p, rect.center(), Color32::from_gray((165.0 + 75.0 * hov) as u8));
    resp
}

// ───────────────────────── Widgets ─────────────────────────

fn toggle(ui: &mut Ui, on: &mut bool, acc: Color32) -> Response {
    let (rect, mut resp) = ui.allocate_exact_size(vec2(40.0, 22.0), Sense::click());
    if resp.clicked() {
        *on = !*on;
        resp.mark_changed();
    }
    let t = ui.ctx().animate_bool_with_time(resp.id, *on, 0.15);
    let p = ui.painter();
    p.rect_filled(rect, 11.0, lerp_col(Color32::from_rgb(52, 54, 64), acc, t));
    let x = egui::lerp((rect.left() + 11.0)..=(rect.right() - 11.0), t);
    let knob = if resp.hovered() { 255 } else { 238 };
    p.circle_filled(pos2(x, rect.center().y + 0.5), 8.5, Color32::from_black_alpha(60));
    p.circle_filled(pos2(x, rect.center().y), 8.0, Color32::from_gray(knob));
    resp
}

fn toggle_row(ui: &mut Ui, label: &str, on: &mut bool, acc: Color32) -> Response {
    ui.horizontal(|ui| {
        ui.label(label);
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| toggle(ui, on, acc))
            .inner
    })
    .inner
}

fn weak(ui: &mut Ui, text: &str) {
    ui.label(RichText::new(text).size(12.5).color(Color32::from_gray(140)));
}

fn slider_row<N: egui::emath::Numeric>(
    ui: &mut Ui,
    label: &str,
    v: &mut N,
    range: RangeInclusive<N>,
    fmt: impl Fn(N) -> String,
) -> Response {
    ui.horizontal(|ui| {
        ui.label(label);
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            ui.label(RichText::new(fmt(*v)).color(Color32::from_gray(150)));
        });
    });
    ui.scope(|ui| {
        ui.spacing_mut().slider_width = ui.available_width();
        ui.add(Slider::new(v, range).show_value(false))
    })
    .inner
}

fn pct(v: f32) -> String {
    format!("{:.0} %", (v * 100.0).round())
}

fn segmented<T: PartialEq + Copy>(
    ui: &mut Ui,
    id: &str,
    value: &mut T,
    options: &[(T, &str)],
    color: [u8; 3],
) {
    let acc = accent(color);
    let on = on_accent(color);
    let w = ui.available_width();
    let h = 32.0;
    let (rect, _) = ui.allocate_exact_size(vec2(w, h), Sense::hover());
    let n = options.len() as f32;
    let cw = w / n;
    let sel = options.iter().position(|(v, _)| v == value).unwrap_or(0) as f32;
    let ax = ui
        .ctx()
        .animate_value_with_time(ui.id().with(("seg_anim", id)), sel, 0.18);
    let mut clicked = None;
    {
        let p = ui.painter();
        p.rect_filled(rect, 10.0, Color32::from_rgb(30, 32, 39));
        let hl = Rect::from_min_size(pos2(rect.left() + ax * cw, rect.top()), vec2(cw, h)).shrink(3.0);
        p.rect_filled(hl, 8.0, acc);
    }
    for (i, (v, label)) in options.iter().enumerate() {
        let r = Rect::from_min_size(pos2(rect.left() + i as f32 * cw, rect.top()), vec2(cw, h));
        let resp = ui.interact(r, ui.id().with(("seg", id, i)), Sense::click());
        if resp.clicked() {
            clicked = Some(*v);
        }
        let overlap = (1.0 - (ax - i as f32).abs()).clamp(0.0, 1.0);
        let base = if resp.hovered() {
            Color32::from_gray(240)
        } else {
            Color32::from_gray(185)
        };
        ui.painter().text(
            r.center(),
            Align2::CENTER_CENTER,
            *label,
            FontId::proportional(14.0),
            lerp_col(base, on, overlap),
        );
    }
    if let Some(v) = clicked {
        *value = v;
    }
}

fn time_edit(ui: &mut Ui, h: &mut u32, m: &mut u32, size: f32) -> bool {
    let mut changed = false;
    ui.scope(|ui| {
        ui.spacing_mut().item_spacing.x = 3.0;
        ui.spacing_mut().button_padding = vec2(6.0, 2.0);
        ui.style_mut().override_font_id = Some(FontId::proportional(size));
        changed |= ui
            .add(
                DragValue::new(h)
                    .range(0..=23)
                    .speed(0.06)
                    .custom_formatter(|n, _| format!("{:02}", n as i64)),
            )
            .on_hover_text(t("Ziehen oder klicken und tippen", "Drag, or click and type"))
            .changed();
        ui.label(":");
        changed |= ui
            .add(
                DragValue::new(m)
                    .range(0..=59)
                    .speed(0.12)
                    .custom_formatter(|n, _| format!("{:02}", n as i64)),
            )
            .on_hover_text(t("Ziehen oder klicken und tippen", "Drag, or click and type"))
            .changed();
    });
    changed
}

fn minutes_edit(ui: &mut Ui, v: &mut u32) {
    let mut h = *v / 60;
    let mut m = *v % 60;
    if time_edit(ui, &mut h, &mut m, 15.0) {
        *v = h * 60 + m;
    }
}

fn card<R>(ui: &mut Ui, title: &str, acc: Color32, add: impl FnOnce(&mut Ui) -> R) -> R {
    ui.add_space(14.0);
    ui.label(
        RichText::new(title.to_uppercase())
            .family(ui_bold_family())
            .size(11.5)
            .color(acc)
            .extra_letter_spacing(1.4),
    );
    ui.add_space(2.0);
    Frame::new()
        .fill(Color32::from_rgb(23, 24, 30))
        .stroke(Stroke::new(1.0_f32, Color32::from_white_alpha(8)))
        .corner_radius(12)
        .inner_margin(Margin::same(12))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            add(ui)
        })
        .inner
}

fn kbd(ui: &mut Ui, text: &str) {
    Frame::new()
        .fill(Color32::from_rgb(40, 42, 50))
        .corner_radius(5)
        .inner_margin(Margin::symmetric(7, 2))
        .show(ui, |ui| {
            ui.label(RichText::new(text).size(12.5).color(Color32::from_gray(220)));
        });
}

// ───────────────────────── Schwebende Schaltflächen ─────────────────────────

pub fn hover_controls(app: &mut NightClockApp, ctx: &Context) {
    let active = (app.last_activity.elapsed().as_secs_f32() < 2.5 || app.settings_open)
        && app.ringing.is_none();
    let vis = ctx.animate_bool_with_time(Id::new("hover_ctl"), active, 0.3);
    if vis < 0.01 {
        return;
    }
    let panel = ctx.animate_bool_with_time(Id::new("settings_anim"), app.settings_open, 0.22);
    let screen = ctx.screen_rect();
    let size = 38.0;
    let gap = 10.0;
    let margin = 16.0;
    let panel_w = 392.0f32.min(screen.width() - 24.0);
    let right = screen.right() - margin - panel * (panel_w + 12.0);
    let left = right - (2.0 * size + gap);

    let mut toggle_fs = false;
    let mut toggle_settings = false;
    let is_fs = app.is_fullscreen;
    Area::new(Id::new("hover_controls"))
        .order(Order::Middle)
        .fixed_pos(pos2(left, screen.top() + margin))
        .constrain(false)
        .show(ctx, |ui| {
            ui.set_opacity(vis);
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = gap;
                if round_button(ui, size, |p, c, col| fullscreen_icon(p, c, size, col, is_fs))
                    .on_hover_text(if is_fs { t("Vollbild beenden (Esc)", "Exit fullscreen (Esc)") } else { t("Vollbild (F11)", "Fullscreen (F11)") })
                    .clicked()
                {
                    toggle_fs = true;
                }
                if round_button(ui, size, |p, c, col| gear_icon(p, c, size, col))
                    .on_hover_text(t("Einstellungen (S)", "Settings (S)"))
                    .clicked()
                {
                    toggle_settings = true;
                }
            });
        });
    if toggle_fs {
        app.toggle_fullscreen(ctx);
    }
    if toggle_settings {
        app.settings_open = !app.settings_open;
    }
}

// ───────────────────────── Hinweis-Einblendung ─────────────────────────

pub fn toast(app: &mut NightClockApp, ctx: &Context) {
    let Some((text, t0)) = app.toast.clone() else {
        return;
    };
    const DUR: f32 = 2.6;
    let el = t0.elapsed().as_secs_f32();
    if el > DUR + 0.4 {
        app.toast = None;
        return;
    }
    let a = (el / 0.2).min(1.0) * ((DUR + 0.4 - el) / 0.4).clamp(0.0, 1.0);
    let screen = ctx.screen_rect();
    Area::new(Id::new("toast"))
        .order(Order::Tooltip)
        .interactable(false)
        .anchor(
            Align2::CENTER_BOTTOM,
            vec2(0.0, -screen.height() * 0.14 + (1.0 - a) * 10.0),
        )
        .show(ctx, |ui| {
            ui.set_opacity(a);
            Frame::new()
                .fill(Color32::from_rgba_unmultiplied(22, 23, 29, 240))
                .stroke(Stroke::new(1.0_f32, Color32::from_white_alpha(16)))
                .corner_radius(20)
                .inner_margin(Margin::symmetric(18, 9))
                .show(ui, |ui| {
                    ui.label(RichText::new(text).size(15.0).color(Color32::from_gray(215)));
                });
        });
    ctx.request_repaint();
}

// ───────────────────────── Wecker klingelt ─────────────────────────

pub fn ringing_overlay(app: &mut NightClockApp, ctx: &Context) {
    let Some(r) = &app.ringing else {
        return;
    };
    let (h, m) = (r.hour, r.minute);
    let secs = r.started.elapsed().as_secs_f32();
    let acc = accent(app.s.color);
    let on = on_accent(app.s.color);
    let screen = ctx.screen_rect();
    let mut snooze = false;
    let mut off = false;
    let snooze_min = app.s.snooze_minutes;

    Area::new(Id::new("ringing"))
        .order(Order::Foreground)
        .anchor(Align2::CENTER_BOTTOM, vec2(0.0, -screen.height() * 0.07))
        .show(ctx, |ui| {
            let pulse = 0.5 + 0.5 * (secs * TAU / 1.8).sin();
            Frame::new()
                .fill(Color32::from_rgba_unmultiplied(16, 17, 22, 236))
                .stroke(Stroke::new(1.5_f32, acc.gamma_multiply(0.25 + 0.45 * pulse)))
                .corner_radius(24)
                .shadow(Shadow {
                    offset: [0, 10],
                    blur: 40,
                    spread: 0,
                    color: Color32::from_black_alpha(160),
                })
                .inner_margin(Margin::symmetric(26, 20))
                .show(ui, |ui| {
                    let w = 410.0f32.min(screen.width() - 80.0);
                    ui.set_width(w);
                    ui.with_layout(Layout::top_down(Align::Center), |ui| {
                        let (rect, _) = ui.allocate_exact_size(vec2(w, 30.0), Sense::hover());
                        let text = format!("{}  ·  {h:02}:{m:02}", t("Wecker", "Alarm"));
                        let g = ui.painter().layout_no_wrap(
                            text,
                            FontId::new(20.0, ui_bold_family()),
                            Color32::from_gray(235),
                        );
                        let total = 26.0 + 10.0 + g.size().x;
                        let x0 = rect.center().x - total * 0.5;
                        let swing = (secs * TAU * 1.6).sin() * 0.06;
                        bell_icon(ui.painter(), pos2(x0 + 13.0 + swing * 26.0, rect.center().y), 24.0, acc);
                        ui.painter().galley(
                            pos2(x0 + 36.0, rect.center().y - g.size().y * 0.5),
                            g,
                            Color32::from_gray(235),
                        );
                        ui.add_space(10.0);
                        ui.horizontal(|ui| {
                            ui.spacing_mut().item_spacing.x = 10.0;
                            let bw = (w - 10.0) * 0.6;
                            let b1 = Button::new(
                                RichText::new(format!("{}  ·  {snooze_min} min", t("Schlummern", "Snooze")))
                                    .size(17.0)
                                    .color(on),
                            )
                            .fill(acc)
                            .corner_radius(14)
                            .min_size(vec2(bw, 50.0));
                            if ui.add(b1).clicked() {
                                snooze = true;
                            }
                            let b2 = Button::new(RichText::new(t("Ausschalten", "Turn off")).size(17.0))
                                .fill(Color32::from_rgb(40, 42, 50))
                                .corner_radius(14)
                                .min_size(vec2(w - 10.0 - bw, 50.0));
                            if ui.add(b2).clicked() {
                                off = true;
                            }
                        });
                        ui.add_space(4.0);
                        ui.label(
                            RichText::new(t("Leertaste = Schlummern   ·   Enter = Aus", "Space = Snooze   ·   Enter = Off"))
                                .size(12.0)
                                .color(Color32::from_gray(125)),
                        );
                    });
                });
        });
    if snooze {
        app.snooze();
    }
    if off {
        app.dismiss();
    }
}

// ───────────────────────── Einstellungen ─────────────────────────

pub fn settings_panel(app: &mut NightClockApp, ctx: &Context) {
    let anim = ctx.animate_bool_with_time(Id::new("settings_anim"), app.settings_open, 0.22);
    if anim <= 0.001 {
        return;
    }
    let screen = ctx.screen_rect();
    let margin = 12.0;
    let w = 392.0f32.min(screen.width() - 2.0 * margin);
    let eased = 1.0 - (1.0 - anim).powi(3);
    let x = screen.right() - (w + margin) * eased;
    let h = screen.height() - 2.0 * margin;
    let acc = accent(app.s.color);

    Area::new(Id::new("settings_area"))
        .order(Order::Foreground)
        .fixed_pos(pos2(x, screen.top() + margin))
        .constrain(false)
        .movable(false)
        .show(ctx, |ui| {
            ui.set_opacity(anim);
            Frame::new()
                .fill(Color32::from_rgba_unmultiplied(14, 15, 20, 246))
                .stroke(Stroke::new(1.0_f32, Color32::from_white_alpha(14)))
                .corner_radius(18)
                .shadow(Shadow {
                    offset: [0, 8],
                    blur: 36,
                    spread: 0,
                    color: Color32::from_black_alpha(150),
                })
                .inner_margin(Margin::symmetric(18, 14))
                .show(ui, |ui| {
                    ui.set_width(w - 36.0);
                    ui.set_height(h - 28.0);
                    ui.horizontal(|ui| {
                        ui.label(
                            RichText::new(t("Einstellungen", "Settings"))
                                .family(ui_bold_family())
                                .size(21.0)
                                .color(Color32::from_gray(238)),
                        );
                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            if round_button(ui, 30.0, |p, c, col| close_icon(p, c, 30.0, col))
                                .on_hover_text(t("Schließen (Esc)", "Close (Esc)"))
                                .clicked()
                            {
                                app.settings_open = false;
                            }
                            ui.add_space(8.0);
                            let color = app.s.color;
                            ui.allocate_ui(vec2(92.0, 32.0), |ui| {
                                segmented(
                                    ui,
                                    "language",
                                    &mut app.s.language,
                                    &[(Language::De, "DE"), (Language::En, "EN")],
                                    color,
                                );
                            })
                            .response
                            .on_hover_text(t("Sprache", "Language"));
                        });
                    });
                    let mut sa = ScrollArea::vertical().auto_shrink([false, false]);
                    if let Some(off) = app.dev_scroll {
                        sa = sa.vertical_scroll_offset(off);
                    }
                    sa.show(ui, |ui| {
                            ui.set_width(ui.available_width() - 8.0);
                            section_look(app, ui, acc);
                            section_ambient(app, ui, ctx, acc);
                            section_display(app, ui, acc);
                            section_alarm(app, ui, acc);
                            section_night(app, ui, acc);
                            section_screen(app, ui, ctx, acc);
                            section_keys(ui, acc);
                            ui.add_space(14.0);
                            ui.label(
                                RichText::new(format!(
                                    "NightClock {}  ·  {}",
                                    env!("CARGO_PKG_VERSION"),
                                    t("Einstellungen werden automatisch gespeichert", "Settings are saved automatically")
                                ))
                                .size(11.5)
                                .color(Color32::from_gray(105)),
                            );
                            ui.add_space(8.0);
                        });
                });
        });
}

fn section_look(app: &mut NightClockApp, ui: &mut Ui, acc: Color32) {
    let color = app.s.color;
    card(ui, t("Darstellung", "Appearance"), acc, |ui| {
        weak(ui, t("Stil", "Style"));
        segmented(
            ui,
            "style",
            &mut app.s.style,
            &[
                (ClockStyle::Text, t("Schrift", "Font")),
                (ClockStyle::Segment, "Segment"),
                (ClockStyle::Analog, "Analog"),
            ],
            color,
        );
        if app.s.style != ClockStyle::Analog {
            ui.add_space(4.0);
            weak(ui, t("Anordnung (Taste Z)", "Layout (key Z)"));
            segmented(
                ui,
                "layout",
                &mut app.s.layout,
                &[
                    (LayoutMode::Single, t("Einzeilig", "Single line")),
                    (LayoutMode::Stacked, t("Zweizeilig", "Two lines")),
                    (LayoutMode::Auto, t("Automatisch", "Automatic")),
                ],
                color,
            );
        }
        ui.add_space(4.0);
        weak(ui, t("Schriftart (Ziffern und Datum)", "Font (digits and date)"));
        font_combo(app, ui);
        if let Some((min, def, max)) = app.vfont.weight_axis() {
            if max > min {
                let mut w = if app.s.font_weight > 0.0 { app.s.font_weight } else { def };
                let names = |v: f32| {
                    let n = match v.round() as i32 {
                        ..=150 => t("Hauchdünn", "Thin"),
                        151..=250 => t("Extraleicht", "Extra light"),
                        251..=350 => t("Leicht", "Light"),
                        351..=450 => t("Normal", "Regular"),
                        451..=550 => t("Mittel", "Medium"),
                        551..=650 => t("Halbfett", "Semibold"),
                        651..=750 => t("Fett", "Bold"),
                        _ => t("Extrafett", "Extra bold"),
                    };
                    format!("{n}  ·  {v:.0}")
                };
                if slider_row(ui, t("Schriftstärke", "Font weight"), &mut w, min..=max, names).changed() {
                    app.s.font_weight = w;
                }
            }
        }
        ui.add_space(4.0);
        weak(ui, t("Farbe", "Color"));
        color_row(ui, &mut app.s.color);
        ui.add_space(2.0);
        slider_row(ui, t("Helligkeit", "Brightness"), &mut app.s.brightness, 0.02..=1.0, pct);
        slider_row(ui, t("Leuchten", "Glow"), &mut app.s.glow, 0.0..=1.0, pct);
        slider_row(ui, t("Größe", "Size"), &mut app.s.scale, 0.3..=1.0, pct);
        ui.horizontal(|ui| {
            ui.label(t("Hintergrund", "Background"));
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if ui.small_button(t("Schwarz", "Black")).clicked() {
                    app.s.background = [0, 0, 0];
                }
                ui.color_edit_button_srgb(&mut app.s.background);
            });
        });
    });
}

fn ambient_slot(m: AmbientMode) -> &'static str {
    match m {
        AmbientMode::Off => "prev-off",
        AmbientMode::Plasma => "prev-plasma",
        AmbientMode::Lava => "prev-lava",
        AmbientMode::Aurora => "prev-aurora",
        AmbientMode::Stars => "prev-stars",
        AmbientMode::Fireflies => "prev-fireflies",
        AmbientMode::Breath => "prev-breath",
    }
}

fn section_ambient(app: &mut NightClockApp, ui: &mut Ui, ctx: &Context, acc: Color32) {
    let color = app.s.color;
    card(ui, t("Stimmung", "Ambience"), acc, |ui| {
        weak(ui, t("Bewegter Hintergrund zum Einschlafen", "Moving background for falling asleep"));
        let gap = 8.0;
        let w = ((ui.available_width() - gap) / 2.0).floor();
        let tile_h = 56.0;
        for row in AmbientMode::ALL.chunks(2) {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = gap;
                for &m in row {
                    let (rect, resp) = ui.allocate_exact_size(vec2(w, tile_h + 22.0), Sense::click());
                    let prev = Rect::from_min_size(rect.min, vec2(w, tile_h));
                    let hov = ui.ctx().animate_bool_with_time(resp.id, resp.hovered(), 0.12);
                    ui.painter().rect_filled(prev, 9.0, Color32::from_rgb(8, 9, 12));
                    if m == AmbientMode::Off {
                        ui.painter().text(
                            prev.center(),
                            Align2::CENTER_CENTER,
                            "—",
                            FontId::proportional(18.0),
                            Color32::from_gray(90),
                        );
                    } else {
                        let params = AmbientParams {
                            mode: m,
                            k: 0.95,
                            tint: app.s.ambient_tint.then_some(color),
                            bg: [0, 0, 0],
                            eco: app.s.eco_mode,
                        };
                        app.ambient.paint(ctx, ui.painter(), prev, params, ambient_slot(m), 72.0, 9);
                    }
                    let selected = app.s.ambient == m;
                    let stroke = if selected {
                        Stroke::new(2.0_f32, acc)
                    } else {
                        Stroke::new(1.0_f32, Color32::from_white_alpha((14.0 + 30.0 * hov) as u8))
                    };
                    ui.painter().rect_stroke(prev, 9.0, stroke, egui::StrokeKind::Inside);
                    ui.painter().text(
                        pos2(prev.left() + 2.0, prev.bottom() + 11.0),
                        Align2::LEFT_CENTER,
                        m.label(),
                        FontId::proportional(13.0),
                        if selected { acc } else { Color32::from_gray((170.0 + 50.0 * hov) as u8) },
                    );
                    if resp.clicked() {
                        app.s.ambient = m;
                    }
                }
            });
        }
        if app.s.ambient != AmbientMode::Off {
            ui.add_space(2.0);
            slider_row(ui, t("Helligkeit", "Brightness"), &mut app.s.ambient_intensity, 0.05..=1.0, pct);
            slider_row(ui, t("Tempo", "Speed"), &mut app.s.ambient_speed, 0.2..=2.5, |v| format!("{v:.1}×"));
            toggle_row(ui, t("In Uhrfarbe einfärben", "Tint with clock color"), &mut app.s.ambient_tint, acc);
            slider_row(ui, t("Ausblenden nach", "Fade out after"), &mut app.s.ambient_timeout, 0..=180, |v| {
                if v == 0 {
                    t("nie", "never").to_string()
                } else {
                    format!("{v} min")
                }
            })
            .on_hover_text(t(
                "Blendet den Hintergrund aus, wenn so lange nichts bedient wurde (spart Strom)",
                "Fades the background out after this long without input (saves power)",
            ));
            if app.s.ambient == AmbientMode::Breath {
                ui.label(
                    RichText::new(t(
                        "Atme mit dem Licht: 4 Sekunden ein, 6 Sekunden aus",
                        "Breathe with the light: 4 seconds in, 6 seconds out",
                    ))
                    .size(12.5)
                    .color(Color32::from_gray(140)),
                );
            }
        }
    });
}

fn color_row(ui: &mut Ui, color: &mut [u8; 3]) {
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing = vec2(7.0, 7.0);
        for (c, de, en) in PRESETS {
            let (rect, resp) = ui.allocate_exact_size(vec2(28.0, 28.0), Sense::click());
            let hov = ui.ctx().animate_bool_with_time(resp.id, resp.hovered(), 0.1);
            let p = ui.painter();
            let selected = *color == c;
            if selected {
                p.circle_stroke(rect.center(), 13.0, Stroke::new(2.0_f32, Color32::from_gray(235)));
            }
            p.circle_filled(rect.center(), 9.5 + 1.5 * hov, accent(c));
            if resp.clicked() {
                *color = c;
            }
            resp.on_hover_text(t(de, en));
        }
        ui.add_space(2.0);
        ui.color_edit_button_srgb(color).on_hover_text(t("Eigene Farbe", "Custom color"));
    });
}

fn font_combo(app: &mut NightClockApp, ui: &mut Ui) {
    let list = app.font_list.lock().ok().and_then(|g| g.clone());
    let mut chosen: Option<(String, String, u32)> = None;
    let current = app.s.font_name.clone();
    ComboBox::from_id_salt("font_combo")
        .width(ui.available_width())
        .height(420.0)
        .close_behavior(PopupCloseBehavior::CloseOnClickOutside)
        .selected_text(&current)
        .show_ui(ui, |ui| {
            ui.add(
                TextEdit::singleline(&mut app.font_search)
                    .hint_text(t("Schrift suchen …", "Search fonts …"))
                    .desired_width(f32::INFINITY),
            );
            ui.add_space(4.0);
            match &list {
                None => {
                    ui.label(t("Schriften werden geladen …", "Loading fonts …"));
                }
                Some(list) => {
                    let q = app.font_search.to_lowercase();
                    let mut item = |ui: &mut Ui, f: &crate::fonts::FontEntry| {
                        if ui.selectable_label(current == f.name, &f.name).clicked() {
                            chosen = Some((f.name.clone(), f.path.to_string_lossy().into_owned(), f.index));
                        }
                    };
                    if q.is_empty() {
                        weak(ui, t("EMPFOHLEN", "RECOMMENDED"));
                        for f in list.iter().filter(|f| f.recommended) {
                            item(ui, f);
                        }
                        ui.separator();
                        weak(ui, t("ALLE SCHRIFTEN", "ALL FONTS"));
                    }
                    for f in list.iter().filter(|f| q.is_empty() || f.name.to_lowercase().contains(&q)) {
                        item(ui, f);
                    }
                }
            }
        });
    if let Some((name, path, index)) = chosen {
        app.s.font_name = name;
        app.s.font_path = path;
        app.s.font_index = index;
        ui.close_menu();
    }
}

fn section_display(app: &mut NightClockApp, ui: &mut Ui, acc: Color32) {
    let color = app.s.color;
    card(ui, t("Anzeige", "Display"), acc, |ui| {
        weak(ui, t("Sekunden", "Seconds"));
        let opts: Vec<(SecondsMode, &str)> = if app.s.style == ClockStyle::Analog {
            vec![(SecondsMode::Off, t("Aus", "Off")), (SecondsMode::Digits, t("Sekundenzeiger", "Second hand"))]
        } else {
            vec![(SecondsMode::Off, t("Aus", "Off")), (SecondsMode::Digits, t("Ziffern", "Digits")), (SecondsMode::Line, t("Linie", "Line"))]
        };
        if app.s.style == ClockStyle::Analog && app.s.seconds == SecondsMode::Line {
            app.s.seconds = SecondsMode::Digits;
        }
        segmented(ui, "seconds", &mut app.s.seconds, &opts, color);
        ui.add_space(2.0);
        toggle_row(ui, t("24-Stunden-Format", "24-hour format"), &mut app.s.use_24h, acc);
        if app.s.style != ClockStyle::Analog {
            toggle_row(ui, t("Führende Null (07:30)", "Leading zero (07:30)"), &mut app.s.leading_zero, acc);
            toggle_row(ui, t("Doppelpunkt pulsiert", "Pulsing colon"), &mut app.s.blink_colon, acc);
        }
        toggle_row(ui, t("Datum anzeigen", "Show date"), &mut app.s.show_date, acc);
        toggle_row(ui, t("Nächsten Wecker anzeigen", "Show next alarm"), &mut app.s.show_next_alarm, acc);
        if app.s.style == ClockStyle::Segment {
            toggle_row(ui, t("Inaktive Segmente zeigen", "Show inactive segments"), &mut app.s.ghost_segments, acc);
            toggle_row(ui, t("Kursive Segmente", "Slanted segments"), &mut app.s.slanted_segments, acc);
        }
    });
}

fn day_hint(a: &Alarm) -> &'static str {
    let d = a.days;
    if !d.iter().any(|x| *x) {
        t("einmalig", "once")
    } else if d.iter().all(|x| *x) {
        t("täglich", "daily")
    } else if d[..5].iter().all(|x| *x) && !d[5] && !d[6] {
        t("werktags", "weekdays")
    } else if !d[..5].iter().any(|x| *x) && d[5] && d[6] {
        t("am Wochenende", "weekends")
    } else {
        ""
    }
}

fn section_alarm(app: &mut NightClockApp, ui: &mut Ui, acc: Color32) {
    let color = app.s.color;
    let on = on_accent(color);
    card(ui, t("Wecker", "Alarm"), acc, |ui| {
        if let Some(su) = app.snooze_until {
            use chrono::Timelike;
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new(format!("{} {:02}:{:02}", t("Schlummert bis", "Snoozing until"), su.hour(), su.minute()))
                        .color(acc),
                );
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if ui.small_button(t("Beenden", "End")).clicked() {
                        app.snooze_until = None;
                    }
                });
            });
            ui.add_space(4.0);
        }

        let mut remove = None;
        let count = app.s.alarms.len();
        for (i, a) in app.s.alarms.iter_mut().enumerate() {
            Frame::new()
                .fill(Color32::from_rgb(30, 32, 39))
                .corner_radius(10)
                .inner_margin(Margin::symmetric(10, 8))
                .show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    ui.horizontal(|ui| {
                        toggle(ui, &mut a.enabled, acc);
                        ui.add_space(6.0);
                        time_edit(ui, &mut a.hour, &mut a.minute, 24.0);
                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            if count > 0
                                && round_button(ui, 26.0, |p, c, col| close_icon(p, c, 26.0, col))
                                    .on_hover_text(t("Wecker löschen", "Delete alarm"))
                                    .clicked()
                            {
                                remove = Some(i);
                            }
                            ui.label(RichText::new(day_hint(a)).size(12.5).color(Color32::from_gray(130)));
                        });
                    });
                    ui.add_space(2.0);
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = 4.0;
                        ui.spacing_mut().button_padding = vec2(2.0, 2.0);
                        let pill_w = ((ui.available_width() - 6.0 * 4.0) / 7.0).max(24.0);
                        for (d, name) in crate::i18n::day_short_all().iter().enumerate() {
                            let sel = a.days[d];
                            let b = Button::new(
                                RichText::new(*name)
                                    .size(12.5)
                                    .color(if sel { on } else { Color32::from_gray(170) }),
                            )
                            .fill(if sel { acc } else { Color32::from_rgb(42, 44, 53) })
                            .corner_radius(7)
                            .min_size(vec2(pill_w, 24.0));
                            if ui.add(b).clicked() {
                                a.days[d] = !a.days[d];
                            }
                        }
                    });
                });
            ui.add_space(4.0);
        }
        if let Some(i) = remove {
            app.s.alarms.remove(i);
        }
        if app.s.alarms.len() < 8
            && ui
                .add(
                    Button::new(t("+   Wecker hinzufügen", "+   Add alarm"))
                        .min_size(vec2(ui.available_width(), 32.0))
                        .corner_radius(10),
                )
                .clicked()
        {
            app.s.alarms.push(Alarm::default());
        }

        ui.add_space(6.0);
        weak(ui, t("Weckton", "Alarm sound"));
        ui.horizontal(|ui| {
            let playing = app.preview.is_some();
            let bw = 96.0;
            ComboBox::from_id_salt("sound_combo")
                .width(ui.available_width() - bw - 8.0)
                .selected_text(app.s.sound.label())
                .show_ui(ui, |ui| {
                    for snd in AlarmSound::ALL {
                        ui.selectable_value(&mut app.s.sound, snd, snd.label());
                    }
                });
            let label = if playing { t("■  Stopp", "■  Stop") } else { t("▶  Anhören", "▶  Preview") };
            if ui
                .add(Button::new(label).min_size(vec2(bw, 26.0)))
                .clicked()
            {
                app.toggle_preview();
            }
        });
        if app.s.sound == AlarmSound::Custom {
            ui.horizontal(|ui| {
                let name = std::path::Path::new(&app.s.custom_sound_path)
                    .file_name()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_else(|| t("Keine Datei gewählt", "No file selected").into());
                if ui.button(t("Datei wählen …", "Choose file …")).clicked() {
                    if let Some(p) = rfd::FileDialog::new()
                        .set_title(t("Weckton auswählen", "Choose alarm sound"))
                        .add_filter("Audio", &["mp3", "wav", "ogg", "flac", "m4a", "aac"])
                        .pick_file()
                    {
                        app.s.custom_sound_path = p.to_string_lossy().into_owned();
                        app.preview = None;
                    }
                }
                ui.add(egui::Label::new(RichText::new(name).color(Color32::from_gray(150))).truncate());
            });
        }
        slider_row(ui, t("Lautstärke", "Volume"), &mut app.s.volume, 0.0..=1.0, pct);
        slider_row(ui, t("Sanft lauter werden", "Gentle fade-in"), &mut app.s.fade_in_secs, 0.0..=180.0, |v| {
            if v < 1.0 {
                t("aus", "off").into()
            } else {
                format!("{v:.0} s")
            }
        });
        slider_row(ui, t("Schlummerzeit", "Snooze time"), &mut app.s.snooze_minutes, 1..=30, |v| format!("{v} min"));
        slider_row(ui, t("Automatisch aus nach", "Auto-off after"), &mut app.s.auto_stop_minutes, 1..=60, |v| format!("{v} min"));
        ui.add_space(2.0);
        toggle_row(ui, t("Lichtwecker (Sonnenaufgang)", "Wake-up light (sunrise)"), &mut app.s.sunrise, acc)
            .on_hover_text(t("Vor dem Wecken wird der Bildschirm langsam warm und hell", "Before the alarm, the screen slowly turns warm and bright"));
        if app.s.sunrise {
            slider_row(ui, t("Beginnt vorher", "Starts before"), &mut app.s.sunrise_minutes, 5..=45, |v| format!("{v} min"));
        }
    });
}

fn section_night(app: &mut NightClockApp, ui: &mut Ui, acc: Color32) {
    card(ui, t("Nachtmodus", "Night mode"), acc, |ui| {
        toggle_row(ui, t("Nachts automatisch dimmen", "Dim automatically at night"), &mut app.s.night_dim, acc);
        if app.s.night_dim {
            ui.horizontal(|ui| {
                ui.label(t("von", "from"));
                minutes_edit(ui, &mut app.s.night_start);
                ui.add_space(8.0);
                ui.label(t("bis", "to"));
                minutes_edit(ui, &mut app.s.night_end);
            });
            slider_row(ui, t("Helligkeit nachts", "Brightness at night"), &mut app.s.night_brightness, 0.02..=1.0, pct);
        }
    });
}

fn section_screen(app: &mut NightClockApp, ui: &mut Ui, ctx: &Context, acc: Color32) {
    let color = app.s.color;
    card(ui, t("Bildschirm", "Screen"), acc, |ui| {
        weak(ui, t("Bildschirm für das Vollbild", "Display for fullscreen"));
        let mons = app.monitors.clone();
        let target = app.target_monitor();
        let target_dev = target.as_ref().map(|m| m.device.clone()).unwrap_or_default();
        let sel_text = target
            .as_ref()
            .map(|m| {
                let i = mons.iter().position(|x| x.device == m.device).unwrap_or(0);
                m.label(i)
            })
            .unwrap_or_else(|| "–".into());
        let mut chosen = None;
        ComboBox::from_id_salt("monitor_combo")
            .width(ui.available_width())
            .selected_text(sel_text)
            .show_ui(ui, |ui| {
                for (i, m) in mons.iter().enumerate() {
                    if ui.selectable_label(m.device == target_dev, m.label(i)).clicked() {
                        chosen = Some(m.device.clone());
                    }
                }
            });
        if let Some(d) = chosen {
            app.switch_monitor(ctx, d);
        }
        ui.horizontal(|ui| {
            let label = if app.is_fullscreen { t("Vollbild beenden", "Exit fullscreen") } else { t("Vollbild starten", "Start fullscreen") };
            if ui
                .add(
                    Button::new(RichText::new(label).color(on_accent(color)))
                        .fill(acc)
                        .min_size(vec2(150.0, 28.0)),
                )
                .clicked()
            {
                app.toggle_fullscreen(ctx);
            }
            if ui.button(t("Neu erkennen", "Detect again")).on_hover_text(t("Bildschirme neu suchen", "Search for displays again")).clicked() {
                app.refresh_monitors();
            }
        });
        ui.add_space(4.0);
        toggle_row(ui, t("Beim Start im Vollbild", "Start in fullscreen"), &mut app.s.start_fullscreen, acc);
        toggle_row(ui, t("Bildschirm wach halten", "Keep screen awake"), &mut app.s.keep_awake, acc)
            .on_hover_text(t("Verhindert Bildschirmschoner und Abschalten des Monitors", "Prevents the screensaver and monitor sleep"));
        toggle_row(ui, t("Einbrennschutz (Uhr wandert sanft)", "Burn-in protection (clock drifts slowly)"), &mut app.s.burn_in_shift, acc)
            .on_hover_text(t("Schützt OLED-Bildschirme vor eingebrannten Ziffern", "Protects OLED screens from burned-in digits"));
        toggle_row(ui, t("Mauszeiger im Vollbild ausblenden", "Hide mouse cursor in fullscreen"), &mut app.s.hide_cursor, acc);
        toggle_row(ui, t("Immer im Vordergrund", "Always on top"), &mut app.s.always_on_top, acc);
        toggle_row(ui, t("Sparmodus für schwache Grafik", "Eco mode for weak graphics"), &mut app.s.eco_mode, acc)
            .on_hover_text(t(
                "Weniger Bilder pro Sekunde und ein einfacheres Leuchten, z. B. für Laptops oder Onboard-Grafik",
                "Fewer frames per second and a simpler glow, e.g. for laptops or integrated graphics",
            ));
    });
}

fn section_keys(ui: &mut Ui, acc: Color32) {
    card(ui, t("Tastenkürzel", "Keyboard shortcuts"), acc, |ui| {
        let keys: [(&str, &str); 9] = [
            (t("F11 · F · Doppelklick", "F11 · F · Double-click"), t("Vollbild an / aus", "Fullscreen on / off")),
            ("Esc", t("Vollbild bzw. Menü schließen", "Exit fullscreen / close menu")),
            (t("S · Rechtsklick", "S · Right-click"), t("Einstellungen", "Settings")),
            (t("Mausrad · ↑ ↓", "Mouse wheel · ↑ ↓"), t("Helligkeit", "Brightness")),
            ("M", t("Nächster Bildschirm", "Next display")),
            ("1 · 2 · 3", t("Stil wechseln", "Change style")),
            ("Z", t("Einzeilig / Zweizeilig", "Single / two lines")),
            (t("Leertaste", "Space"), t("Schlummern", "Snooze")),
            ("Enter", t("Wecker ausschalten", "Turn off alarm")),
        ];
        egui::Grid::new("keys_grid")
            .num_columns(2)
            .spacing([12.0, 6.0])
            .show(ui, |ui| {
                for (k, d) in keys {
                    kbd(ui, k);
                    ui.label(RichText::new(d).size(13.5).color(Color32::from_gray(190)));
                    ui.end_row();
                }
            });
    });
}

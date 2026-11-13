//! Stimmungs-Hintergründe (Plasma, Lavalampe, Polarlicht, Sternenhimmel,
//! Glühwürmchen, Atem-Licht).
//!
//! Bewusst sparsam, damit es auch auf schwacher Onboard-Grafik läuft:
//! Weiche Flächen werden auf der CPU in sehr kleiner Auflösung berechnet
//! (längste Seite ~200 px, max. 20 Aktualisierungen pro Sekunde) und von der
//! Grafikkarte nur weich hochskaliert. Feine Details (Sterne, Glühwürmchen)
//! sind wenige, einfache Vektor-Punkte. Keine eigenen Shader.

use crate::settings::AmbientMode;
use egui::{pos2, vec2, Color32, ColorImage, Mesh, Painter, Pos2, Rect, Shape, Stroke, TextureHandle, TextureOptions};
use std::collections::HashMap;
use std::f32::consts::{PI, TAU};
use std::time::{Duration, Instant};

/// Darstellungs-Parameter für einen Hintergrund.
#[derive(Clone, Copy)]
pub struct AmbientParams {
    pub mode: AmbientMode,
    /// Gesamthelligkeit (Intensität × Uhr-Helligkeit × Ausblenden)
    pub k: f32,
    /// Mit der Uhrfarbe einfärben
    pub tint: Option<[u8; 3]>,
    pub bg: [u8; 3],
    /// Sparmodus: weniger Sterne, seltener neu berechnen
    pub eco: bool,
}

struct Slot {
    tex: TextureHandle,
    img: ColorImage,
    /// zwischengespeicherter, unveränderlicher Grund (Sterne, Glühwürmchen)
    base: Vec<[f32; 3]>,
    base_key: (AmbientMode, [usize; 2]),
    last_update: Instant,
    key: (AmbientMode, [usize; 2], u32, Option<[u8; 3]>, [u8; 3]),
}

struct Star {
    x: f32,
    y: f32,
    mag: f32,
    size: f32,
    freq: f32,
    phase: f32,
    color: [f32; 3],
}

struct Firefly {
    x: f32,
    y: f32,
    ax: f32,
    ay: f32,
    fx: f32,
    fy: f32,
    blink: f32,
    phase: f32,
}

pub struct Ambient {
    slots: HashMap<&'static str, Slot>,
    stars: Vec<Star>,
    flies: Vec<Firefly>,
    /// Laufende Zeit in Sekunden (mit Tempo-Faktor)
    pub time: f32,
    last: Instant,
}

// ───────────────────────── kleine Mathe-Helfer ─────────────────────────

struct Rng(u32);
impl Rng {
    fn next(&mut self) -> f32 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 17;
        self.0 ^= self.0 << 5;
        (self.0 & 0xFF_FFFF) as f32 / 16_777_215.0
    }
}

fn hash(x: i32, y: i32, seed: u32) -> f32 {
    let mut h = (x as u32).wrapping_mul(374_761_393)
        ^ (y as u32).wrapping_mul(668_265_263)
        ^ seed.wrapping_mul(2_246_822_519);
    h = (h ^ (h >> 13)).wrapping_mul(1_274_126_177);
    h ^= h >> 16;
    (h & 0xFF_FFFF) as f32 / 16_777_215.0
}

fn smooth(t: f32) -> f32 {
    t * t * (3.0 - 2.0 * t)
}

fn noise1(x: f32, seed: u32) -> f32 {
    let i = x.floor();
    let f = smooth(x - i);
    let a = hash(i as i32, 0, seed);
    let b = hash(i as i32 + 1, 0, seed);
    a + (b - a) * f
}

fn fbm1(x: f32, seed: u32) -> f32 {
    let (mut v, mut amp, mut fr, mut norm) = (0.0, 0.5, 1.0, 0.0);
    for o in 0..4 {
        v += noise1(x * fr, seed + o) * amp;
        norm += amp;
        amp *= 0.5;
        fr *= 2.0;
    }
    v / norm
}

fn noise2(x: f32, y: f32, seed: u32) -> f32 {
    let (ix, iy) = (x.floor(), y.floor());
    let (fx, fy) = (smooth(x - ix), smooth(y - iy));
    let (ix, iy) = (ix as i32, iy as i32);
    let a = hash(ix, iy, seed);
    let b = hash(ix + 1, iy, seed);
    let c = hash(ix, iy + 1, seed);
    let d = hash(ix + 1, iy + 1, seed);
    let top = a + (b - a) * fx;
    let bot = c + (d - c) * fx;
    top + (bot - top) * fy
}

fn fbm2(x: f32, y: f32, seed: u32) -> f32 {
    let (mut v, mut amp, mut fr, mut norm) = (0.0, 0.5, 1.0, 0.0);
    for o in 0..4 {
        v += noise2(x * fr, y * fr, seed + o) * amp;
        norm += amp;
        amp *= 0.5;
        fr *= 2.03;
    }
    v / norm
}

fn mix3(a: [f32; 3], b: [f32; 3], t: f32) -> [f32; 3] {
    [
        a[0] + (b[0] - a[0]) * t,
        a[1] + (b[1] - a[1]) * t,
        a[2] + (b[2] - a[2]) * t,
    ]
}

fn add3(a: [f32; 3], b: [f32; 3], k: f32) -> [f32; 3] {
    [a[0] + b[0] * k, a[1] + b[1] * k, a[2] + b[2] * k]
}

/// Atemkurve: 4 s einatmen, 6 s ausatmen (bei Tempo 1).
pub fn breath(t: f32) -> f32 {
    let ph = (t / 10.0).fract();
    let ease = |x: f32| 0.5 - 0.5 * (PI * x.clamp(0.0, 1.0)).cos();
    if ph < 0.4 {
        ease(ph / 0.4)
    } else {
        ease(1.0 - (ph - 0.4) / 0.6)
    }
}

// ───────────────────────── Lavalampe ─────────────────────────

/// Schwelle des Feldes: darüber ist Wachs
const LAVA_T: f32 = 0.2;

#[derive(Clone, Copy)]
struct Ball {
    x: f32,
    y: f32,
    /// Quadrat der Reichweite R (Feld = (1 - d²/R²)³ innerhalb von R)
    r2: f32,
    /// < 1 = senkrecht gestreckt
    s: f32,
}

/// Reichweite² aus dem sichtbaren Radius einer einzelnen Blase.
fn reach2(visible_r: f32) -> f32 {
    let r = visible_r / 0.644;
    r * r
}

/// Wachs-Teile einer Lavalampe. Jeder Tropfen besteht aus mehreren kleinen,
/// unabhängig wabernden Teilen, damit er ständig seine Form ändert (auch allein).
/// Die Tropfen wandern nach einem langsamen Zufallsrauschen: steigen, ruhen oben,
/// sinken, ziehen seitlich und kehren auch mal auf halbem Weg um. In Bewegung
/// ziehen sie einen Schwanz hinter sich her.
/// Koordinaten in Bildschirmhöhen (x: 0..Seitenverhältnis, y: 0..1, unten = 1).
fn lava_balls(t: f32, aspect: f32) -> Vec<Ball> {
    let unit = aspect.min(1.0);
    let mut v = Vec::with_capacity(48);

    // Wachs-See am Boden, wabernd
    for i in 0..5 {
        let fi = i as f32;
        let fx = (fi + 0.5) / 5.0;
        let wob_y = unit * (0.02 * (t * 0.13 + fi * 1.9).sin() + 0.012 * (t * 0.31 + fi * 0.7).sin());
        let wob_r = 1.0 + 0.14 * (t * 0.09 + fi * 2.7).sin();
        v.push(Ball {
            x: aspect * (0.06 + 0.88 * fx) + unit * 0.025 * (t * 0.07 + fi).sin(),
            y: 1.0 + unit * 0.07 + wob_y,
            r2: reach2(unit * 0.15 * wob_r),
            s: 1.0,
        });
    }
    // kleiner Rest oben
    for i in 0..2 {
        let fi = i as f32;
        let fx = (fi + 0.5) / 2.0;
        let wob = 1.0 + 0.15 * (t * 0.08 + fi * 3.1).sin();
        v.push(Ball {
            x: aspect * (0.32 + 0.36 * fx),
            y: -unit * 0.045,
            r2: reach2(unit * 0.08 * wob),
            s: 1.0,
        });
    }

    // Tropfen: einige große, ein paar kleine Perlen
    const SIZES: [f32; 7] = [0.16, 0.14, 0.12, 0.105, 0.085, 0.045, 0.032];
    let mut rng = Rng(0x00C0_FFEE);
    for (i, &sz) in SIZES.iter().enumerate() {
        let seed = 100 + i as u32 * 17;
        let fy = 0.010 + 0.008 * rng.next();
        let fxq = 0.006 + 0.006 * rng.next();
        let off = rng.next() * 50.0;
        // senkrechte Position: Rauschen, an den Enden gesättigt -> Tropfen ruhen oben/unten
        let ypos = |tt: f32| {
            let n = fbm1(tt * fy + off, seed);
            let e = ((n - 0.5) * 3.2 + 0.5).clamp(0.0, 1.0);
            0.07 + 0.88 * smooth(e)
        };
        let y = ypos(t);
        let vy = (ypos(t + 1.0) - ypos(t - 1.0)) * 0.5;
        let base_x = 0.18 + 0.64 * (i as f32 * 0.618).fract();
        let drift = 0.18 * (fbm1(t * fxq + off * 1.3, seed + 7) - 0.5) * 2.0;
        let x = aspect * (base_x + drift).clamp(0.08, 0.92);
        let r = unit * sz;

        // wabernde Teile
        const PART: [f32; 3] = [0.80, 0.72, 0.64];
        for &pk in &PART {
            let w1 = 0.07 + 0.06 * rng.next();
            let w2 = 0.05 + 0.07 * rng.next();
            let p1 = rng.next() * TAU;
            let p2 = rng.next() * TAU;
            let amp = r * (0.28 + 0.16 * rng.next());
            v.push(Ball {
                x: x + amp * (t * w1 + p1).cos(),
                y: y + amp * 1.15 * (t * w2 + p2).sin(),
                r2: reach2(r * pk),
                s: 1.0,
            });
        }
        // Schwanz entgegen der Bewegung (steigend = y nimmt ab -> Schwanz unten)
        let speed = (vy.abs() * 55.0).min(1.0);
        if speed > 0.05 && sz > 0.04 {
            let dir = if vy < 0.0 { 1.0 } else { -1.0 };
            let len = r * (0.55 + 0.7 * speed);
            v.push(Ball {
                x: x + r * 0.12 * (t * 0.2 + off).sin(),
                y: y + dir * len,
                r2: reach2(r * 0.52 * (0.7 + 0.3 * speed)),
                s: 1.0,
            });
        }
    }
    v
}

/// Feldwert (>= LAVA_T = Wachs) und Gradient. Begrenzte Reichweite, damit nur
/// nahe Blasen verschmelzen.
fn lava_field(balls: &[Ball], x: f32, y: f32) -> (f32, f32, f32) {
    let (mut f, mut gx, mut gy) = (0.0, 0.0, 0.0);
    for b in balls {
        let dx = x - b.x;
        let dy = (y - b.y) * b.s;
        let q = (dx * dx + dy * dy) / b.r2;
        if q >= 1.0 {
            continue;
        }
        let u = 1.0 - q;
        f += u * u * u;
        // df/dx = -3 u² · 2dx/R²
        let k = -6.0 * u * u / b.r2;
        gx += k * dx;
        gy += k * dy * b.s;
    }
    (f, gx, gy)
}

/// Farbe des Wachses: innen hell glühend, am Rand satter, unten von der Lampe angestrahlt.
fn wax_color(f: f32, y: f32, gx: f32, gy: f32) -> [f32; 3] {
    let depth = smooth(((f - LAVA_T) / 0.75).clamp(0.0, 1.0));
    // oben kühleres Rot, unten nahe der Lampe glühendes Orange-Rot
    let cool = [0.95, 0.10, 0.13];
    let hot = [1.0, 0.38, 0.10];
    let wax = mix3(cool, hot, smooth(((y - 0.35) / 0.65).clamp(0.0, 1.0)));
    let body = 0.55 + 0.45 * depth;
    let mut c = [wax[0] * body, wax[1] * body, wax[2] * body];
    // Normale zeigt nach außen (entgegen dem Gradienten)
    let len = (gx * gx + gy * gy).sqrt().max(1e-6);
    let ny = -gy / len;
    let edge = 1.0 - depth;
    // Lichtsaum von der Lampe unten
    let rim = ny.max(0.0).powi(2) * edge * (0.3 + 0.7 * y);
    c = add3(c, [1.0, 0.62, 0.35], rim * 0.5);
    // zarter Glanz oben
    let sheen = (-ny).max(0.0).powi(6) * edge * 0.10;
    add3(c, [1.0, 0.75, 0.85], sheen)
}

fn is_dynamic(mode: AmbientMode) -> bool {
    matches!(
        mode,
        AmbientMode::Plasma | AmbientMode::Lava | AmbientMode::Aurora | AmbientMode::Breath
    )
}

// ───────────────────────── Flächen (niedrige Auflösung) ─────────────────────────

/// Unveränderlicher Grund für Sterne/Glühwürmchen (einmal berechnet).
fn render_base(mode: AmbientMode, w: usize, h: usize, out: &mut Vec<[f32; 3]>) {
    out.clear();
    out.reserve(w * h);
    let aspect = w as f32 / h as f32;
    for py in 0..h {
        let y = (py as f32 + 0.5) / h as f32;
        for px in 0..w {
            let x = (px as f32 + 0.5) / h as f32;
            let c = match mode {
                AmbientMode::Stars => {
                    // Nachtblau, unten etwas heller, zarte Milchstraßen-Nebel
                    let sky = mix3([0.010, 0.016, 0.060], [0.040, 0.035, 0.120], y);
                    let band = (-((x - aspect * 0.55) * 0.9 + (y - 0.45) * 0.7).powi(2) * 6.0).exp();
                    let n = fbm2(x * 3.0, y * 3.0, 7);
                    let neb = (n - 0.35).max(0.0) * band;
                    add3(sky, [0.32, 0.22, 0.55], neb * 0.55)
                }
                AmbientMode::Fireflies => {
                    // Waldnacht mit leichtem Nebel über dem Boden
                    let sky = mix3([0.008, 0.020, 0.025], [0.020, 0.050, 0.040], y);
                    let mist = fbm2(x * 2.2, y * 4.0, 11) * (y * y);
                    add3(sky, [0.10, 0.20, 0.16], mist * 0.35)
                }
                _ => [0.0; 3],
            };
            out.push(c);
        }
    }
}

/// Bewegte Flächen, Rückgabe in `out` (Farben 0..~1, ohne Helligkeit).
fn render_dynamic(mode: AmbientMode, w: usize, h: usize, t: f32, out: &mut Vec<[f32; 3]>) {
    out.clear();
    out.reserve(w * h);
    let aspect = w as f32 / h as f32;
    match mode {
        AmbientMode::Plasma => {
            let (cx, cy) = (aspect * (0.5 + 0.35 * (t * 0.07).sin()), 0.5 + 0.3 * (t * 0.05).cos());
            let indigo = [0.10, 0.06, 0.32];
            let violet = [0.36, 0.10, 0.46];
            let teal = [0.04, 0.26, 0.42];
            for py in 0..h {
                let y = (py as f32 + 0.5) / h as f32;
                for px in 0..w {
                    let x = (px as f32 + 0.5) / h as f32;
                    let (sx, sy) = (x * 2.6, y * 2.6);
                    let d = ((x - cx).powi(2) + (y - cy).powi(2)).sqrt();
                    let v = (sx + t * 0.23).sin()
                        + (sy * 1.3 - t * 0.17).sin()
                        + ((sx + sy) * 0.7 + t * 0.13).sin()
                        + (d * 5.0 - t * 0.3).sin();
                    let s = v * 0.125 + 0.5; // 0..1
                    let c = if s < 0.5 {
                        mix3(indigo, teal, smooth(s * 2.0))
                    } else {
                        mix3(teal, violet, smooth((s - 0.5) * 2.0))
                    };
                    let shade = 0.55 + 0.45 * (v * 0.9 + t * 0.05).sin().abs();
                    out.push([c[0] * shade, c[1] * shade, c[2] * shade]);
                }
            }
        }
        AmbientMode::Lava => {
            // Nur die leuchtende Flüssigkeit und der weiche Schein um das Wachs;
            // das Wachs selbst wird scharf als Vektorfläche gezeichnet (paint_lava).
            let balls = lava_balls(t, aspect);
            for py in 0..h {
                let y = (py as f32 + 0.5) / h as f32;
                // von unten beleuchtet
                // violette Flüssigkeit, unten von der Lampe aufgehellt
                let liquid = mix3([0.03, 0.006, 0.07], [0.24, 0.045, 0.34], y * y);
                for px in 0..w {
                    let x = (px as f32 + 0.5) / h as f32;
                    let (f, _, _) = lava_field(&balls, x, y);
                    let halo = (f / LAVA_T).min(1.0).powi(2) * 0.5;
                    let bulb = (-((x - aspect * 0.5).powi(2) * 3.0 + (1.0 - y).powi(2) * 9.0)).exp() * 0.18;
                    out.push(add3(add3(liquid, [0.85, 0.12, 0.22], halo * 0.45), [0.95, 0.40, 0.55], bulb));
                }
            }
        }
        AmbientMode::Aurora => {
            // Fast alles hängt nur von x ab -> pro Spalte vorberechnen
            let mut yc = vec![0.0f32; w];
            let mut ray = vec![0.0f32; w];
            let mut br = vec![0.0f32; w];
            let mut yc2 = vec![0.0f32; w];
            for px in 0..w {
                let x = (px as f32 + 0.5) / h as f32;
                yc[px] = 0.44 + 0.15 * (fbm1(x * 0.9 + t * 0.03, 3) - 0.5) * 2.0;
                yc2[px] = 0.30 + 0.12 * (fbm1(x * 0.8 - t * 0.022, 9) - 0.5) * 2.0;
                ray[px] = 0.62 + 0.38 * noise1(x * 6.5 + t * 0.16, 5);
                br[px] = smooth((fbm1(x * 0.6 - t * 0.02, 13) * 1.9 - 0.4).clamp(0.0, 1.0));
            }
            let green = [0.12, 0.85, 0.45];
            let violet = [0.45, 0.18, 0.75];
            for py in 0..h {
                let y = (py as f32 + 0.5) / h as f32;
                let sky = mix3([0.006, 0.012, 0.040], [0.015, 0.040, 0.080], y);
                for px in 0..w {
                    let curtain = |c: f32| {
                        if y <= c {
                            (-(c - y) * 3.2).exp()
                        } else {
                            (-((y - c) * 14.0)).exp()
                        }
                    };
                    let a1 = curtain(yc[px]) * ray[px] * (0.25 + 0.75 * br[px]);
                    let a2 = curtain(yc2[px]) * ray[px] * (1.0 - br[px] * 0.6) * 0.55;
                    let up = ((yc[px] - y) * 2.0).clamp(0.0, 1.0);
                    let col = mix3(green, violet, smooth(up));
                    out.push(add3(add3(sky, col, a1 * 1.05), [0.18, 0.60, 0.70], a2 * 0.7));
                }
            }
        }
        AmbientMode::Breath => {
            let b = breath(t);
            let r = 0.20 + 0.16 * b;
            let (cx, cy) = (aspect * 0.5, 0.5);
            for py in 0..h {
                let y = (py as f32 + 0.5) / h as f32;
                for px in 0..w {
                    let x = (px as f32 + 0.5) / h as f32;
                    let d = ((x - cx).powi(2) + (y - cy).powi(2)).sqrt() / r;
                    let v = (-d * d * 1.6).exp() * (0.30 + 0.70 * b) + (-d * 0.9).exp() * 0.06;
                    out.push([0.36 * v, 0.42 * v, 1.0 * v]);
                }
            }
        }
        _ => {}
    }
}

fn tex_size(rect: Rect, long: f32) -> [usize; 2] {
    let a = (rect.width() / rect.height().max(1.0)).clamp(0.2, 5.0);
    if a >= 1.0 {
        [long as usize, (long / a).round().max(8.0) as usize]
    } else {
        [(long * a).round().max(8.0) as usize, long as usize]
    }
}

/// Weicher, leuchtender Punkt (additiv, wenige Dreiecke).
fn soft_dot(p: &Painter, c: Pos2, r: f32, col: [f32; 3], k: f32) {
    const SEGS: usize = 18;
    const RINGS: usize = 5;
    let color = |t: f32| {
        let f = (-4.0 * t * t).exp() * (1.0 - t) * k;
        Color32::from_rgba_premultiplied(
            (col[0] * f * 255.0).clamp(0.0, 255.0) as u8,
            (col[1] * f * 255.0).clamp(0.0, 255.0) as u8,
            (col[2] * f * 255.0).clamp(0.0, 255.0) as u8,
            0,
        )
    };
    let mut mesh = Mesh::default();
    mesh.colored_vertex(c, color(0.0));
    for j in 1..=RINGS {
        let t = j as f32 / RINGS as f32;
        let ci = color(t);
        for i in 0..SEGS {
            let a = i as f32 / SEGS as f32 * TAU;
            mesh.colored_vertex(c + vec2(a.cos(), a.sin()) * r * t, ci);
        }
    }
    let ring = |j: usize| 1 + (j - 1) * SEGS;
    for i in 0..SEGS {
        let i2 = (i + 1) % SEGS;
        mesh.add_triangle(0, (ring(1) + i) as u32, (ring(1) + i2) as u32);
    }
    for j in 2..=RINGS {
        for i in 0..SEGS {
            let i2 = (i + 1) % SEGS;
            let (a0, b0) = (ring(j - 1), ring(j));
            mesh.add_triangle((a0 + i) as u32, (b0 + i) as u32, (b0 + i2) as u32);
            mesh.add_triangle((a0 + i) as u32, (b0 + i2) as u32, (a0 + i2) as u32);
        }
    }
    p.add(Shape::mesh(mesh));
}

fn add_col(col: [f32; 3], k: f32) -> Color32 {
    Color32::from_rgba_premultiplied(
        (col[0] * k * 255.0).clamp(0.0, 255.0) as u8,
        (col[1] * k * 255.0).clamp(0.0, 255.0) as u8,
        (col[2] * k * 255.0).clamp(0.0, 255.0) as u8,
        0,
    )
}

impl Ambient {
    pub fn new() -> Self {
        let mut rng = Rng(0x9E37_79B9);
        let stars = (0..600)
            .map(|_| {
                let m = rng.next();
                let temp = rng.next();
                Star {
                    x: rng.next(),
                    y: rng.next(),
                    mag: 0.25 + 0.75 * m.powf(3.0),
                    size: 0.5 + 1.6 * rng.next().powf(4.0),
                    freq: 0.6 + 2.2 * rng.next(),
                    phase: rng.next() * TAU,
                    color: if temp < 0.2 {
                        [0.75, 0.85, 1.0]
                    } else if temp > 0.88 {
                        [1.0, 0.85, 0.65]
                    } else {
                        [0.95, 0.95, 1.0]
                    },
                }
            })
            .collect();
        // über die ganze Höhe verteilt, unten etwas dichter
        let flies = (0..30)
            .map(|_| Firefly {
                x: 0.05 + 0.90 * rng.next(),
                y: 0.05 + 0.90 * rng.next().powf(0.8),
                ax: 0.03 + 0.07 * rng.next(),
                ay: 0.02 + 0.05 * rng.next(),
                fx: 0.05 + 0.12 * rng.next(),
                fy: 0.04 + 0.1 * rng.next(),
                blink: 0.12 + 0.18 * rng.next(),
                phase: rng.next() * 10.0,
            })
            .collect();
        Self {
            slots: HashMap::new(),
            stars,
            flies,
            time: 0.0,
            last: Instant::now(),
        }
    }

    /// Zeit weiterlaufen lassen (Tempo-Faktor).
    pub fn advance(&mut self, speed: f32) {
        let dt = self.last.elapsed().as_secs_f32().min(0.5);
        self.last = Instant::now();
        self.time += dt * speed;
        if self.time > 100_000.0 {
            self.time -= 100_000.0;
        }
    }

    /// Zeichnet den Hintergrund in `rect`. `slot` trennt Vollbild und Vorschauen.
    pub fn paint(
        &mut self,
        ctx: &egui::Context,
        p: &Painter,
        rect: Rect,
        params: AmbientParams,
        slot: &'static str,
        long_side: f32,
        corner: u8,
    ) {
        if params.mode == AmbientMode::Off || params.k <= 0.002 {
            return;
        }
        let t = self.time;
        let size = tex_size(rect, long_side);
        let k_key = (params.k * 200.0) as u32;
        let key = (params.mode, size, k_key, params.tint, params.bg);

        // Fläche aktualisieren (höchstens 20x pro Sekunde)
        let needs_new = !self.slots.contains_key(slot);
        if needs_new {
            let img = ColorImage::new(size, Color32::BLACK);
            let tex = ctx.load_texture(format!("ambient-{slot}"), img.clone(), TextureOptions::LINEAR);
            self.slots.insert(
                slot,
                Slot {
                    tex,
                    img,
                    base: Vec::new(),
                    base_key: (AmbientMode::Off, [0, 0]),
                    last_update: Instant::now() - Duration::from_secs(1),
                    key: (AmbientMode::Off, [0, 0], 0, None, [0; 3]),
                },
            );
        }
        let s = self.slots.get_mut(slot).expect("slot");
        let due = s.last_update.elapsed() >= Duration::from_millis(if params.eco { 100 } else { 50 });
        let changed = s.key != key;
        if changed || (is_dynamic(params.mode) && due) {
            let [w, h] = size;
            let mut field: Vec<[f32; 3]> = Vec::new();
            if is_dynamic(params.mode) {
                render_dynamic(params.mode, w, h, t, &mut field);
            } else {
                if s.base_key != (params.mode, size) {
                    render_base(params.mode, w, h, &mut s.base);
                    s.base_key = (params.mode, size);
                }
                field.clone_from(&s.base);
            }
            let tint = params.tint.map(|c| [c[0] as f32 / 255.0, c[1] as f32 / 255.0, c[2] as f32 / 255.0]);
            let bg = [
                params.bg[0] as f32 / 255.0,
                params.bg[1] as f32 / 255.0,
                params.bg[2] as f32 / 255.0,
            ];
            if s.img.size != size {
                s.img = ColorImage::new(size, Color32::BLACK);
            }
            for (px, c) in s.img.pixels.iter_mut().zip(field.iter()) {
                let mut c = *c;
                if let Some(tc) = tint {
                    let l = (0.30 * c[0] + 0.59 * c[1] + 0.11 * c[2]) * 1.9;
                    c = [tc[0] * l, tc[1] * l, tc[2] * l];
                }
                let f = |i: usize| ((bg[i] + c[i] * params.k) * 255.0).clamp(0.0, 255.0) as u8;
                *px = Color32::from_rgb(f(0), f(1), f(2));
            }
            s.tex.set(s.img.clone(), TextureOptions::LINEAR);
            s.last_update = Instant::now();
            s.key = key;
        }
        let uv = Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0));
        p.add(Shape::Rect(
            egui::epaint::RectShape::filled(rect, corner, Color32::WHITE).with_texture(s.tex.id(), uv),
        ));

        // Feine Details als Vektorpunkte
        let clip = p.with_clip_rect(rect.intersect(p.clip_rect()));
        match params.mode {
            AmbientMode::Stars => {
                self.paint_stars(&clip, rect, params, 1.0);
                self.paint_shooting_star(&clip, rect, params);
            }
            AmbientMode::Aurora => self.paint_stars(&clip, rect, params, 0.35),
            AmbientMode::Fireflies => self.paint_fireflies(&clip, rect, params),
            AmbientMode::Lava => self.paint_lava(&clip, rect, params),
            _ => {}
        }
    }

    fn scale(rect: Rect) -> f32 {
        (rect.width().min(rect.height()) / 700.0).clamp(0.35, 2.2)
    }

    fn tinted(&self, col: [f32; 3], params: &AmbientParams) -> [f32; 3] {
        match params.tint {
            Some(tc) => {
                let w = 0.45;
                [
                    col[0] * (1.0 - w) + tc[0] as f32 / 255.0 * w,
                    col[1] * (1.0 - w) + tc[1] as f32 / 255.0 * w,
                    col[2] * (1.0 - w) + tc[2] as f32 / 255.0 * w,
                ]
            }
            None => col,
        }
    }

    fn paint_stars(&self, p: &Painter, rect: Rect, params: AmbientParams, amount: f32) {
        let t = self.time;
        let sc = Self::scale(rect);
        let area = rect.width() * rect.height();
        let cap = if params.eco { 160.0 } else { 320.0 };
        let n = ((area / 4000.0) * amount).clamp(30.0 * amount, cap) as usize;
        let drift = t * 0.0012;
        for s in self.stars.iter().take(n) {
            let x = (s.x + drift).fract();
            let pos = pos2(rect.left() + x * rect.width(), rect.top() + s.y * rect.height());
            let tw = 0.6 + 0.4 * (t * s.freq + s.phase).sin();
            let b = s.mag * tw * params.k * 1.4;
            let col = self.tinted(s.color, &params);
            let r = (0.55 + s.size * 0.9) * sc;
            if s.mag > 0.75 {
                p.circle_filled(pos, r * 3.2, add_col(col, b * 0.10));
            }
            p.circle_filled(pos, r, add_col(col, b));
        }
    }

    fn paint_shooting_star(&self, p: &Painter, rect: Rect, params: AmbientParams) {
        const PERIOD: f32 = 9.0;
        const LIFE: f32 = 1.4;
        let t = self.time;
        let k = (t / PERIOD).floor();
        let local = t - k * PERIOD;
        let mut rng = Rng(0x1234_5678 ^ (k as i64 as u32).wrapping_mul(2_654_435_761) | 1);
        if rng.next() > 0.65 || local > LIFE {
            return;
        }
        let sc = Self::scale(rect);
        let start = pos2(
            rect.left() + rect.width() * (0.1 + 0.8 * rng.next()),
            rect.top() + rect.height() * (0.05 + 0.35 * rng.next()),
        );
        let ang = 0.35 + 0.45 * rng.next();
        let dir_x = if rng.next() < 0.5 { 1.0 } else { -1.0 };
        let dir = vec2(ang.cos() * dir_x, ang.sin());
        let len = rect.width().max(rect.height()) * 0.35;
        let prog = local / LIFE;
        let fade = (prog * 6.0).min(1.0) * ((1.0 - prog) * 3.0).min(1.0);
        let head = start + dir * len * prog;
        let col = self.tinted([1.0, 0.97, 0.9], &params);
        const SEG: usize = 14;
        let trail = len * 0.28;
        for i in 0..SEG {
            let a = head - dir * trail * (i as f32 / SEG as f32);
            let b = head - dir * trail * ((i + 1) as f32 / SEG as f32);
            let f = (1.0 - i as f32 / SEG as f32).powi(2);
            p.line_segment([a, b], Stroke::new(1.6 * sc * f + 0.4, add_col(col, params.k * 1.6 * f * fade)));
        }
        p.circle_filled(head, 1.8 * sc, add_col(col, params.k * 1.8 * fade));
    }

    /// Wachs als glatte Vektorfläche (Marching Squares) mit weicher Schattierung.
    fn paint_lava(&self, p: &Painter, rect: Rect, params: AmbientParams) {
        let (w, h) = (rect.width(), rect.height());
        if w < 4.0 || h < 4.0 {
            return;
        }
        let balls = lava_balls(self.time, w / h);
        let max_cells = if params.eco { 4500.0 } else { 11000.0 };
        let mut cell = (w.min(h) / 64.0).max(2.5);
        while (w / cell).ceil() * (h / cell).ceil() > max_cells {
            cell *= 1.12;
        }
        let nx = (w / cell).ceil() as usize;
        let ny = (h / cell).ceil() as usize;
        let mut vals = Vec::with_capacity((nx + 1) * (ny + 1));
        for j in 0..=ny {
            for i in 0..=nx {
                let (px, py) = ((i as f32 * cell).min(w), (j as f32 * cell).min(h));
                vals.push(lava_field(&balls, px / h, py / h));
            }
        }
        let tint = params.tint.map(|c| [c[0] as f32 / 255.0, c[1] as f32 / 255.0, c[2] as f32 / 255.0]);
        let bg = params.bg;
        let k = params.k;
        let color = |py: f32, v: (f32, f32, f32)| {
            let mut c = wax_color(v.0, py / h, v.1, v.2);
            if let Some(tc) = tint {
                let l = (0.30 * c[0] + 0.59 * c[1] + 0.11 * c[2]) * 1.9;
                c = [tc[0] * l, tc[1] * l, tc[2] * l];
            }
            let ch = |i: usize| (bg[i] as f32 + c[i] * k * 255.0).clamp(0.0, 255.0) as u8;
            Color32::from_rgb(ch(0), ch(1), ch(2))
        };
        let at = |px: f32, py: f32| pos2(rect.left() + px, rect.top() + py);

        let mut mesh = Mesh::default();
        let mut contour: Vec<(Pos2, Pos2, Color32)> = Vec::new();
        let mut poly: Vec<(Pos2, Color32, bool)> = Vec::with_capacity(8);
        for j in 0..ny {
            for i in 0..nx {
                let corners = [(i, j), (i + 1, j), (i + 1, j + 1), (i, j + 1)];
                let cv = corners.map(|(a, b)| vals[b * (nx + 1) + a]);
                let inside = cv.map(|v| v.0 >= LAVA_T);
                if !inside.iter().any(|b| *b) {
                    continue;
                }
                let cp = corners.map(|(a, b)| ((a as f32 * cell).min(w), (b as f32 * cell).min(h)));
                poly.clear();
                for c in 0..4 {
                    let c2 = (c + 1) % 4;
                    if inside[c] {
                        poly.push((at(cp[c].0, cp[c].1), color(cp[c].1, cv[c]), false));
                    }
                    if inside[c] != inside[c2] {
                        let tt = ((LAVA_T - cv[c].0) / (cv[c2].0 - cv[c].0)).clamp(0.0, 1.0);
                        let ex = cp[c].0 + (cp[c2].0 - cp[c].0) * tt;
                        let ey = cp[c].1 + (cp[c2].1 - cp[c].1) * tt;
                        let fv = lava_field(&balls, ex / h, ey / h);
                        poly.push((at(ex, ey), color(ey, fv), true));
                    }
                }
                let m = poly.len();
                for a in 0..m {
                    let b = (a + 1) % m;
                    if poly[a].2 && poly[b].2 {
                        contour.push((poly[a].0, poly[b].0, poly[a].1));
                    }
                }
                let base = mesh.vertices.len() as u32;
                for (pos, col, _) in &poly {
                    mesh.colored_vertex(*pos, *col);
                }
                for a in 1..m.saturating_sub(1) {
                    mesh.add_triangle(base, base + a as u32, base + a as u32 + 1);
                }
            }
        }
        p.add(Shape::mesh(mesh));
        // Kanten glätten (egui-Feathering)
        let px = 1.0 / p.ctx().pixels_per_point();
        for (a, b, c) in contour {
            p.line_segment([a, b], Stroke::new(px * 1.5, c));
        }
    }

    fn paint_fireflies(&self, p: &Painter, rect: Rect, params: AmbientParams) {
        let t = self.time;
        let sc = Self::scale(rect);
        let n = if rect.width() * rect.height() < 60_000.0 { 8 } else { self.flies.len() };
        for f in self.flies.iter().take(n) {
            let x = f.x + f.ax * (t * f.fx + f.phase).sin() + f.ax * 0.4 * (t * f.fx * 2.3).cos();
            let y = f.y + f.ay * (t * f.fy + f.phase * 1.3).cos();
            let pos = pos2(rect.left() + x * rect.width(), rect.top() + y * rect.height());
            let c = (t * f.blink + f.phase).fract();
            let pulse = if c < 0.35 { (c / 0.35 * PI).sin() } else { 0.0 };
            let b = (0.12 + 0.88 * pulse) * params.k;
            let col = self.tinted([0.80, 1.0, 0.35], &params);
            soft_dot(p, pos, 46.0 * sc, col, b * 2.6);
            soft_dot(p, pos, 14.0 * sc, col, b * 2.4);
            p.circle_filled(pos, 2.6 * sc, add_col(col, b * 1.6));
        }
    }
}

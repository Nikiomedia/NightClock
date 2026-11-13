//! Vektor-Schrift für die großen Ziffern: Glyphen-Umrisse werden direkt aus der
//! Schriftdatei gelesen und trianguliert. Dadurch sind die Ziffern in jeder Größe
//! gestochen scharf (auch auf 4K) und belegen keinen Platz im Schrift-Atlas.
//! Variable Schriften (z. B. Bahnschrift) erlauben eine frei wählbare Schriftstärke.

use egui::{pos2, Color32, Mesh, Painter, Pos2, Rect, Shape, Stroke, Vec2};
use lyon_tessellation::geom::point;
use lyon_tessellation::path::iterator::PathIterator;
use lyon_tessellation::path::{Path, PathEvent};
use lyon_tessellation::{BuffersBuilder, FillOptions, FillRule, FillTessellator, FillVertex, VertexBuffers};
use std::collections::HashMap;
use std::sync::Arc;

/// Interne Auflösung: 1000 Einheiten pro Geviert
const UNITS: f32 = 1000.0;
const TOLERANCE: f32 = 0.35;

pub struct GlyphGeom {
    /// Vorschubbreite in Geviert-Einheiten (1.0 = Schriftgröße)
    pub adv: f32,
    /// Tintenrechteck relativ zur Grundlinie (y nach unten)
    pub ink: Rect,
    verts: Vec<Pos2>,
    indices: Vec<u32>,
    contours: Vec<Vec<Pos2>>,
}

pub struct VectorFont {
    data: Arc<Vec<u8>>,
    index: u32,
    weight: Option<f32>,
    glyphs: HashMap<char, Option<GlyphGeom>>,
    axis: Option<(f32, f32, f32)>,
    pub valid: bool,
}

struct OutlineToPath {
    b: lyon_tessellation::path::path::Builder,
    scale: f32,
    open: bool,
}

impl OutlineToPath {
    fn p(&self, x: f32, y: f32) -> lyon_tessellation::math::Point {
        point(x * self.scale, -y * self.scale)
    }
}

impl ttf_parser::OutlineBuilder for OutlineToPath {
    fn move_to(&mut self, x: f32, y: f32) {
        if self.open {
            self.b.end(true);
        }
        let p = self.p(x, y);
        self.b.begin(p);
        self.open = true;
    }
    fn line_to(&mut self, x: f32, y: f32) {
        let p = self.p(x, y);
        self.b.line_to(p);
    }
    fn quad_to(&mut self, x1: f32, y1: f32, x: f32, y: f32) {
        let (c, p) = (self.p(x1, y1), self.p(x, y));
        self.b.quadratic_bezier_to(c, p);
    }
    fn curve_to(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, x: f32, y: f32) {
        let (c1, c2, p) = (self.p(x1, y1), self.p(x2, y2), self.p(x, y));
        self.b.cubic_bezier_to(c1, c2, p);
    }
    fn close(&mut self) {
        if self.open {
            self.b.end(true);
            self.open = false;
        }
    }
}

fn wght_tag() -> ttf_parser::Tag {
    ttf_parser::Tag::from_bytes(b"wght")
}

impl VectorFont {
    /// Lädt die Schrift; bei Fehlern wird die eingebaute egui-Schrift verwendet.
    pub fn load(path: &str, index: u32, weight: Option<f32>) -> Self {
        let from_file = (!path.is_empty())
            .then(|| std::fs::read(path).ok())
            .flatten()
            .filter(|d| ttf_parser::Face::parse(d, index).is_ok());
        let (data, index) = match from_file {
            Some(d) => (d, index),
            None => {
                let defs = egui::FontDefinitions::default();
                let d = defs
                    .font_data
                    .get("Ubuntu-Light")
                    .map(|f| f.font.to_vec())
                    .unwrap_or_default();
                (d, 0)
            }
        };
        let mut vf = Self {
            data: Arc::new(data),
            index,
            weight,
            glyphs: HashMap::new(),
            axis: None,
            valid: false,
        };
        if let Ok(face) = ttf_parser::Face::parse(&vf.data, vf.index) {
            vf.valid = true;
            vf.axis = face
                .variation_axes()
                .into_iter()
                .find(|a| a.tag == wght_tag())
                .map(|a| (a.min_value, a.def_value, a.max_value));
        }
        vf
    }

    /// (min, standard, max) der Schriftstärke, falls die Schrift variabel ist.
    pub fn weight_axis(&self) -> Option<(f32, f32, f32)> {
        self.axis
    }

    pub fn glyph(&mut self, c: char) -> Option<&GlyphGeom> {
        if !self.glyphs.contains_key(&c) {
            let g = self.build(c);
            self.glyphs.insert(c, g);
        }
        self.glyphs.get(&c).and_then(|g| g.as_ref())
    }

    fn build(&self, c: char) -> Option<GlyphGeom> {
        let mut face = ttf_parser::Face::parse(&self.data, self.index).ok()?;
        if let (Some(w), Some((min, _, max))) = (self.weight, self.axis) {
            face.set_variation(wght_tag(), w.clamp(min, max));
        }
        let gid = face.glyph_index(c)?;
        let upem = face.units_per_em() as f32;
        let scale = UNITS / upem;
        let adv = face.glyph_hor_advance(gid).unwrap_or(0) as f32 / upem;

        let mut b = OutlineToPath {
            b: Path::builder(),
            scale,
            open: false,
        };
        face.outline_glyph(gid, &mut b);
        if b.open {
            b.b.end(true);
        }
        let path = b.b.build();

        // Umrisse (für Kantenglättung)
        let mut contours: Vec<Vec<Pos2>> = Vec::new();
        let mut cur: Vec<Pos2> = Vec::new();
        let mut min = pos2(f32::MAX, f32::MAX);
        let mut max = pos2(f32::MIN, f32::MIN);
        for evt in path.iter().flattened(TOLERANCE) {
            match evt {
                PathEvent::Begin { at } => {
                    cur.clear();
                    cur.push(pos2(at.x / UNITS, at.y / UNITS));
                }
                PathEvent::Line { to, .. } => cur.push(pos2(to.x / UNITS, to.y / UNITS)),
                PathEvent::End { .. } => {
                    if cur.len() > 2 {
                        for p in &cur {
                            min = min.min(*p);
                            max = max.max(*p);
                        }
                        contours.push(std::mem::take(&mut cur));
                    }
                }
                _ => {}
            }
        }

        // Füllung triangulieren
        let mut buffers: VertexBuffers<Pos2, u32> = VertexBuffers::new();
        let mut tess = FillTessellator::new();
        let ok = tess
            .tessellate_path(
                &path,
                &FillOptions::tolerance(TOLERANCE).with_fill_rule(FillRule::NonZero),
                &mut BuffersBuilder::new(&mut buffers, |v: FillVertex| {
                    let p = v.position();
                    pos2(p.x / UNITS, p.y / UNITS)
                }),
            )
            .is_ok();
        if !ok {
            buffers.vertices.clear();
            buffers.indices.clear();
        }

        let ink = if contours.is_empty() {
            Rect::from_min_max(pos2(0.0, -0.7), pos2(adv, 0.0))
        } else {
            Rect::from_min_max(min, max)
        };
        Some(GlyphGeom {
            adv,
            ink,
            verts: buffers.vertices,
            indices: buffers.indices,
            contours,
        })
    }
}

impl GlyphGeom {
    /// Zeichnet die Glyphe; `origin` = linker Punkt auf der Grundlinie.
    pub fn draw(&self, p: &Painter, origin: Pos2, size: f32, color: Color32) {
        if self.verts.is_empty() {
            return;
        }
        let mut mesh = Mesh::default();
        mesh.vertices.reserve(self.verts.len());
        for v in &self.verts {
            mesh.colored_vertex(origin + v.to_vec2() * size, color);
        }
        mesh.indices = self.indices.clone();
        p.add(Shape::mesh(mesh));
        // Kanten mit egui-Feathering glätten
        let px = 1.0 / p.ctx().pixels_per_point();
        for c in &self.contours {
            let pts: Vec<Pos2> = c.iter().map(|v| origin + v.to_vec2() * size).collect();
            p.add(Shape::closed_line(pts, Stroke::new(px, color)));
        }
    }

    /// Nur die Füllung (für Leuchteffekt-Kopien, ohne Kantenglättung).
    pub fn draw_fill(&self, p: &Painter, origin: Pos2, size: f32, color: Color32) {
        if self.verts.is_empty() {
            return;
        }
        let mut mesh = Mesh::default();
        mesh.vertices.reserve(self.verts.len());
        for v in &self.verts {
            mesh.colored_vertex(origin + v.to_vec2() * size, color);
        }
        mesh.indices = self.indices.clone();
        p.add(Shape::mesh(mesh));
    }

    /// Weicher Lichtschein: viele leicht versetzte, additive Kopien.
    pub fn draw_glow(&self, p: &Painter, origin: Pos2, size: f32, glow: impl Fn(f32) -> Color32) {
        const RINGS: [(f32, f32); 4] = [(0.005, 0.30), (0.013, 0.35), (0.030, 0.25), (0.058, 0.16)];
        const N: usize = 12;
        for (ri, &(r, w)) in RINGS.iter().enumerate() {
            let col = glow(w * 0.14);
            if col.r() == 0 && col.g() == 0 && col.b() == 0 {
                continue;
            }
            for i in 0..N {
                let a = (i as f32 + 0.5 * ri as f32) / N as f32 * std::f32::consts::TAU;
                let off = Vec2::new(a.cos(), a.sin()) * (r * size).max(1.0);
                self.draw_fill(p, origin + off, size, col);
            }
        }
    }
}

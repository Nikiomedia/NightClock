//! Prozedural gezeichnetes App-Icon: eine leuchtende Bernstein-Mondsichel
//! mit kleinem Stern auf einer nachtblauen Kachel.
//!
//! Wird sowohl zur Laufzeit (Fenster-/Taskleisten-Icon) als auch vom
//! Build-Skript (eingebettetes .ico der exe) verwendet.

#![allow(dead_code)]

/// Rendert das Icon als RGBA-Pixel (nicht vormultipliert), `size` x `size`.
pub fn render_icon(size: u32) -> Vec<u8> {
    const SS: u32 = 4; // Supersampling pro Achse
    let n = size as f32;
    let mut out = vec![0u8; (size * size * 4) as usize];

    for y in 0..size {
        for x in 0..size {
            let mut acc = [0f32; 4]; // vormultipliert
            for sy in 0..SS {
                for sx in 0..SS {
                    let u = (x as f32 + (sx as f32 + 0.5) / SS as f32) / n;
                    let v = (y as f32 + (sy as f32 + 0.5) / SS as f32) / n;
                    let [r, g, b, a] = shade(u, v);
                    acc[0] += r * a;
                    acc[1] += g * a;
                    acc[2] += b * a;
                    acc[3] += a;
                }
            }
            let cnt = (SS * SS) as f32;
            let a = acc[3] / cnt;
            let i = ((y * size + x) * 4) as usize;
            if a > 0.0 {
                out[i] = to_u8(acc[0] / cnt / a);
                out[i + 1] = to_u8(acc[1] / cnt / a);
                out[i + 2] = to_u8(acc[2] / cnt / a);
                out[i + 3] = to_u8(a);
            }
        }
    }
    out
}

fn to_u8(v: f32) -> u8 {
    (v.clamp(0.0, 1.0) * 255.0 + 0.5) as u8
}

fn len(x: f32, y: f32) -> f32 {
    (x * x + y * y).sqrt()
}

/// Abgerundetes Rechteck als Signed-Distance-Feld.
fn sd_round_box(px: f32, py: f32, half: f32, r: f32) -> f32 {
    let qx = px.abs() - half + r;
    let qy = py.abs() - half + r;
    len(qx.max(0.0), qy.max(0.0)) + qx.max(qy).min(0.0) - r
}

fn shade(u: f32, v: f32) -> [f32; 4] {
    // Kachel
    let d_tile = sd_round_box(u - 0.5, v - 0.5, 0.47, 0.2);
    if d_tile > 0.0 {
        return [0.0; 4];
    }

    // Hintergrund: tiefes Nachtblau mit leichtem Verlauf
    let top = [0.075, 0.085, 0.19];
    let bottom = [0.015, 0.018, 0.055];
    let mut c = [
        top[0] + (bottom[0] - top[0]) * v,
        top[1] + (bottom[1] - top[1]) * v,
        top[2] + (bottom[2] - top[2]) * v,
    ];

    // Mondsichel = Kreis A minus Kreis B
    let (ax, ay, ar) = (0.46, 0.53, 0.27);
    let (bx, by, br) = (0.585, 0.415, 0.235);
    let da = len(u - ax, v - ay) - ar;
    let db = len(u - bx, v - by) - br;
    let crescent = da.max(-db);

    let amber = [1.0, 0.70, 0.30];

    // weiches Leuchten um die Sichel
    let glow = (-crescent.max(0.0) * 13.0).exp() * 0.55;
    for k in 0..3 {
        c[k] += amber[k] * glow * 0.6;
    }

    // Sichel selbst, zur Kante hin heller
    if crescent < 0.0 {
        let edge = (1.0 + crescent * 9.0).clamp(0.0, 1.0);
        let moon = [
            amber[0],
            amber[1] + 0.18 * edge,
            amber[2] + 0.25 * edge,
        ];
        c = moon;
    }

    // kleiner vierzackiger Stern
    let (sx, sy) = (u - 0.75, v - 0.27);
    let star = sx.abs().sqrt() + sy.abs().sqrt();
    if star < 0.255 {
        c = [1.0, 0.93, 0.80];
    } else {
        let sg = (-(star - 0.255) * 18.0).exp() * 0.35;
        for k in 0..3 {
            c[k] += sg;
        }
    }

    [c[0].min(1.0), c[1].min(1.0), c[2].min(1.0), 1.0]
}

/// Erzeugt eine .ico-Datei (BMP-Einträge, 32 Bit) mit mehreren Größen.
pub fn encode_ico(sizes: &[u32]) -> Vec<u8> {
    let mut images: Vec<Vec<u8>> = Vec::new();
    for &s in sizes {
        let rgba = render_icon(s);
        let mask_row = ((s + 31) / 32 * 4) as usize;
        let mut img = Vec::new();
        // BITMAPINFOHEADER
        img.extend_from_slice(&40u32.to_le_bytes());
        img.extend_from_slice(&(s as i32).to_le_bytes());
        img.extend_from_slice(&((s * 2) as i32).to_le_bytes());
        img.extend_from_slice(&1u16.to_le_bytes());
        img.extend_from_slice(&32u16.to_le_bytes());
        img.extend_from_slice(&0u32.to_le_bytes());
        let size_image = s * s * 4 + mask_row as u32 * s;
        img.extend_from_slice(&size_image.to_le_bytes());
        img.extend_from_slice(&0i32.to_le_bytes());
        img.extend_from_slice(&0i32.to_le_bytes());
        img.extend_from_slice(&0u32.to_le_bytes());
        img.extend_from_slice(&0u32.to_le_bytes());
        // Pixel von unten nach oben, BGRA
        for y in (0..s).rev() {
            for x in 0..s {
                let i = ((y * s + x) * 4) as usize;
                img.extend_from_slice(&[rgba[i + 2], rgba[i + 1], rgba[i], rgba[i + 3]]);
            }
        }
        // AND-Maske (leer, Alpha wird verwendet)
        img.extend(std::iter::repeat(0u8).take(mask_row * s as usize));
        images.push(img);
    }

    let mut out = Vec::new();
    out.extend_from_slice(&0u16.to_le_bytes());
    out.extend_from_slice(&1u16.to_le_bytes());
    out.extend_from_slice(&(sizes.len() as u16).to_le_bytes());
    let mut offset = 6 + 16 * sizes.len() as u32;
    for (i, &s) in sizes.iter().enumerate() {
        let dim = if s >= 256 { 0u8 } else { s as u8 };
        out.push(dim);
        out.push(dim);
        out.push(0);
        out.push(0);
        out.extend_from_slice(&1u16.to_le_bytes());
        out.extend_from_slice(&32u16.to_le_bytes());
        out.extend_from_slice(&(images[i].len() as u32).to_le_bytes());
        out.extend_from_slice(&offset.to_le_bytes());
        offset += images[i].len() as u32;
    }
    for img in images {
        out.extend_from_slice(&img);
    }
    out
}

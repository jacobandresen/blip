//! The inscription cut round the Warlock's arch on the title: "JACOB IS
//! AWESOME" in the Elder Futhark (ᛃᚨᚲᛟᛒ ᛁᛊ ᚨᚹᛖᛊᛟᛗᛖ). The painting carves the
//! strokes; the game lights them. Shared by title.rs and art.rs.

/// Each letter's strokes, as polylines in a box half a unit wide and a unit
/// tall either side of its centre (y down).
fn glyph(c: char) -> &'static [&'static [(f32, f32)]] {
    match c {
        // jera: two hooks
        'J' => &[&[(-0.05, -0.8), (-0.42, -0.35), (-0.05, 0.05)], &[(0.05, -0.05), (0.42, 0.35), (0.05, 0.8)]],
        // ansuz
        'A' => &[&[(-0.3, -1.0), (-0.3, 1.0)], &[(-0.3, -1.0), (0.32, -0.6)], &[(-0.3, -0.45), (0.32, -0.05)]],
        // kaunan
        'C' => &[&[(0.3, -0.55), (-0.3, 0.0), (0.3, 0.55)]],
        // othala
        'O' => &[&[(0.0, -1.0), (0.42, -0.4), (-0.42, 1.0)], &[(0.0, -1.0), (-0.42, -0.4), (0.42, 1.0)]],
        // berkanan
        'B' => &[&[(-0.3, -1.0), (-0.3, 1.0)], &[(-0.3, -1.0), (0.3, -0.5), (-0.3, 0.0), (0.3, 0.5), (-0.3, 1.0)]],
        // isaz
        'I' => &[&[(0.0, -1.0), (0.0, 1.0)]],
        // sowilo
        'S' => &[&[(0.3, -1.0), (-0.3, -0.15), (0.3, 0.15), (-0.3, 1.0)]],
        // wunjo
        'W' => &[&[(-0.3, -1.0), (-0.3, 1.0)], &[(-0.3, -1.0), (0.32, -0.5), (-0.3, 0.0)]],
        // ehwaz
        'E' => &[&[(-0.4, 1.0), (-0.4, -1.0), (0.0, -0.25), (0.4, -1.0), (0.4, 1.0)]],
        // mannaz
        'M' => &[&[(-0.4, -1.0), (-0.4, 1.0)], &[(0.4, -1.0), (0.4, 1.0)], &[(-0.4, -1.0), (0.4, 0.05)], &[(0.4, -1.0), (-0.4, 0.05)]],
        // the word divider: two points
        _ => &[&[(0.0, -0.35), (0.0, -0.2)], &[(0.0, 0.2), (0.0, 0.35)]],
    }
}

const TEXT: &str = "JACOB:IS:AWESOME";
const CX: f32 = 396.0;
const CY: f32 = 176.0;
const R: f32 = 89.0;
const SIZE: f32 = 6.0;

/// Every stroke of the inscription in picture pixels: (letter index, from, to).
pub fn strokes() -> Vec<(usize, (f32, f32), (f32, f32))> {
    let n = TEXT.len();
    let (a0, a1) = (std::f32::consts::PI + 0.16, std::f32::consts::TAU - 0.5);
    let mut out = Vec::new();
    for (i, c) in TEXT.chars().enumerate() {
        let a = a0 + (a1 - a0) * i as f32 / (n - 1) as f32;
        let (cx, cy) = (CX + a.cos() * R, CY + a.sin() * R);
        // the letter stands with its head toward the outside of the arch
        let up = (a.cos(), a.sin());
        let right = (-up.1, up.0);
        let at = |p: (f32, f32)| (cx + right.0 * p.0 * SIZE + up.0 * -p.1 * SIZE, cy + right.1 * p.0 * SIZE + up.1 * -p.1 * SIZE);
        for line in glyph(c) {
            for w in line.windows(2) {
                out.push((i, at(w[0]), at(w[1])));
            }
        }
    }
    out
}

/// How many letters (and dividers) the inscription has.
#[allow(dead_code)]
pub fn letters() -> usize {
    TEXT.len()
}

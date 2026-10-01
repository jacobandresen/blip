//! The last level flies in over land. It starts at sea off the carrier like
//! every level; after COAST_DIST of sea the coast comes up, its first stretch
//! inland thick with flak batteries, then farmland, and from where the last
//! boss appears, a city.
//!
//! Distances here are "inland": pixels from the coastline, growing as the
//! world scrolls down the screen. A point `d` inland is drawn at
//! `coast_y - d`.

use super::*;

/// Pixels of sea scrolled before the coast reaches the top of the screen
/// (about 25 s at cruise).
pub const COAST_DIST: f32 = 1800.0;
const BEACH: f32 = 12.0;
const FIELD: f32 = 44.0;
const CITY_BLOCK: f32 = 64.0;
const STREET: f32 = 9.0;

/// A flak battery on land: one gun in a sandbag pit, shootable.
#[derive(Copy, Clone)]
pub struct Battery { pub x: f32, pub d: f32, pub gun: FlakGun, pub hp: i32 }

const BATTERY_HP: i32 = 4;
const BATTERY_SCORE: i32 = 80;
/// The coast belt is dense, inland sparse; land guns fire less often than
/// an island's, since several share the sky.
const BELT: (f32, f32) = (40.0, 720.0);
const BELT_GUNS: usize = 16;
const INLAND_GAP: f32 = 300.0;
const INLAND_END: f32 = 16_000.0;
const LAND_RELOAD: (f32, f32) = (3.4, 5.0);

pub fn is_land_level(g: &Game) -> bool { g.sess.level == MAX_LEVEL }

/// Screen y of the coastline (land above it), when the coast is in this level.
pub fn coast_y(g: &Game) -> Option<f32> {
    is_land_level(g).then(|| g.land_dist - COAST_DIST)
}

/// The shoreline is not ruled straight: a gentle wiggle across the screen.
fn wiggle(x: f32) -> f32 { (x * 0.021).sin() * 10.0 + (x * 0.053 + 1.3).sin() * 5.0 }

/// Is (`x`, `y`) on screen over land?
pub fn over_land(g: &Game, x: f32, y: f32) -> bool {
    coast_y(g).is_some_and(|cy| y < cy + wiggle(x))
}

/// The batteries for a fresh land level, laid out once.
pub fn lay_batteries() -> Vec<Battery> {
    let gun = |k: usize| FlakGun { angle: 0.0, reload: 1.0 + (k % 5) as f32 * 0.6, flash: 0.0 };
    let mut v = Vec::new();
    for k in 0..BELT_GUNS {
        let d = BELT.0 + (BELT.1 - BELT.0) * (k as f32 + rand01() * 0.6) / BELT_GUNS as f32;
        v.push(Battery { x: 24.0 + rand01() * (WIN_W as f32 - 48.0), d, gun: gun(k), hp: BATTERY_HP });
    }
    let mut d = BELT.1 + INLAND_GAP;
    let mut k = BELT_GUNS;
    while d < INLAND_END {
        v.push(Battery { x: 24.0 + rand01() * (WIN_W as f32 - 48.0), d, gun: gun(k), hp: BATTERY_HP });
        d += INLAND_GAP * (0.7 + rand01() * 0.6);
        k += 1;
    }
    v
}

fn battery_pos(cy: f32, b: &Battery) -> (f32, f32) { (b.x, cy - b.d) }

/// Batteries in the upper part of the screen, ahead of the plane, fire.
pub fn update_batteries(g: &mut Game, dt: f32, sfx: &Sounds) {
    let Some(cy) = coast_y(g) else { return };
    let (px, py) = (g.player_x + PLAYER_W as f32 / 2.0, g.player_y + PLAYER_H as f32 / 2.0);
    for i in 0..g.batteries.len() {
        let (x, y) = battery_pos(cy, &g.batteries[i]);
        let b = &mut g.batteries[i];
        if b.hp <= 0 || y < 10.0 || y > py - 60.0 { continue; }
        if aim_flak_gun(&mut b.gun, x, y, px, py, dt, LAND_RELOAD) {
            fire_flak(g, x, y, sfx);
        }
    }
}

/// A player round at (`x`, `y`) hitting a battery: true if it was used up.
pub fn hit_battery(g: &mut Game, x: f32, y: f32, sfx: &Sounds) -> bool {
    let Some(cy) = coast_y(g) else { return false };
    for i in 0..g.batteries.len() {
        let (bx, by) = battery_pos(cy, &g.batteries[i]);
        if g.batteries[i].hp <= 0 || (bx - x).hypot(by - y) > 10.0 { continue; }
        g.batteries[i].hp -= 1;
        if g.batteries[i].hp <= 0 {
            g.spawn_explosion(bx, by, 1.2, EXPLOSION_ORANGE);
            g.score_at(bx, by, BATTERY_SCORE * g.sess.level);
            play_take(&sfx.enemy_explode, 0.7);
        } else {
            g.spawn_explosion(x, y, 0.4, EXPLOSION_ORANGE);
        }
        return true;
    }
    false
}

/// A cheap repeatable hash: 0..1 for a grid cell.
fn hash(a: i32, b: i32) -> f32 {
    let n = (a.wrapping_mul(374_761_393) ^ b.wrapping_mul(668_265_263)).wrapping_mul(1_274_126_177);
    ((n >> 8) & 0xFFFF) as f32 / 65_535.0
}

const FIELDS: [(f32, f32, f32); 6] = [
    (0.30, 0.44, 0.20), (0.40, 0.50, 0.24), (0.50, 0.46, 0.28),
    (0.34, 0.40, 0.21), (0.52, 0.52, 0.30), (0.42, 0.54, 0.34),
];

/// The land above the coastline: fields with hedgerows and roads, the city
/// from `g.city_from` inland, and the beach and surf along the shore.
pub fn draw_land(blip: &Blip, g: &Game) {
    let Some(cy) = coast_y(g) else { return };
    if cy + 20.0 < 0.0 { return; }
    let w = WIN_W as f32;
    let d_top = cy;                       // inland distance at the top edge
    let d_low = (cy - WIN_H as f32).max(0.0);
    let city = g.city_from.unwrap_or(f32::MAX);

    // farmland, row by row of fields
    let r0 = (d_low / FIELD) as i32;
    let r1 = (d_top / FIELD) as i32 + 1;
    for r in r0..=r1 {
        let (d0, d1) = (r as f32 * FIELD, (r + 1) as f32 * FIELD);
        if d0 >= city { break; }
        let (y0, y1) = (cy - d1, cy - d0);
        let shift = hash(r, 7) * FIELD;
        let mut x = -shift;
        let mut c = 0;
        while x < w {
            let fw = FIELD * (0.8 + hash(r, c) * 1.2);
            let (fr, fg, fb) = FIELDS[(hash(c, r) * FIELDS.len() as f32) as usize % FIELDS.len()];
            blip.fill_rect(x, y0, fw + 1.0, y1 - y0 + 1.0, BlipColor::new(fr, fg, fb, 1.0));
            // hedgerow along the field's edge
            blip.fill_rect(x, y0, 1.5, y1 - y0, BlipColor::new(0.18, 0.26, 0.12, 0.8));
            x += fw;
            c += 1;
        }
        blip.fill_rect(0.0, y0, w, 1.5, BlipColor::new(0.18, 0.26, 0.12, 0.8));
        // a road every few rows
        if r % 5 == 2 {
            blip.fill_rect(0.0, y0 + FIELD * 0.45, w, 4.0, BlipColor::new(0.55, 0.52, 0.46, 1.0));
        }
    }

    // the city
    if city < d_top {
        let c_low = city.max(d_low);
        blip.fill_rect(0.0, cy - d_top, w, d_top - c_low, BlipColor::new(0.30, 0.30, 0.32, 1.0));
        let b0 = (c_low / CITY_BLOCK) as i32;
        let b1 = (d_top / CITY_BLOCK) as i32 + 1;
        for r in b0..=b1 {
            let d0 = r as f32 * CITY_BLOCK;
            if d0 + CITY_BLOCK < city { continue; }
            for c in 0..(w / CITY_BLOCK) as i32 + 1 {
                let (bx, by) = (c as f32 * CITY_BLOCK + STREET / 2.0, cy - d0 - CITY_BLOCK + STREET / 2.0);
                let inner = CITY_BLOCK - STREET;
                if hash(r, c + 50) < 0.12 {
                    // a park
                    blip.fill_rect(bx, by, inner, inner, BlipColor::new(0.28, 0.42, 0.22, 1.0));
                    continue;
                }
                // two to four buildings, each a roof with its shadow
                let n = 2 + (hash(c, r + 9) * 3.0) as i32;
                for k in 0..n {
                    let (fx, fy) = (hash(r * 7 + k, c), hash(c * 5 + k, r));
                    let (bw, bh) = (inner * (0.3 + 0.35 * hash(k, r + c)), inner * (0.3 + 0.35 * hash(r + c, k)));
                    let (x, y) = (bx + fx * (inner - bw), by + fy * (inner - bh));
                    blip.fill_rect(x + 3.0, y + 4.0, bw, bh, BlipColor::new(0.10, 0.10, 0.12, 0.6));
                    let tone = hash(r + k, c * 3);
                    let roof = if tone < 0.3 { BlipColor::new(0.55, 0.30, 0.24, 1.0) }
                        else if tone < 0.7 { BlipColor::new(0.52, 0.52, 0.54, 1.0) }
                        else { BlipColor::new(0.36, 0.34, 0.33, 1.0) };
                    blip.fill_rect(x, y, bw, bh, roof);
                    blip.fill_rect(x, y, bw, 1.5, BlipColor::new(1.0, 1.0, 1.0, 0.18));
                }
            }
        }
    }

    // the shore: grass down to the beach, sand, and surf on the sea side
    let step = 6.0;
    let mut x = 0.0;
    while x < w {
        let yc = cy + wiggle(x + step / 2.0);
        if d_low <= 30.0 {
            blip.fill_rect(x, yc - BEACH - 22.0, step + 0.5, 22.0, BlipColor::new(0.34, 0.46, 0.22, 1.0));
        }
        blip.fill_rect(x, yc - BEACH, step + 0.5, BEACH, BlipColor::new(0.82, 0.76, 0.56, 1.0));
        let foam = 0.55 + 0.35 * ((x * 0.3 + g.sea_scroll * 0.08).sin() * 0.5 + 0.5);
        blip.fill_rect(x, yc - 1.0, step + 0.5, 3.0, BlipColor::new(0.95, 0.97, 1.0, foam));
        x += step;
    }

    draw_batteries(blip, g, cy);
}

/// Each battery: sandbags round a gun pit, the barrel on the player; a
/// blackened crater once it is knocked out.
fn draw_batteries(blip: &Blip, g: &Game, cy: f32) {
    for b in &g.batteries {
        let (x, y) = battery_pos(cy, b);
        if y < -12.0 || y > WIN_H as f32 + 12.0 { continue; }
        if b.hp <= 0 {
            blip.fill_circle(x, y, 8.0, BlipColor::new(0.12, 0.10, 0.08, 0.9));
            continue;
        }
        draw_flak_pit(blip, x, y, &b.gun);
    }
}

//! What is on the street: people, bicycles and their riders, cars, and every hazard in its own look.

use super::*;

// ---- what is on the street -------------------------------------------------

pub(super) fn person(blip: &Blip, x: f32, y: f32, coat: [u8; 3], hair: [u8; 3], swing: f32) {
    let (ox, oy) = sun(2.5);
    blip.fill_circle(x + ox, y + oy, 7.0, rgba([20, 20, 30], 0.25));
    blip.fill_rect(x - 8.0, y - 2.0 + swing, 4.0, 4.0, rgb(coat));
    blip.fill_rect(x + 4.0, y - 2.0 - swing, 4.0, 4.0, rgb(coat));
    blip.fill_circle(x, y, 6.0, rgb(coat));
    blip.fill_circle(x, y - 1.0, 4.0, rgb(hair));
}

/// Are bike lamps on: in the blue hour before the sun is up, and in fog (the law asks for lights in the dark).
pub fn lamps_on(g: &Game) -> bool {
    daylight(g) < 0.45 || g.weather() == Weather::Fog
}

/// A bicycle as Copenhagen rides them, seen from above: tyres showing past the mudguards, a thin frame, the crank
/// and pedals, a saddle, black swept-back bars with grips and a bell, a rack (or a wicker basket on the front), and
/// lamps, lit white ahead and red behind when `lit`. `dir` is +1 riding down the screen, -1 up; `wheelbase` is
/// axle to axle; `bars` how far ahead of `y` the bars are; `steer` turns the front wheel and bars.
#[allow(clippy::too_many_arguments)]
pub(super) fn bicycle(blip: &Blip, x: f32, y: f32, dir: f32, wheelbase: f32, bars: f32, steer: f32, frame: [u8; 3], basket: bool, lit: bool) {
    let (front, rear) = (y + dir * wheelbase / 2.0, y - dir * wheelbase / 2.0);
    let bars_y = y + dir * bars;
    // the shadow: the wheels' line and the bars, a little to the side
    let shade = rgba([16, 16, 24], 0.28);
    let (ox, oy) = sun(2.0);
    blip.fill_rect(x - 1.5 + ox, y - wheelbase / 2.0 - 6.0 + oy, 3.0, wheelbase + 12.0, shade);
    blip.fill_rect(x + steer - 6.5 + ox * 1.5, bars_y + oy * 1.5, 13.0, 2.0, shade);
    let tyre = rgb([20, 20, 24]);
    let guard = rgb(shade_u(frame, 0.75));
    for (wx, wy) in [(x, rear), (x + steer, front)] {
        blip.fill_rect(wx - 1.5, wy - 6.5, 3.0, 13.0, tyre);
        blip.fill_rect(wx - 1.0, wy - 4.0, 2.0, 8.0, guard);
    }
    blip.draw_line_ex(x, rear + dir * 3.0, x + steer * 0.6, bars_y, 2.5, rgb(frame));
    // the crank and pedals, turning
    let crank = y - dir * 1.0;
    blip.fill_circle(x, crank, 1.8, rgb([150, 150, 156]));
    // rack and rear lamp, or a basket in front
    blip.fill_rect(x - 2.0, rear - 3.5, 4.0, 7.0, rgb([46, 46, 52]));
    blip.fill_rect(x - 1.0, rear - 3.0, 2.0, 6.0, rgb([90, 90, 98]));
    if basket {
        blip.fill_rect(x + steer - 4.0, front - 4.0, 8.0, 7.0, rgb([150, 108, 56]));
        blip.fill_rect(x + steer - 3.0, front - 3.0, 6.0, 5.0, rgb([196, 150, 88]));
        blip.fill_rect(x + steer - 3.0, front - 1.0, 6.0, 1.0, rgb([150, 108, 56]));
    }
    // the saddle, brown leather, nose forward
    blip.fill_rect(x - 1.5, y - dir * 5.0 - 2.0, 3.0, 4.0, rgb([70, 46, 30]));
    blip.fill_rect(x - 0.5, y - dir * 3.0 - 1.0, 1.0, 2.0, rgb([70, 46, 30]));
    // black bars swept back to dark grips, the bell's silver dome on the left
    let ink = rgb([18, 18, 22]);
    let (l, r) = (x + steer - 6.5, x + steer + 6.5);
    blip.draw_line_ex(l + 2.5, bars_y, r - 2.5, bars_y, 1.5, ink);
    blip.draw_line_ex(l + 2.5, bars_y, l, bars_y - dir * 2.5, 1.5, ink);
    blip.draw_line_ex(r - 2.5, bars_y, r, bars_y - dir * 2.5, 1.5, ink);
    blip.fill_circle(x + steer - 3.0, bars_y + dir * 0.5, 1.3, rgb([222, 226, 232]));
    // lamps: white on the front mudguard, red on the rack
    let (fx, fy, ry) = (x + steer, front + dir * 6.0, rear - dir * 4.5);
    if lit {
        blip.fill_glow_circle(fx, fy, 2.0, rgb([255, 252, 230]));
        blip.fill_glow_circle(x, ry, 1.7, rgb([255, 40, 40]));
    } else {
        blip.fill_rect(fx - 1.0, fy - 1.0, 2.0, 2.0, rgb([196, 196, 190]));
        blip.fill_rect(x - 1.0, ry - 1.0, 2.0, 2.0, rgb([130, 30, 30]));
    }
}

/// A rider sitting up on a bicycle, seen from above as Copenhagen rides: knees rising either side of the frame in
/// turn, the coat's back over the saddle, shoulders square, arms forward to the grips, the head (a helmet or hair)
/// on top. `bars` is how far ahead of `y` the bars are.
#[allow(clippy::too_many_arguments)]
pub(super) fn rider_on(blip: &Blip, x: f32, y: f32, dir: f32, bars: f32, steer: f32, coat: [u8; 3], head: [u8; 3], t: f32, s: f32) {
    let pedal = (t * 9.0).sin();
    let legs = rgb([44, 48, 64]);
    // knees: the one coming up shows further forward
    blip.fill_rect(x - 4.0 * s, y + dir * (1.5 + pedal * 2.0) * s - 1.5 * s, 2.5 * s, 3.0 * s, legs);
    blip.fill_rect(x + 1.5 * s, y + dir * (1.5 - pedal * 2.0) * s - 1.5 * s, 2.5 * s, 3.0 * s, legs);
    let back = rgb(shade_u(coat, 0.8));
    blip.fill_rect(x - 3.0 * s, y - dir * 4.0 * s - 2.0 * s, 6.0 * s, 5.0 * s, back);
    let shoulders = y - dir * 0.5 * s;
    let grips = y + dir * (bars - 2.0);
    let sleeve = rgb(shade_u(coat, 0.9));
    blip.draw_line_ex(x - 3.5 * s, shoulders, x + steer - 5.5, grips, 2.0 * s, sleeve);
    blip.draw_line_ex(x + 3.5 * s, shoulders, x + steer + 5.5, grips, 2.0 * s, sleeve);
    blip.fill_rect(x + steer - 6.5, grips - 1.0, 2.0, 2.0, rgb([226, 180, 140]));
    blip.fill_rect(x + steer + 4.5, grips - 1.0, 2.0, 2.0, rgb([226, 180, 140]));
    // square shoulders, the head on top, a touch forward
    blip.fill_rect(x - 4.5 * s, shoulders - 2.0 * s, 9.0 * s, 4.0 * s, rgb(coat));
    blip.fill_circle(x - 4.0 * s, shoulders, 2.0 * s, rgb(coat));
    blip.fill_circle(x + 4.0 * s, shoulders, 2.0 * s, rgb(coat));
    let hy = shoulders + dir * 1.0 * s;
    blip.fill_circle(x + steer * 0.2, hy, 2.9 * s, rgb(head));
    blip.fill_rect(x + steer * 0.2 - 1.0 * s, hy - 2.5 * s, 2.0 * s, 1.0 * s, rgba([255, 255, 255], 0.35));
}

pub(super) fn car(blip: &Blip, x: f32, y: f32, across: bool, body: [u8; 3]) {
    // a small box in height: the body, then the glasshouse and the roof
    let (hw, hh) = if across { (20.0, 11.0) } else { (11.0, 20.0) };
    let (ox, oy) = sun(4.0);
    blip.fill_rect(x - hw + ox, y - hh + oy, hw * 2.0, hh * 2.0, rgba([20, 20, 30], 0.3));
    block(x, y, hw, hh, 0.0, 10.0, rgb(shade_u(body, 0.85)), rgb(body));
    let (cw, ch) = if across { (hw * 0.5, hh - 1.5) } else { (hw - 1.5, hh * 0.5) };
    block(x, y, cw, ch, 10.0, 17.0, rgb([110, 140, 170]), rgb(shade_u(body, 0.92)));
}

pub(super) fn draw_hazard(blip: &Blip, g: &Game, h: &Hazard, t: f32) {
    if h.gone { return; }
    let y = sy(h.d);
    if y < -80.0 || y > WIN_H as f32 + 80.0 { return; }
    let x = h.x;
    let blink = (t * 6.0).fract() < 0.5;
    match h.kind {
        Hz::Car if h.bus => {
            // a city bus coming: its headlights on the setts ahead of it
            for k in 0..6 {
                let k = k as f32;
                blip.fill_rect(x - 10.0 - k * 2.0, y + 42.0 + k * 10.0, 20.0 + k * 4.0, 10.0, rgba([255, 240, 170], 0.16 - k * 0.02));
            }
            let line = if district(Side::Left, row_at(h.d).unwrap_or(0)) == District::Norrebro { "5C" } else { "1A" };
            bus(blip, x, y, false, (h.d as usize).is_multiple_of(3), line, false, false, t);
        }
        Hz::Bus => {
            let up = h.side == Side::Right;
            let line = if district(Side::Left, row_at(h.d).unwrap_or(0)) == District::Norrebro { "5C" } else { "1A" };
            // passengers stepping over the cycle track between the door and the kerb, on the ground beside the bus
            if h.phase == TELL {
                let lane = BIKE_LANE[h.side.index()];
                let out = (lane - x).signum();
                for p in 0..2 {
                    let k = ((h.t * 0.8 + p as f32 * 0.5).fract() * 2.0 - 1.0).abs();
                    let px = blip::lerp(x + out * 22.0, lane + out * 14.0, k);
                    person(blip, px, y - 20.0 + p as f32 * 12.0, [[60, 80, 140], [200, 90, 60]][p], [80, 60, 40], (t * 6.0 + p as f32).sin() * 2.0);
                }
            }
            bus(blip, x, y, up, false, line, h.phase == TELL, h.phase == TELL && h.t > BUS_DWELL - 1.4, t);
        }
        Hz::Tourist => {
            // a white rental e-bike, the big screen glowing on its bars, a backpack on the rider
            let (_, staring, _) = tourist_phase(h.t + h.beat);
            let staring = staring && !h.guide;
            let up = h.side == Side::Right;
            let dir = if up { -1.0 } else { 1.0 };
            let lean = if h.guide { 0.0 } else { (tourist_x(h.home, h.side, h.t + h.beat + 0.1) - x) * 0.6 };
            bicycle(blip, x, y, dir, 26.0, 9.0, lean, [236, 236, 232], false, lamps_on(g));
            // the e-bike's battery on the down tube
            blip.fill_rect(x - 1.5, y + dir * 2.0 - 3.0, 3.0, 6.0, rgb([70, 70, 78]));
            rider_on(blip, x, y, dir, 9.0, lean, [250, 120, 60], [240, 210, 150], t * 0.8 + h.beat, 1.0);
            // the backpack on the rider's back, and the big tablet on the bars, glowing when stared at
            blip.fill_rect(x - 3.5, y - dir * 4.0 - 2.5, 7.0, 5.0, rgb([60, 130, 90]));
            let glow = if staring && (t * 8.0).fract() < 0.6 { [160, 230, 255] } else { [70, 110, 140] };
            let sy0 = y + dir * 11.5;
            if staring { blip.fill_glow_circle(x + lean, sy0, 6.0, rgba(glow, 0.25)); }
            blip.fill_rect(x - 4.0 + lean, sy0 - 2.0, 8.0, 4.0, rgb([30, 30, 34]));
            blip.fill_rect(x - 3.0 + lean, sy0 - 1.0, 6.0, 2.0, rgb(glow));
            if staring && !h.guide { label(blip, "?", x + 10.0, y - 26.0, 2.0, rgb([255, 255, 255])); }
            if h.guide {
                // the guide's flag, held high so the flock can follow it
                let (fx, fy) = lift(x + 4.0, y, 34.0);
                blip.draw_line(x + 4.0, y, fx, fy, rgb([60, 60, 66]));
                blip.fill_rect(fx, fy, 10.0, 7.0, rgb([200, 22, 40]));
                blip.fill_rect(fx + 3.0, fy, 2.0, 7.0, rgb([250, 250, 250]));
                blip.fill_rect(fx, fy + 3.0, 10.0, 1.5, rgb([250, 250, 250]));
            }
        }
        Hz::Car if h.oncoming => {
            // headlights reach down the street ahead of it: you see the light before the car
            if h.phase >= GO || y > -200.0 {
                for k in 0..6 {
                    let k = k as f32;
                    blip.fill_rect(x - 9.0 - k * 2.0, y + 22.0 + k * 10.0, 18.0 + k * 4.0, 10.0, rgba([255, 240, 170], 0.16 - k * 0.02));
                }
            }
            let taxi = (h.d as usize / 110) % 3 == 1;
            car(blip, x, y, false, if taxi { [36, 36, 40] } else { CARS[(h.home as usize + h.d as usize / 7) % CARS.len()] });
            blip.fill_rect(x - 8.0, y + 18.0, 4.0, 2.0, rgb([255, 250, 210]));
            blip.fill_rect(x + 4.0, y + 18.0, 4.0, 2.0, rgb([255, 250, 210]));
            if taxi {
                // a Copenhagen taxi: the TAXA sign lit on the roof
                let (tx, ty) = lift(x, y, 19.0);
                blip.fill_rect(tx - 7.0, ty - 3.0, 14.0, 6.0, rgb([250, 214, 70]));
                blip.draw_text("TAXA", tx - 6.0, ty - 2.0, 0.5, rgb([40, 30, 10]));
            }
        }
        Hz::Car => {
            let across = h.phase < PAUSE || h.phase == PAUSE && h.t < 0.25;
            let body = CARS[(h.d as usize / 13) % CARS.len()];
            car(blip, x, y, across, body);
            if across && (h.phase == TELL || h.phase == GO) {
                // the reverse lights: white and glowing at the end nearest the street
                let rx = x - h.side.sign() * 20.0;
                for dy in [-7.0, 7.0] { blip.fill_glow_circle(rx, y + dy, 3.5, rgba([255, 255, 240], if blink { 1.0 } else { 0.6 })); }
            }
        }
        Hz::Dog => {
            if h.phase == WAIT || h.phase == TELL {
                // behind the gate: an ear and a wagging tail over the hedge
                if h.phase == TELL { label(blip, "!", x, y - 22.0, 2.0, rgb([255, 220, 90])); }
                blip.fill_circle(x, y, 5.0, rgb([150, 100, 60]));
                return;
            }
            let wag = (t * 18.0).sin() * 4.0;
            let (ox, oy) = sun(2.5);
            blip.fill_circle(x + ox, y + oy, 8.0, rgba([20, 20, 30], 0.25));
            blip.fill_rect(x - 5.0, y - 7.0, 10.0, 15.0, rgb([170, 112, 64]));
            blip.fill_circle(x, y - 9.0, 5.5, rgb([186, 126, 74]));
            blip.fill_rect(x - 6.0, y - 13.0, 3.0, 5.0, rgb([110, 70, 40]));
            blip.fill_rect(x + 3.0, y - 13.0, 3.0, 5.0, rgb([110, 70, 40]));
            blip.draw_line_ex(x, y + 8.0, x + wag, y + 14.0, 2.0, rgb([170, 112, 64]));
            if h.phase == AWAY && h.fetch.is_none() || h.phase == HOME && h.t < 3.0 && h.fetch.is_none() && h.touched {
                blip.fill_rect(x - 5.0, y - 15.0, 10.0, 3.0, rgb([250, 248, 236]));
            }
        }
        Hz::Sprinkler => {
            blip.fill_circle(x, y, 3.0, rgb([90, 90, 100]));
            if h.phase == TELL {
                for k in 0..4 {
                    let a = (t * 3.0 + k as f32 / 4.0).fract();
                    blip.fill_circle(x + (k as f32 - 1.5) * 3.0, y - a * 10.0, 1.5, rgba([200, 230, 255], 1.0 - a));
                }
            }
            if h.phase == GO {
                // a fan of droplets over the pavement and the bike lane
                let reach = (CURB[0] + SPRAY_REACH) - FENCE[0] + 14.0;
                for k in 0..40 {
                    let a = scatter(k, 31);
                    let dist = 10.0 + ((t * 1.6 + scatter(k, 32)).fract()) * reach;
                    let ang = (a - 0.5) * 1.6;
                    let px = x - h.side.sign() * dist * ang.cos();
                    let py = y + dist * ang.sin() * 0.9;
                    blip.fill_rect(px, py, 2.0, 2.0, rgba([180, 220, 255], 0.75));
                }
            }
        }
        Hz::Blader => {
            let swing = (t * 8.0).sin() * 3.0;
            blip.fill_rect(x - 4.0, y + 6.0, 2.0, 6.0, rgb([60, 60, 70]));
            blip.fill_rect(x + 2.0, y + 6.0, 2.0, 6.0, rgb([60, 60, 70]));
            person(blip, x + swing * 0.5, y, [80, 200, 210], [250, 120, 80], swing);
        }
        Hz::Van => {
            // a white delivery van in height: the cargo box, the cab in front, hazard lights when it is parked
            let up = h.side == Side::Right;
            let cab = if up { y - 22.0 } else { y + 22.0 };
            let (ox, oy) = sun(5.0);
            blip.fill_rect(x - 13.0 + ox, y - 30.0 + oy, 26.0, 60.0, rgba([20, 20, 30], 0.3));
            block(x, y + if up { 6.0 } else { -6.0 }, 13.0, 24.0, 0.0, 30.0, rgb([214, 214, 210]), rgb([240, 240, 236]));
            block(x, cab, 12.0, 8.0, 0.0, 20.0, rgb([110, 140, 170]), rgb([236, 236, 232]));
            if h.side == Side::Left && blink {
                for (dx, dy) in [(-12.0, -29.0), (10.0, -29.0), (-12.0, 27.0), (10.0, 27.0)] {
                    blip.fill_glow_circle(x + dx + 1.0, y + dy + 1.0, 3.0, rgb([255, 170, 40]));
                }
            }
        }
        Hz::Puddle(_) => {
            let big = if g.rain() { 1.25 } else { 1.0 };
            for (k, a) in [(1.0, 0.75), (0.7, 0.5)] {
                let (rw, rh) = (22.0 * big * k, 14.0 * big * k);
                for i in 0..(rh as i32) {
                    let f = i as f32 / rh;
                    let w = rw * (1.0 - (2.0 * f - 1.0).powi(2)).sqrt();
                    blip.fill_rect(x - w, y - rh / 2.0 + i as f32, w * 2.0, 1.0, rgba([96, 130, 160], a));
                }
            }
            let ripple = (t * 1.5).fract();
            blip.draw_circle(x + 4.0, y, 3.0 + ripple * 9.0, 1.0, rgba([220, 240, 255], 0.5 * (1.0 - ripple)));
        }
        Hz::Pigeons(_) => {
            for k in 0..5 {
                let (ox, oy) = ((scatter(k, 41) - 0.5) * 34.0, (scatter(k, 42) - 0.5) * 26.0);
                if h.phase == GO {
                    let up = h.t * 120.0;
                    let flap = (t * 20.0 + k as f32).sin() * 5.0;
                    let px = x + ox * (1.0 + h.t * 2.0) + (k as f32 - 2.0) * up * 0.4;
                    let py = y + oy - up;
                    blip.fill_rect(px - 6.0, py + flap * 0.2, 12.0, 2.0, rgb([150, 150, 165]));
                    blip.fill_circle(px, py, 3.0, rgb([120, 120, 135]));
                } else {
                    let peck = ((t * 3.0 + k as f32).sin() > 0.6) as i32 as f32 * 2.0;
                    blip.fill_circle(x + ox, y + oy, 4.0, rgb([136, 136, 150]));
                    blip.fill_circle(x + ox, y + oy - 4.0 + peck, 2.5, rgb([90, 110, 120]));
                }
            }
        }
        Hz::Cart => {
            let (ox, oy) = sun(3.0);
            blip.fill_rect(x - 12.0 + ox, y - 14.0 + oy, 24.0, 28.0, rgba([20, 20, 30], 0.3));
            blip.fill_rect(x - 12.0, y - 14.0, 24.0, 28.0, rgb([200, 60, 50]));
            for k in 0..4 { blip.fill_rect(x - 9.0, y - 11.0 + k as f32 * 6.0, 18.0, 4.0, rgb([248, 244, 232])); }
            blip.fill_circle(x - 12.0, y + 10.0, 3.0, rgb([40, 40, 40]));
            blip.fill_circle(x + 12.0, y + 10.0, 3.0, rgb([40, 40, 40]));
            let bob = (t * 4.0).sin() * 2.0;
            label(blip, "+10", x, y - 30.0 + bob, 2.0, rgb(GOLD));
        }
        Hz::Stall => {
            let c = [[220, 70, 70], [70, 140, 210], [240, 180, 60], [90, 170, 110]][(h.d as usize / 11) % 4];
            let (ox, oy) = sun(4.0);
            blip.fill_rect(x - 25.0 + ox, y - 32.0 + oy, 50.0, 64.0, rgba([20, 20, 30], 0.3));
            for k in 0..8 {
                let col = if k % 2 == 0 { c } else { [250, 246, 236] };
                blip.fill_rect(x - 25.0, y - 32.0 + k as f32 * 8.0, 50.0, 8.0, rgb(col));
            }
            for k in 0..6 {
                let fx = x - 18.0 + (k % 3) as f32 * 18.0;
                let fy = y - 16.0 + (k / 3) as f32 * 26.0;
                blip.fill_circle(fx, fy, 5.0, rgb([[240, 120, 40], [220, 40, 60], [120, 190, 70]][k % 3]));
            }
        }
        Hz::CrossCar => {
            let body = if h.bus { [244, 196, 40] } else { CARS[(h.d as usize + h.home as usize / 3) % CARS.len()] };
            car(blip, x, y, true, body);
            let front = x + if h.side == Side::Left { 20.0 } else { -20.0 };
            blip.fill_rect(front - 1.0, y - 8.0, 2.0, 4.0, rgb([255, 250, 210]));
            blip.fill_rect(front - 1.0, y + 4.0, 2.0, 4.0, rgb([255, 250, 210]));
        }
        Hz::Cyclist => {
            // a commuter on a Copenhagen bike, upright, now and then a cargo bike with a child in the box
            let up = h.side == Side::Right;
            let dir = if up { -1.0 } else { 1.0 };
            let pick = |n: usize, m: usize| (h.d as usize / n) % m;
            let cargo = pick(17, 3) == 0;
            let coat = if g.rain() { [[250, 210, 40], [40, 60, 110], [230, 60, 60], [60, 140, 90]][pick(5, 4)] }
                else { [[60, 70, 90], [180, 60, 60], [230, 220, 200], [70, 120, 90]][pick(5, 4)] };
            // frames: black, racing green, cream, sky blue, the classic red of a Danish city bike
            let frame = [[34, 34, 38], [40, 90, 60], [226, 214, 186], [110, 160, 200], [176, 40, 40]][pick(7, 5)];
            // half wear a helmet (white, black or orange), half ride in their hair
            let head = [[240, 240, 236], [90, 60, 40], [30, 30, 34], [230, 200, 120], [236, 120, 40], [40, 30, 30]][pick(3, 6)];
            let lit = lamps_on(g);
            if cargo {
                // a long-john cargo bike: the wooden box between the rider and the small front wheel, a child in it
                bicycle(blip, x, y - dir * 4.0, dir, 36.0, 5.0, 0.0, frame, false, lit);
                let bx = y + dir * 9.0;
                blip.fill_rect(x - 7.0, bx - 6.0, 14.0, 12.0, rgb([150, 104, 60]));
                blip.fill_rect(x - 6.0, bx - 5.0, 12.0, 10.0, rgb([188, 140, 84]));
                for k in [-2.5, 0.5, 3.5] { blip.fill_rect(x - 6.0, bx + k - 0.5, 12.0, 1.0, rgb([150, 104, 60])); }
                blip.fill_circle(x, bx, 3.0, rgb([230, 120, 90]));
                blip.fill_circle(x, bx - dir * 0.5, 2.0, rgb([240, 200, 140]));
                rider_on(blip, x, y - dir * 4.0, dir, 5.0, 0.0, coat, head, t + h.d * 0.01, 1.0);
            } else {
                bicycle(blip, x, y, dir, 26.0, 9.0, 0.0, frame, pick(11, 3) == 0, lit);
                rider_on(blip, x, y, dir, 9.0, 0.0, coat, head, t + h.d * 0.01, 1.0);
            }
        }
        Hz::Walker => {
            let swing = (t * 6.0).sin() * 2.0;
            let coat = [[150, 80, 160], [60, 120, 90], [200, 120, 60]][(h.d as usize / 9) % 3];
            person(blip, x, y, coat, [90, 60, 40], swing);
            blip.fill_rect(x + 6.0, y + 2.0, 5.0, 6.0, rgb([230, 210, 160]));
            if g.rain() { blip.fill_circle(x, y - 3.0, 10.0, rgb([[40, 40, 50], [200, 40, 50], [40, 90, 160]][(h.d as usize / 7) % 3])); }
        }
    }
}

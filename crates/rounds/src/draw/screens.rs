//! The screens around the ride: the weather icon, the bar, the day's card, the tally, the week, the canal's HUD,
//! banners, the title and game over.

use super::*;

// ---- screens ------------------------------------------------------------------

/// A weather picture for the day card and the bar: the day's tell, drawn.
pub fn weather_icon(blip: &Blip, weather: Weather, x: f32, y: f32, k: f32, t: f32) {
    match weather {
        Weather::Clear => {
            blip.fill_sunburst(x, y, 12, t * 0.3, 16.0 * k, [rgba(GOLD, 0.8), rgba(GOLD, 0.0)]);
            blip.fill_circle(x, y, 9.0 * k, rgb([255, 200, 70]));
        }
        Weather::Wind(_) => {
            for i in 0..3 {
                let yy = y - 8.0 * k + i as f32 * 8.0 * k;
                let off = ((t * 2.0 + i as f32) % 1.0) * 4.0;
                blip.fill_rect(x - 14.0 * k + off, yy, (22.0 - i as f32 * 4.0) * k, 2.0 * k.max(1.0), rgb([220, 236, 250]));
            }
        }
        Weather::Rain => {
            for (dx, r) in [(-7.0, 7.0), (2.0, 9.0), (10.0, 6.0)] { blip.fill_circle(x + dx * k, y - 4.0 * k, r * k, rgb([170, 180, 200])); }
            for i in 0..4 {
                let dy = ((t * 2.0 + i as f32 * 0.25) % 1.0) * 10.0 * k;
                blip.fill_rect(x + (i as f32 * 6.0 - 9.0) * k, y + 6.0 * k + dy, 2.0, 4.0 * k, rgb([120, 170, 240]));
            }
        }
        Weather::Fog => {
            for i in 0..3 { blip.fill_rect(x - 14.0 * k, y - 8.0 * k + i as f32 * 7.0 * k, 28.0 * k, 4.0 * k, rgba([230, 232, 236], 0.9)); }
        }
        Weather::Market => {
            for i in 0..4 {
                let c = if i % 2 == 0 { [220, 70, 70] } else { [250, 246, 236] };
                blip.fill_rect(x - 14.0 * k + i as f32 * 7.0 * k, y - 10.0 * k, 7.0 * k, 9.0 * k, rgb(c));
            }
            blip.fill_rect(x - 12.0 * k, y - 1.0 * k, 24.0 * k, 10.0 * k, rgb([170, 116, 70]));
        }
        Weather::Bundles => {
            for i in 0..3 {
                let yy = y + (4.0 - i as f32 * 6.0) * k;
                blip.fill_rect(x - 12.0 * k, yy, 24.0 * k, 5.0 * k, rgb([248, 246, 236]));
                blip.fill_rect(x - 1.0 * k, yy, 2.0 * k, 5.0 * k, rgb([200, 60, 60]));
            }
        }
    }
}

pub(super) fn panel(blip: &Blip, x: f32, y: f32, w: f32, h: f32) {
    blip.fill_rect(x + 4.0, y + 4.0, w, h, rgba([0, 0, 0], 0.4));
    blip.fill_rect(x, y, w, h, rgb([250, 242, 222]));
    blip.draw_rect(x + 3.0, y + 3.0, w - 6.0, h - 6.0, rgb([200, 80, 90]));
}

pub(super) const WEEKDAYS: [&str; 7] = ["MON", "TUE", "WED", "THU", "FRI", "SAT", "SUN"];

pub fn draw_bar(blip: &Blip, g: &Game, t: f32) {
    let y = WIN_H as f32 - BAR_H;
    let w = WIN_W as f32;
    // The shared HUD's bar, upside down: ink with a cream rim.
    blip.fill_rect(0.0, y, w, BAR_H, BLIP_INK);
    blip.fill_rect(0.0, y + 1.0, w, 2.0, BLIP_CREAM);
    let ty = y + 9.0;
    blip.draw_text(WEEKDAYS[g.day], 10.0, ty, 2.0, rgb([255, 236, 200]));
    weather_icon(blip, g.weather(), 64.0, y + 15.0, 0.55, t);
    // papers in the box
    blip.fill_rect(90.0, ty + 2.0, 14.0, 8.0, rgb([248, 246, 236]));
    blip.fill_rect(96.0, ty + 2.0, 2.0, 8.0, rgb([200, 60, 60]));
    let low = g.stock <= 3 && (t * 4.0).fract() < 0.5;
    blip.draw_number(g.stock, 110.0, ty, 2.0, if low { rgb([255, 120, 110]) } else { BLIP_WHITE });
    // the round: homes that take the paper (a house: the hearts up top are lives)
    blip.fill_rect(160.0, ty + 4.0, 12.0, 8.0, BLIP_CREAM);
    tri((158.0, ty + 5.0), (174.0, ty + 5.0), (166.0, ty - 2.0), BLIP_HEART);
    blip.fill_rect(165.0, ty + 7.0, 3.0, 5.0, BLIP_INK);
    blip.draw_number(g.subscribers() as i32, 178.0, ty, 2.0, BLIP_WHITE);
    if g.two_player {
        blip.draw_text("2P", 230.0, ty, 2.0, rgb([140, 230, 160]));
        blip.draw_number(g.score2, 258.0, ty, 2.0, BLIP_WHITE);
    }
    // how far along the round
    let (x0, x1) = (if g.two_player { 350.0 } else { 236.0 }, w - 44.0);
    let k = if g.state == State::Canal { 1.0 - g.timer.remaining() / CANAL_SECS } else { g.bike_d / g.end_d() };
    blip.fill_rect(x0, ty + 4.0, x1 - x0, 4.0, rgb([70, 62, 80]));
    blip.fill_rect(x0, ty + 4.0, (x1 - x0) * k.clamp(0.0, 1.0), 4.0, rgb([240, 190, 90]));
    blip.fill_circle(x0 + (x1 - x0) * k.clamp(0.0, 1.0), ty + 6.0, 5.0, rgb([236, 110, 60]));
    blip.fill_circle(x1 + 6.0, ty + 6.0, 5.0, rgb([120, 200, 140]));
}

pub fn draw_card(blip: &Blip, g: &Game, t: f32) {
    let (x, y, w, h) = (60.0, 120.0, 360.0, 250.0);
    let slide = ((g.clock / 0.35).min(1.0) - 1.0) * 40.0;
    let y = y + slide;
    panel(blip, x, y, w, h);
    let ink = rgb([60, 40, 50]);
    blip.draw_centered(&format!("WEEK {}  NOERREBRO TO NYHAVN", g.week), y + 22.0, 2.0, rgb([200, 80, 90]));
    blip.draw_centered(g.today().name, y + 46.0, 4.0, ink);
    weather_icon(blip, g.weather(), 240.0, y + 112.0, 1.6, t);
    blip.draw_centered(g.today().tell, y + 150.0, 2.0, ink);
    blip.draw_centered(&format!("ROUND {} HOMES", g.subscribers()), y + 182.0, 2.0, rgb([200, 80, 90]));
    blip.draw_centered(&format!("PAPERS {}", g.stock), y + 206.0, 2.0, ink);
    if g.weather() == Weather::Wind(-60.0) || matches!(g.weather(), Weather::Wind(_)) {
        // the wind's way, an arrow along the street
        let dir = -g.wind().signum();
        let (ax, ay) = (290.0, y + 112.0);
        blip.draw_line_ex(ax, ay - 12.0 * dir, ax, ay + 12.0 * dir, 3.0, ink);
        blip.draw_line_ex(ax, ay + 12.0 * dir, ax - 6.0, ay + 5.0 * dir, 3.0, ink);
        blip.draw_line_ex(ax, ay + 12.0 * dir, ax + 6.0, ay + 5.0 * dir, 3.0, ink);
    }
}

pub fn draw_tally(blip: &Blip, g: &Game) {
    let (x, y, w, h) = (50.0, 96.0, 380.0, 320.0);
    panel(blip, x, y, w, h);
    let ink = rgb([60, 40, 50]);
    let accent = rgb([200, 80, 90]);
    blip.draw_centered(&format!("{} DONE", g.today().name), y + 22.0, 3.0, ink);
    let x2 = bundles_mult(g);
    let rows: [(&str, String, bool); 6] = [
        ("LETTERBOX", format!("{} X {}", g.tally.letterbox, LETTERBOX_PTS * x2), true),
        ("DOORSTEP", format!("{} X {}", g.tally.doorstep, DOORSTEP_PTS * x2), true),
        ("SAMPLES", format!("{} X {}", g.tally.samples, SAMPLE_PTS * x2), true),
        ("MISSED", g.tally.misses.to_string(), g.tally.misses > 0),
        ("CANCELLED", g.tally.lost.to_string(), g.tally.lost > 0),
        ("NEW TOMORROW", g.tally.won.to_string(), g.tally.won > 0),
    ];
    for (i, (name, val, on)) in rows.iter().enumerate() {
        if g.clock < 0.25 + i as f32 * 0.22 { break; }
        let ry = y + 64.0 + i as f32 * 30.0;
        let c = if *on { ink } else { rgba([60, 40, 50], 0.4) };
        blip.draw_text(name, x + 24.0, ry, 2.0, c);
        blip.draw_text(val, x + w - 24.0 - text_width(val, 2.0), ry, 2.0, c);
    }
    if g.clock > 1.8 {
        if g.tally.perfect > 0 {
            let pulse = 0.75 + 0.25 * (g.clock * 6.0).sin();
            blip.draw_centered(&format!("PERFECT DAY +{}", g.tally.perfect), y + 252.0, 2.0, BlipColor { a: pulse, ..accent });
        }
        blip.draw_centered("NEXT: THE CANAL RUN", y + 282.0, 2.0, rgb([60, 110, 150]));
    }
}

pub(super) fn bundles_mult(g: &Game) -> i32 { if g.weather() == Weather::Bundles { 2 } else { 1 } }

pub fn draw_week(blip: &Blip, g: &Game) {
    let (x, y, w, h) = (60.0, 140.0, 360.0, 220.0);
    panel(blip, x, y, w, h);
    let ink = rgb([60, 40, 50]);
    blip.draw_centered(&format!("WEEK {} DONE", g.week), y + 26.0, 3.0, ink);
    blip.draw_centered(&format!("{} HOMES KEPT", g.tally.won), y + 80.0, 2.0, ink);
    blip.draw_centered(&format!("+{}", g.tally.won * KEPT_PTS), y + 108.0, 3.0, rgb([200, 80, 90]));
    blip.draw_centered("A SPARE LIFE", y + 150.0, 2.0, ink);
    blip.draw_centered("THE STREETS GET BUSIER", y + 180.0, 2.0, rgb([60, 110, 150]));
}

pub fn draw_canal_hud(blip: &Blip, g: &Game) {
    if g.clock < 2.2 {
        let a = (2.2 - g.clock).min(1.0);
        label(blip, "NYHAVN", 240.0, 120.0, 5.0, rgba([255, 236, 200], a));
        label(blip, "PAPERS FOR THE CAFES", 240.0, 176.0, 2.0, rgba([255, 255, 255], a));
        label(blip, "AND THE BOATS", 240.0, 200.0, 2.0, rgba([255, 255, 255], a));
    }
    let secs = g.timer.remaining().ceil() as i32;
    label(blip, &secs.to_string(), 240.0, HUD_H + 10.0, 3.0, rgb(if secs <= 3 { [255, 140, 120] } else { [255, 255, 255] }));
}

pub fn draw_banner(blip: &Blip, g: &Game) {
    // at the T: what the turn is waiting for
    if g.in_junction() && g.heading < 0.5 {
        label(blip, "TURN LEFT HERE", 240.0, BIKE_Y - 90.0, 2.0, rgb([255, 236, 160]));
    }
    if g.state == State::Play && g.place_t < 3.0 {
        let a = (3.0 - g.place_t).min(1.0) * (g.place_t * 3.0).min(1.0);
        let name = PLACES[g.place].1;
        blip.draw_message(name, 240.0, HUD_H + 22.0, 2.0, BLIP_CREAM, a);
    }
    if let Some((text, left)) = &g.banner {
        label(blip, text, 240.0, 150.0, 2.0, rgba(GOLD, left.min(1.0)));
    }
    if g.state == State::Play && g.clock < 1.2 {
        label(blip, "GO!", 240.0, 200.0, 5.0, rgba([255, 236, 200], (1.2 - g.clock).min(1.0)));
    }
}

pub fn draw_title(blip: &Blip, g: &Game, hi: i32, t: f32) {
    draw_world(blip, g, t);
    draw_weather(blip, g, t);
    skyline(blip, t);
    let (cx, cy) = (240.0, 70.0);
    blip.draw_logo("ROUNDS", cx, cy, 7.0, rgb([240, 120, 150]));
    label(blip, "A MORNING ROUND IN COPENHAGEN", cx, cy + 60.0, 2.0, rgb([255, 236, 210]));
    if hi > 0 { blip.draw_message(&format!("HI {hi}"), cx, cy + 96.0, 2.0, BLIP_GOLD, 1.0); }
    blip.draw_start_prompt(WIN_H as f32 * 0.52);
    let by = blip::web::controls();
    blip.draw_panel(48.0, 318.0, WIN_W as f32 - 96.0, 82.0, 1.0);
    label(blip, by.pick("A D STEER  W S SPEED", "STICK STEERS AND PEDALS", "DRAG TO STEER AND PEDAL"), cx, 330.0, 2.0, BLIP_WHITE);
    label(blip, by.pick("F THROWS LEFT  G RIGHT", "A THROWS LEFT  B RIGHT", "TAP LEFT OR RIGHT TO THROW"), cx, 354.0, 2.0, BLIP_WHITE);
    label(blip, by.pick("J: TWO ON A TANDEM", "2P JOINS ON THE BACK", "2P JOINS ON THE BACK"), cx, 378.0, 2.0, rgb([160, 230, 170]));
    let _ = BLIP_BLACK;
}

/// Copenhagen at dawn, as a postcard: the sky warming, the towers dark against it (Vor Frelsers Kirke's twisting
/// spire, Christiansborg's tower, Marmorkirken's dome, Rundetaarn), and Nyhavn's houses on the water in front.
pub(super) fn skyline(blip: &Blip, t: f32) {
    let (top, horizon, bottom) = (0.0, 238.0, 290.0);
    let mut y = top;
    while y < horizon {
        let f = (y - top) / (horizon - top);
        let c = [(60.0 + 195.0 * f) as u8, (50.0 + 120.0 * f * f) as u8, (110.0 - 10.0 * f) as u8];
        blip.fill_rect(0.0, y, WIN_W as f32, 4.0, rgb(c));
        y += 4.0;
    }
    blip.fill_circle(330.0, horizon - 6.0, 30.0, rgba([255, 214, 130], 0.9));
    blip.fill_sunburst(330.0, horizon - 6.0, 14, t * 0.08, 140.0, [rgba([255, 214, 140], 0.18), rgba([255, 214, 140], 0.0)]);
    let ink = rgb([46, 36, 70]);
    // Vor Frelsers Kirke
    blip.fill_rect(40.0, horizon - 50.0, 26.0, 50.0, ink);
    for i in 0..90 {
        let f = i as f32 / 90.0;
        let half = 10.0 * (1.0 - f) + 1.0;
        blip.fill_rect(53.0 - half, horizon - 50.0 - i as f32, half * 2.0, 1.2, ink);
        let a = f * TAU * 4.0;
        if a.sin() > 0.0 { blip.fill_rect(53.0 + a.cos() * (half + 1.5) - 1.0, horizon - 50.0 - i as f32, 2.0, 1.5, rgb(GOLD)); }
    }
    blip.fill_circle(53.0, horizon - 143.0, 3.0, rgb(GOLD));
    // Christiansborg's tower and its crowns
    blip.fill_rect(118.0, horizon - 70.0, 30.0, 70.0, ink);
    blip.fill_rect(124.0, horizon - 96.0, 18.0, 26.0, ink);
    for i in 0..30 { let half = 7.0 * (1.0 - i as f32 / 30.0) + 1.0; blip.fill_rect(133.0 - half, horizon - 96.0 - i as f32, half * 2.0, 1.2, ink); }
    for k in 0..3 { blip.fill_circle(133.0, horizon - 132.0 - k as f32 * 7.0, 3.5, ink); }
    blip.fill_rect(80.0, horizon - 36.0, 120.0, 36.0, ink);
    // Marmorkirken
    blip.fill_rect(200.0, horizon - 40.0, 70.0, 40.0, ink);
    for i in 0..34 { let f = i as f32 / 34.0; let half = 30.0 * (1.0 - f * f).sqrt(); blip.fill_rect(235.0 - half, horizon - 40.0 - i as f32, half * 2.0, 1.2, ink); }
    blip.fill_rect(231.0, horizon - 90.0, 8.0, 18.0, ink);
    blip.fill_rect(234.0, horizon - 98.0, 2.0, 8.0, ink);
    // Rundetaarn
    blip.fill_rect(380.0, horizon - 62.0, 42.0, 62.0, ink);
    blip.fill_circle(401.0, horizon - 66.0, 8.0, ink);
    blip.fill_rect(300.0, horizon - 24.0, 180.0, 24.0, ink);
    // the water and Nyhavn's houses on it, lit by the low sun
    blip.fill_rect(0.0, horizon, WIN_W as f32, bottom - horizon, rgb([70, 90, 130]));
    for i in 0..16 {
        let x = i as f32 * 30.0;
        let c = NYHAVN[i % NYHAVN.len()];
        let h = 18.0 + (scatter(i, 99) * 3.0).floor() * 5.0;
        blip.fill_rect(x, horizon - h + 26.0, 29.0, h, rgb(c));
        tri((x, horizon - h + 26.0), (x + 29.0, horizon - h + 26.0), (x + 14.5, horizon - h + 16.0), rgb(shade_u(c, 0.92)));
        for wx in [x + 5.0, x + 18.0] { blip.fill_rect(wx, horizon - h + 31.0, 6.0, 5.0, rgb([255, 220, 140])); }
        let ripple = (t * 2.0 + i as f32).sin() * 2.0;
        blip.fill_rect(x + ripple, horizon + 30.0, 29.0, 4.0, rgba(c, 0.35));
    }
}

pub fn draw_over(blip: &Blip, g: &Game, hi: i32) {
    blip.fill_rect(0.0, 0.0, WIN_W as f32, WIN_H as f32, rgba([20, 14, 30], 0.72));
    let title = if g.subscribers() == 0 { "NO ONE LEFT" } else { "GAME OVER" };
    let offering = g.offering();
    blip.draw_end_screen(title, g.sess.score + g.score2, if g.two_player { 0 } else { hi }, rgb([255, 150, 130]),
        rgb([255, 214, 90]), BLIP_WHITE, !offering && !g.wait.active());
    if offering { blip.draw_continue(&g.cont, rgb([255, 214, 90])); }
    let note = if g.two_player { format!("1P {}   2P {}", g.sess.score, g.score2) } else {
        format!("WEEK {} {}   {} HOMES", g.week, WEEKDAYS[g.day], g.subscribers())
    };
    blip.draw_game_over_note(&note, rgb([200, 190, 210]));
}

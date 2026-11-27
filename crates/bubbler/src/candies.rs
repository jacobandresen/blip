//! The sweets a popped monster drops: tumbling, bouncing, collected and chewed.

use super::*;

pub(super) fn move_candies(g: &mut Game, dt: f32, sfx: &Sounds, player_starts: [(f32, f32); 2]) {
    for i in 0..g.fruits.len() {
        let mut f = g.fruits[i];
        if !f.active {
            continue;
        }
        let fruit_from = (f.x + 8.0, f.y + 8.0);
        f.t += dt;
        tumble(g, &mut f, dt);
        for (pi, start) in player_starts.iter().enumerate() {
            let p = &g.p[pi];
            if !p.joined || !p.alive || p.dead_t > 0.0 || f.t < 0.35 {
                continue;
            }
            if blip::sweep_hits(*start, (p.cx(), p.cy()), fruit_from, (f.x + 8.0, f.y + 8.0), (-16.0, -18.0, 32.0, 36.0)) {
                f.active = false;
                eat(g, pi, f, sfx);
                break;
            }
        }
        if f.t > 9.0 {
            f.active = false;
        }
        g.fruits[i] = f;
    }
    g.fruits.retain(|f| f.active);
}

/// A sweet in the air falls and slides to a stop where it lands; on the ground it waits, until the floor goes.
fn tumble(g: &Game, f: &mut Fruit, dt: f32) {
    if !f.on_ground {
        let (mut x, mut y, mut vx, mut vy) = (f.x, f.y, f.vx, f.vy);
        let (ground, _) = g.move_body(&mut x, &mut y, &mut vx, &mut vy, 16.0, 16.0, dt);
        f.x = x;
        f.y = y;
        f.vx = vx * (1.0 - 0.8 * dt);
        f.vy = vy;
        if ground {
            f.on_ground = true;
            f.vx = 0.0;
        }
    } else if !g.floor_at(f.x, f.x + 16.0, f.y + 16.0) {
        f.on_ground = false;
    }
}

/// Player `pi` eats sweet `f`: its points, a sparkle, and the chewing face.
fn eat(g: &mut Game, pi: usize, f: Fruit, sfx: &Sounds) {
    const VALUE: [i32; 4] = [500, 1000, 2000, 4000];
    let v = VALUE[f.kind];
    g.p[pi].score += v;
    g.p[pi].eat = 0.0;
    g.p[pi].eaten = f;
    g.stats.fruit += 1;
    g.pops.push(Popup { x: f.x + 8.0, y: f.y, t: 0.0, value: v, c: rgba(1.0, 1.0, 1.0, 1.0) });
    g.burst(f.x + 8.0, f.y + 8.0, 5, rgba(1.0, 1.0, 0.6, 1.0), 60.0, true);
    play_sfx(&sfx.fruit);
}

/// Crumbs fly from a chewing dragon's mouth on each chomp.
pub(super) fn update_chewing(g: &mut Game, dt: f32) {
    for pi in 0..2 {
        let p = &g.p[pi];
        if p.eat < 0.0 {
            continue;
        }
        if !CHOMPS.iter().any(|&c| p.eat - dt < c && p.eat >= c) {
            continue;
        }
        let (mx, my) = mouth_pos(p);
        let c = col(CANDY[candy_pick(&p.eaten)], 1.0);
        let last = p.eat - dt < CHOMPS[1] && p.eat >= CHOMPS[1];
        let (hx, hy) = (p.cx(), p.y);
        g.burst(mx, my, 4, c, 45.0, false);
        if last {
            // yum: two hearts rise from the dragon's head
            for k in 0..2 {
                let side = k as f32 * 2.0 - 1.0;
                g.hearts.push(Heart { x: hx + side * 6.0, y: hy - 2.0, t: -0.12 * k as f32, sway: side });
            }
        }
    }
}

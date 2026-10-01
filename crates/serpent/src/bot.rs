//! Native-only autopilot (BLIP_BOT=1): BFS to the bonus or the food, never
//! into a pocket smaller than the snake, one tap per turn like a player.

use super::*;
use std::collections::VecDeque;

const DIRS: [(Dir, i32, i32); 4] = [(Dir::Up, 0, -1), (Dir::Right, 1, 0), (Dir::Down, 0, 1), (Dir::Left, -1, 0)];

fn key(d: Dir) -> blip::macroquad::input::KeyCode {
    match d { Dir::Up => BLIP_KEY_UP, Dir::Down => BLIP_KEY_DOWN, Dir::Left => BLIP_KEY_LEFT, Dir::Right => BLIP_KEY_RIGHT }
}

fn blocked(g: &Game) -> Vec<bool> {
    let mut b = vec![false; (COLS * ROWS) as usize];
    // The tail cell frees up as the head moves, so leave it open.
    for i in 0..g.snake_len.saturating_sub(1) {
        let c = g.snake_at(i);
        b[(c.r * COLS + c.c) as usize] = true;
    }
    for o in &g.obstacles[..g.obstacle_count] { b[(o.r * COLS + o.c) as usize] = true; }
    b
}

fn inside(c: i32, r: i32) -> bool { c >= 0 && c < COLS && r >= 0 && r < ROWS }

/// First step and distance of the shortest path from `from` to `to`.
fn bfs(b: &[bool], from: Cell, to: Cell, first_ok: impl Fn(Dir) -> bool) -> Option<(Dir, i32)> {
    let mut seen = b.to_vec();
    let mut q = VecDeque::new();
    for (d, dc, dr) in DIRS {
        if !first_ok(d) { continue; }
        let (c, r) = (from.c + dc, from.r + dr);
        if inside(c, r) && !seen[(r * COLS + c) as usize] {
            seen[(r * COLS + c) as usize] = true;
            q.push_back((c, r, d, 1));
        }
    }
    while let Some((c, r, d, n)) = q.pop_front() {
        if c == to.c && r == to.r { return Some((d, n)); }
        for (_, dc, dr) in DIRS {
            let (nc, nr) = (c + dc, r + dr);
            if inside(nc, nr) && !seen[(nr * COLS + nc) as usize] {
                seen[(nr * COLS + nc) as usize] = true;
                q.push_back((nc, nr, d, n + 1));
            }
        }
    }
    None
}

fn room(b: &[bool], c: i32, r: i32) -> usize {
    if !inside(c, r) || b[(r * COLS + c) as usize] { return 0; }
    let mut seen = b.to_vec();
    let mut st = vec![(c, r)];
    seen[(r * COLS + c) as usize] = true;
    let mut n = 0;
    while let Some((c, r)) = st.pop() {
        n += 1;
        for (_, dc, dr) in DIRS {
            let (nc, nr) = (c + dc, r + dr);
            if inside(nc, nr) && !seen[(nr * COLS + nc) as usize] {
                seen[(nr * COLS + nc) as usize] = true;
                st.push((nc, nr));
            }
        }
    }
    n
}

pub fn drive(g: &Game) {
    match g.state {
        State::Title => { blip::bot::hold(&[BLIP_KEY_SPACE_]); return; }
        State::Play => {}
        _ => { blip::bot::hold(&[]); return; }
    }
    if g.turns_len > 0 { blip::bot::hold(&[]); return; }
    let b = blocked(g);
    let head = g.snake_at(0);
    let back = g.cur_dir.opposite();
    let roomy = |d: Dir| {
        let (_, dc, dr) = DIRS.iter().copied().find(|x| x.0 == d).unwrap();
        room(&b, head.c + dc, head.r + dr) > g.snake_len
    };
    let secs_per_step = g.move_interval() / 1000.0;
    let mut want = None;
    if g.bonus_active() {
        if let Some((d, n)) = bfs(&b, head, g.bonus, |d| d != back && roomy(d)) {
            if n as f32 * secs_per_step < g.bonus_ttl { want = Some(d); }
        }
    }
    if want.is_none() {
        want = bfs(&b, head, g.food, |d| d != back && roomy(d)).map(|x| x.0);
    }
    // No safe path: take the roomiest way on.
    let want = want.unwrap_or_else(|| {
        DIRS.iter().filter(|x| x.0 != back)
            .max_by_key(|(_, dc, dr)| room(&b, head.c + dc, head.r + dr)).unwrap().0
    });
    if want != g.cur_dir { blip::bot::hold(&[key(want)]); } else { blip::bot::hold(&[]); }
}

const BLIP_KEY_SPACE_: blip::macroquad::input::KeyCode = blip::input::BLIP_KEY_SPACE;

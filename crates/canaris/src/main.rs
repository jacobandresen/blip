//! Canaris – privateer arcade game inspired by Kaptajn Kaper i Kattegat (1985).
//!
//! Scene modules each own one entry in the `State` machine:
//!   * [`screens`] — title, dead, game-over, plus the shared HUD overlay
//!   * [`sea`]     — free-roam sailing (also exports the background/foreground/ship-fire
//!                   helpers reused by combat and title)
//!   * [`combat`]  — side-by-side cannon duel
//!   * [`boarding`]— crew-slot duel
//!   * [`port`]    — port shop and world-map selector

mod boarding;
mod combat;
mod port;
mod screens;
mod sea;
mod state;

use blip::{blip_image, blip_sound, load_png, play_ambient, play_music, window_conf, Blip, BLIP_BLACK};

use boarding::{draw_boarding, update_boarding};
use combat::{draw_combat, update_combat};
use port::{draw_map, draw_port, update_map, update_port};
use screens::{draw_dead, draw_gameover, draw_title, update_dead, update_gameover, update_title};
use sea::{draw_sea, update_sea};
use state::{
    Cannonball, Game, Sounds, State, Textures,
    CANNON_ARC_VY, CANNON_SPEED, COMBAT_PLAYER_X, PLAYER_H, PLAYER_RELOAD, PLAYER_W,
    WIN_H, WIN_W,
};

// ── asset bytes ───────────────────────────────────────────────────────────────

const PLAYER_A_PNG: &[u8] = blip_image!("player_ship_a.png");
const PLAYER_B_PNG: &[u8] = blip_image!("player_ship_b.png");
const ENEMY_A_PNG:  &[u8] = blip_image!("enemy_ship_a.png");
const ENEMY_B_PNG:  &[u8] = blip_image!("enemy_ship_b.png");
const BALL_PNG:     &[u8] = blip_image!("cannonball.png");
const EXPLODE_PNG:  &[u8] = blip_image!("explosion.png");
const PORT_BG_PNG:  &[u8] = blip_image!("port_bg.png");
const SEA_WAVE_PNG:   &[u8] = blip_image!("sea_wave.png");
const SEA_WAVE_B_PNG: &[u8] = blip_image!("sea_wave_b.png");
const CREW_PNG:       &[u8] = blip_image!("crew_figure.png");
const MAP_BG_PNG:     &[u8] = blip_image!("kattegat_map.png");

const CANNON_WAV:   &[u8] = blip_sound!("cannon_fire.wav");
const EXPLODE_WAV:  &[u8] = blip_sound!("explosion.wav");
const SPLASH_WAV:   &[u8] = blip_sound!("splash.wav");
const HULL_HIT_WAV: &[u8] = blip_sound!("hull_hit.wav");
const CLASH_WAV:    &[u8] = blip_sound!("boarding_clash.wav");
const COINS_WAV:    &[u8] = blip_sound!("coin_jingle.wav");
const LIFE_WAV:     &[u8] = blip_sound!("life_lost.wav");
const SEA_MUS_WAV:  &[u8] = blip_sound!("sea_music.wav");
const COMBAT_WAV:   &[u8] = blip_sound!("combat_music.wav");
const PORT_MUS_WAV:  &[u8] = blip_sound!("port_music.wav");
const AMBIENT_WAV:   &[u8] = blip_sound!("ocean_ambience.wav");

fn conf() -> blip::macroquad::window::Conf {
    window_conf("CANARIS", WIN_W, WIN_H)
}

#[blip::macroquad::main(conf)]
async fn main() {
    let mut blip = Blip::new(WIN_W, WIN_H);
    let mut g    = Game::new();

    let tex = Textures {
        player_a: load_png(PLAYER_A_PNG),
        player_b: load_png(PLAYER_B_PNG),
        enemy_a:  load_png(ENEMY_A_PNG),
        enemy_b:  load_png(ENEMY_B_PNG),
        ball:     load_png(BALL_PNG),
        explosion:load_png(EXPLODE_PNG),
        port_bg:  load_png(PORT_BG_PNG),
        sea_wave:   load_png(SEA_WAVE_PNG),
        sea_wave_b: load_png(SEA_WAVE_B_PNG),
        crew:       load_png(CREW_PNG),
        map_bg:     load_png(MAP_BG_PNG),
    };

    let sfx = Sounds {
        cannon_fire:    blip::audio::load_sound(CANNON_WAV).await,
        explosion:      blip::audio::load_sound(EXPLODE_WAV).await,
        splash:         blip::audio::load_sound(SPLASH_WAV).await,
        hull_hit:       blip::audio::load_sound(HULL_HIT_WAV).await,
        boarding_clash: blip::audio::load_sound(CLASH_WAV).await,
        coin_jingle:    blip::audio::load_sound(COINS_WAV).await,
        life_lost:      blip::audio::load_sound(LIFE_WAV).await,
        sea_music:      blip::audio::load_sound(SEA_MUS_WAV).await,
        combat_music:   blip::audio::load_sound(COMBAT_WAV).await,
        port_music:     blip::audio::load_sound(PORT_MUS_WAV).await,
        ocean_ambience: blip::audio::load_sound(AMBIENT_WAV).await,
    };

    play_music(&sfx.sea_music);
    play_ambient(&sfx.ocean_ambience);

    let mut shot_frame: u32 = 0;

    loop {
        let dt = blip.delta_time;

        // ── Screenshot autopilot ──────────────────────────────────────────────
        // When BLIP_SCREENSHOT_OUT is set: skip title, enter combat, fire one
        // cannonball, then let blip capture at frame 25.
        if blip.screenshot_mode {
            shot_frame += 1;
            match shot_frame {
                1 => {
                    // Skip title → sea → combat immediately
                    g.start_game();
                    // Enemy 0 is already active from start_game/spawn_enemies;
                    // set reload very high so it never fires during capture.
                    g.enemies[0].reload_t = 999.0;
                    g.enter_combat(0);
                }
                6 => {
                    // Fire one cannonball with a fixed (non-random) arc
                    g.cannonballs[0] = Cannonball {
                        active: true,
                        x:  COMBAT_PLAYER_X + PLAYER_W,
                        y:  g.player.y + PLAYER_H * 0.55,
                        vx: CANNON_SPEED,
                        vy: -CANNON_ARC_VY,
                        player: true,
                    };
                    g.player.reload_t = PLAYER_RELOAD;
                    g.player.cannons -= 1;
                }
                _ => {}
            }
        }

        match g.state {
            State::Title    => update_title(&mut g, dt),
            State::Sea      => update_sea(&mut g, dt, &sfx),
            State::Combat   => update_combat(&mut g, dt, &sfx),
            State::Boarding => update_boarding(&mut g, dt, &sfx),
            State::Port     => update_port(&mut g, dt, &sfx),
            State::Map      => update_map(&mut g, dt, &sfx),
            State::Dead     => update_dead(&mut g, dt, &sfx),
            State::GameOver => update_gameover(&mut g),
        }

        blip.clear(BLIP_BLACK);

        match g.state {
            State::Title    => draw_title(&blip, &g, &tex),
            State::Sea      => draw_sea(&blip, &g, &tex),
            State::Combat   => draw_combat(&blip, &g, &tex),
            State::Boarding => draw_boarding(&blip, &g, &tex),
            State::Port     => draw_port(&blip, &g, &tex),
            State::Map      => draw_map(&blip, &g, &tex),
            State::Dead     => draw_dead(&blip, &g, &tex),
            State::GameOver => draw_gameover(&blip, &g),
        }

        blip.next_frame(60).await;
    }
}

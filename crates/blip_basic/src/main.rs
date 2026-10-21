mod host;
mod interpreter;
mod music;
mod shared_math;
mod sounds;

use blip::{Blip, BLIP_BLACK};
use host::*;
use interpreter::Program;
use macroquad::{audio::load_sound_from_bytes, texture::Texture2D};
use music::Music;
use sounds::Sounds;
use std::collections::HashMap;

/// Every BLIP BASIC program draws on one 640x400 screen, a high-resolution Amiga's.
const WIDTH: i32 = 640;
const HEIGHT: i32 = 400;
include!(concat!(env!("OUT_DIR"), "/tape_gfx.rs"));
include!(concat!(env!("OUT_DIR"), "/rom_sfx.rs"));

fn conf() -> macroquad::window::Conf {
    blip::window_conf("BLIP BASIC", WIDTH, HEIGHT)
}

#[blip::macroquad::main(conf)]
async fn main() {
    #[cfg(target_arch = "wasm32")]
    std::panic::set_hook(Box::new(|error| {
        let message = error.to_string();
        unsafe { blip_basic_runtime_error(message.as_ptr(), message.len() as i32); }
    }));
    let mut blip = Blip::new(WIDTH, HEIGHT);
    blip.set_frame_retention(true);
    blip.show_committed(true);
    #[cfg(target_arch = "wasm32")]
    unsafe {
        blip_basic_ready();
    }
    let mut textures = HashMap::new();
    for (path, bytes) in TAPE_GFX {
        let tex =
            Texture2D::from_file_with_format(bytes, Some(macroquad::prelude::ImageFormat::Png));
        tex.set_filter(macroquad::texture::FilterMode::Nearest);
        textures.insert(path.to_string(), tex.clone());
        let alias = path
            .rsplit('/')
            .next()
            .unwrap_or(path)
            .trim_end_matches(".PNG")
            .to_ascii_lowercase();
        textures.entry(alias).or_insert(tex);
        let parts: Vec<_> = path.split('/').collect();
        if parts.len() >= 5 {
            let prefix = if parts[3] == "GALACTIC_DEFENDER" {
                "GALACTIC"
            } else {
                parts[3]
            };
            let qualified =
                format!("{}_{}", prefix, parts[4].trim_end_matches(".PNG")).to_ascii_lowercase();
            if let Some(texture) = textures.get(*path).cloned() {
                textures.entry(qualified).or_insert(texture);
            }
        }
    }
    let mut sounds = Sounds::new(SFX_NAMES.into_iter().zip(SFX_BYTES));
    let mut music = Music::new();
    let mut program = Program::new();
    let mut old_source = String::new();
    let mut reported_error = String::new();
    let mut program_loaded = false;
    loop {
        if let Some(source) = host_source() {
            if source != old_source {
                old_source = source.clone();
                music.stop();
                music.clear_files();
                blip.resize_canvas(WIDTH, HEIGHT);
                program.load(&source);
                for (path, bytes) in host_user_textures() {
                    let tex = Texture2D::from_file_with_format(
                        &bytes,
                        Some(macroquad::prelude::ImageFormat::Png),
                    );
                    tex.set_filter(macroquad::texture::FilterMode::Nearest);
                    textures.insert(path.clone(), tex.clone());
                    let alias = path
                        .rsplit('/')
                        .next()
                        .unwrap_or(&path)
                        .trim_end_matches(".PNG")
                        .to_ascii_lowercase();
                    textures.entry(alias).or_insert(tex);
                }
                for (path, bytes) in host_user_music() {
                    if let Ok(sound) = load_sound_from_bytes(&bytes).await {
                        music.insert_file(path, sound);
                    }
                }
                reported_error.clear();
                program_loaded = true;
            }
        } else if !old_source.is_empty() {
            old_source.clear();
            music.stop();
            program = Program::new();
            reported_error.clear();
        }
        #[cfg(target_arch = "wasm32")]
        unsafe {
            if blip_basic_export_requested() != 0 {
                let game = match blip_basic_export_game() {
                    1 => "ADDER",
                    2 => "BOUNCER",
                    3 => "BRAWLER",
                    4 => "BUBBLER",
                    5 => "GALACTIC_DEFENDER",
                    6 => "METEORS",
                    7 => "RALLY",
                    8 => "SERPENT",
                    9 => "SKY_RAIDER",
                    _ => "",
                };
                for (path, bytes) in TAPE_GFX {
                    if !game.is_empty() && path.starts_with(&format!("/TAPE/GFX/{game}/")) {
                        blip_basic_export_rom_asset(
                            path.as_ptr(),
                            path.len() as i32,
                            bytes.as_ptr(),
                            bytes.len() as i32,
                        );
                    }
                }
                blip_basic_export_done();
            }
        }
        #[cfg(not(target_arch = "wasm32"))]
        if blip::bot::active() {
            if let Some(error) = &program.error {
                eprintln!("BASIC ERROR {error}");
                std::process::exit(1);
            }
            if !program.active {
                blip::bot::finish("ended");
            }
            blip::bot::hold(&[]);
        }
        program.tick(&mut blip, &textures, &mut sounds);
        if let Err(error) = sounds.flush().await {
            program.error = Some(error);
            program.active = false;
        }
        #[cfg(target_arch = "wasm32")]
        if let Some((frequency, seconds, volume)) = program.sound_request.take() {
            unsafe {
                blip_basic_sound(frequency, seconds, volume);
            }
        }
        if let Some(path) = program.music_file_request.take() {
            if let Err(error) = music.play_file(path) {
                program.error = Some(error);
            }
        }
        if let Some(request) = program.music_request.take() {
            music.play(request).await;
        }
        music.warm_up().await;
        #[cfg(target_arch = "wasm32")]
        if let Some(error) = &program.error {
            if *error != reported_error {
                reported_error = error.clone();
                unsafe {
                    blip_basic_runtime_error(error.as_ptr(), error.len() as i32);
                }
            }
        }
        if !program.active {
            blip.clear(BLIP_BLACK);
        }
        program.draw_output(&blip);
        // A frame the program has not finished drawing waits for its WAIT.
        if program.yielded || !program.active {
            blip.commit_frame();
        }
        blip.next_frame(60).await;
        if program_loaded {
            program_loaded = false;
            #[cfg(target_arch = "wasm32")]
            unsafe {
                blip_basic_program_loaded();
            }
        }
    }
}

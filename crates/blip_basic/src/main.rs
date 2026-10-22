mod host;
mod interpreter;
mod shared_math;
mod sounds;

use blip::{Blip, BLIP_BLACK};
use host::*;
use interpreter::Program;
use blip::BlipTex;
use macroquad::texture::Texture2D;
use sounds::Sounds;
use std::collections::HashMap;

/// Every BLIP BASIC program draws on one 640x400 screen, a high-resolution Amiga's.
const WIDTH: i32 = 640;
const HEIGHT: i32 = 400;

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
    let mut sounds = Sounds::default();
    let mut program = Program::new();
    let mut old_source = String::new();
    let mut reported_error = String::new();
    let mut program_loaded = false;
    loop {
        if let Some(source) = host_source() {
            if source != old_source {
                old_source = source.clone();
                blip.resize_canvas(WIDTH, HEIGHT);
                program.load(&source);
                load_files(&source, &mut textures, &mut sounds).await;
                reported_error.clear();
                program_loaded = true;
            }
        } else if !old_source.is_empty() {
            old_source.clear();
            sounds.stop_music();
            program = Program::new();
            reported_error.clear();
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

/// A program's pictures and sounds: the user's own from /USER, and every file in
/// each tape folder (/TAPE/<GAME>/ASSETS/) its source names. A copy's folder
/// (/USER/<GAME>/ASSETS/) holds what the user changed; the rest comes from the
/// tape's. A tape's tunes, a WAV with an XM beside it, are fetched only when played.
async fn load_files(source: &str, textures: &mut HashMap<String, BlipTex>, sounds: &mut Sounds) {
    textures.clear();
    sounds.clear();
    let mut files = host_user_files();
    let mut folders: Vec<(&str, &str)> = ["/TAPE/", "/USER/"]
        .iter()
        .flat_map(|root| source.match_indices(root))
        .filter_map(|(at, _)| {
            let game = source[at + 6..].split('/').next()?;
            let ok = !game.is_empty() && game.chars().all(|c| c.is_ascii_alphanumeric() || c == '_');
            let folder = &source[at..at + 6 + game.len() + "/ASSETS/".len()];
            (ok && source[at + 6 + game.len()..].starts_with("/ASSETS/")).then_some((folder, game))
        })
        .collect();
    folders.sort();
    folders.dedup();
    for (folder, game) in folders {
        let tape = format!("/TAPE/{game}/ASSETS/");
        let Ok(list) = macroquad::file::load_string(&tape_url(&format!("{tape}manifest.json"))).await else { continue };
        let names: Vec<&str> = list.split('"').skip(1).step_by(2).collect();
        for name in &names {
            let path = format!("{folder}{name}");
            if files.iter().any(|(user, _)| *user == path) {
                continue;
            }
            let url = tape_url(&format!("{tape}{name}"));
            let tune = name.strip_suffix(".WAV").is_some_and(|stem| names.contains(&format!("{stem}.XM").as_str()));
            if tune {
                sounds.add_later(path, url);
            } else if name.ends_with(".PNG") || name.ends_with(".WAV") {
                if let Ok(bytes) = macroquad::file::load_file(&url).await {
                    files.push((path, bytes));
                }
            }
        }
    }
    for (path, bytes) in files {
        if path.ends_with(".PNG") {
            let tex = Texture2D::from_file_with_format(&bytes, Some(macroquad::prelude::ImageFormat::Png));
            tex.set_filter(macroquad::texture::FilterMode::Nearest);
            textures.insert(path, tex);
        } else if path.ends_with(".WAV") {
            sounds.add(path, bytes);
        }
    }
}

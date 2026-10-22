use std::{collections::{BTreeMap, HashMap}, env, fs, path::PathBuf};

use blip::Asset;

// Every game's pictures and effects, from its own gfx.rs and tunes.rs.
#[path = "../adder/src/gfx.rs"]
mod adder_gfx;
#[path = "../adder/src/tunes.rs"]
mod adder_tunes;
#[path = "../bouncer/src/gfx.rs"]
mod bouncer_gfx;
#[path = "../bouncer/src/tunes.rs"]
mod bouncer_tunes;
#[path = "../brawler/src/gfx.rs"]
mod brawler_gfx;
#[path = "../brawler/src/tunes.rs"]
mod brawler_tunes;
#[path = "../bubbler/src/gfx.rs"]
mod bubbler_gfx;
#[path = "../bubbler/src/tunes.rs"]
mod bubbler_tunes;
#[path = "../defender/src/gfx.rs"]
mod defender_gfx;
#[path = "../defender/src/tunes.rs"]
mod defender_tunes;
#[path = "../meteors/src/gfx.rs"]
mod meteors_gfx;
#[path = "../meteors/src/tunes.rs"]
mod meteors_tunes;
#[path = "../rally/src/gfx.rs"]
mod rally_gfx;
#[path = "../rally/src/tunes.rs"]
mod rally_tunes;
#[path = "../serpent/src/gfx.rs"]
mod serpent_gfx;
#[path = "../serpent/src/tunes.rs"]
mod serpent_tunes;
#[path = "../raider/src/gfx.rs"]
mod raider_gfx;
#[path = "../raider/src/tunes.rs"]
mod raider_tunes;

/// A game's pictures, then its effects.
fn both(mut gfx: Vec<Asset>, sounds: Vec<Asset>) -> Vec<Asset> {
    gfx.extend(sounds);
    gfx
}

fn main() {
    let root = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").expect("manifest dir"))
        .parent().expect("crates dir").parent().expect("workspace root").to_path_buf();
    let out = PathBuf::from(env::var_os("OUT_DIR").expect("OUT_DIR"));
    let assets_dir = out.join("assets");
    for game in ["adder", "bouncer", "brawler", "bubbler", "defender", "meteors", "rally", "serpent", "raider"] {
        println!("cargo:rerun-if-changed={}", root.join(format!("crates/{game}/src/gfx.rs")).display());
        println!("cargo:rerun-if-changed={}", root.join(format!("crates/{game}/src/tunes.rs")).display());
    }
    println!("cargo:rerun-if-changed=build.rs");

    let games: [(&str, Vec<Asset>); 9] = [
        ("ADDER", both(adder_gfx::generate(), adder_tunes::sounds())),
        ("BOUNCER", both(bouncer_gfx::generate(), bouncer_tunes::sounds())),
        ("BRAWLER", both(brawler_gfx::generate(), brawler_tunes::sounds())),
        ("BUBBLER", both(bubbler_gfx::generate(), bubbler_tunes::sounds())),
        ("DEFENDER", both(defender_gfx::generate(), defender_tunes::sounds())),
        ("METEORS", both(meteors_gfx::generate(), meteors_tunes::sounds())),
        ("RALLY", both(rally_gfx::generate(), rally_tunes::sounds())),
        ("SERPENT", both(serpent_gfx::generate(), serpent_tunes::sounds())),
        ("RAIDER", both(raider_gfx::generate(), raider_tunes::sounds())),
    ];
    let mut manifest = BTreeMap::<&str, Vec<String>>::new();
    let mut sfx = HashMap::<String, String>::new();
    let mut gfx_source = String::from("pub const TAPE_GFX: &[(&str, &[u8])] = &[\n");
    for (game, assets) in games {
        for (relative, bytes) in assets {
            if !relative.starts_with("images/") && !relative.starts_with("sounds/") { continue; }
            let name = relative.rsplit('/').next().expect("asset name");
            let path = assets_dir.join(game.to_ascii_lowercase()).join(name);
            fs::create_dir_all(path.parent().expect("asset parent")).expect("create asset directory");
            fs::write(&path, bytes).expect("write generated asset");
            if relative.starts_with("images/") && name.ends_with(".png") {
                let filename = name.to_ascii_uppercase();
                let virtual_path = format!("/TAPE/GFX/{game}/{filename}");
                gfx_source.push_str(&format!("    ({virtual_path:?}, include_bytes!({:?})),\n", path.to_string_lossy()));
                manifest.entry(game).or_default().push(filename);
            } else if relative.starts_with("sounds/") && name.ends_with(".wav") {
                sfx.insert(format!("{game}/{}", name.trim_end_matches(".wav")), path.to_string_lossy().into_owned());
            }
        }
    }
    gfx_source.push_str("];\n");
    fs::write(out.join("tape_gfx.rs"), gfx_source).expect("write GFX registry");

    let sound_names = [
        ("wall_hit", "BOUNCER/wall_hit"), ("paddle_hit_0", "BOUNCER/paddle_hit_0"), ("paddle_hit_1", "BOUNCER/paddle_hit_1"),
        ("paddle_hit_2", "BOUNCER/paddle_hit_2"), ("brick_break_0", "BOUNCER/brick_break_0"), ("brick_break_1", "BOUNCER/brick_break_1"),
        ("brick_break_2", "BOUNCER/brick_break_2"), ("life_lost", "BOUNCER/life_lost"), ("win", "BOUNCER/win"), ("bomb", "BOUNCER/bomb"),
        ("shoot", "DEFENDER/shoot"), ("explosion", "DEFENDER/explosion"), ("dive", "DEFENDER/dive"),
        ("game_over", "DEFENDER/game_over"), ("march1", "DEFENDER/march1"), ("march2", "DEFENDER/march2"),
        ("march3", "DEFENDER/march3"), ("march4", "DEFENDER/march4"), ("level_clear", "DEFENDER/level_clear"),
        ("ufo_siren", "DEFENDER/ufo_siren"), ("pickup_good", "BOUNCER/pickup_good"), ("pickup_bad", "BOUNCER/pickup_bad"),
        ("pickup_life", "BOUNCER/pickup_life"), ("brick_hit_0", "BOUNCER/brick_hit_0"), ("brick_hit_1", "BOUNCER/brick_hit_1"),
        ("brick_hit_2", "BOUNCER/brick_hit_2"), ("laser_charge", "DEFENDER/laser_charge"), ("laser_blast", "DEFENDER/laser_blast"),
        ("adder/eat", "ADDER/eat"), ("adder/through", "ADDER/through"), ("adder/strike", "ADDER/strike"), ("adder/pit", "ADDER/pit"), ("adder/game_over", "ADDER/game_over"),
        ("serpent/eat", "SERPENT/eat"), ("serpent/bonus", "SERPENT/bonus"), ("serpent/bonus_eat", "SERPENT/bonus_eat"), ("serpent/level", "SERPENT/level"), ("serpent/game_over", "SERPENT/game_over"),
        ("rally/wall", "RALLY/wall"), ("rally/hit_l", "RALLY/hit_l"), ("rally/hit_r", "RALLY/hit_r"), ("rally/score_l", "RALLY/score_l"), ("rally/score_r", "RALLY/score_r"),
        ("meteors/fire", "METEORS/fire"), ("meteors/thrust", "METEORS/thrust"), ("meteors/bang_small", "METEORS/bang_small"), ("meteors/bang_medium", "METEORS/bang_medium"), ("meteors/bang_large", "METEORS/bang_large"), ("meteors/hyperspace", "METEORS/hyperspace"), ("meteors/ship_explosion", "METEORS/ship_explosion"), ("meteors/saucer_big", "METEORS/saucer_big"), ("meteors/saucer_small", "METEORS/saucer_small"), ("meteors/extra_life", "METEORS/extra_life"),
        ("raider/burst1_1", "RAIDER/burst1_1"), ("raider/burst2_1", "RAIDER/burst2_1"), ("raider/burst3_1", "RAIDER/burst3_1"), ("raider/burst4_1", "RAIDER/burst4_1"), ("raider/burst5_1", "RAIDER/burst5_1"), ("raider/enemy_explode0", "RAIDER/enemy_explode0"), ("raider/enemy_gun", "RAIDER/enemy_gun"), ("raider/player_explode0", "RAIDER/player_explode0"), ("raider/player_hit", "RAIDER/player_hit"), ("raider/stage_clear", "RAIDER/stage_clear"), ("raider/boss_warning", "RAIDER/boss_warning"), ("raider/game_over", "RAIDER/game_over"), ("raider/powerup2", "RAIDER/powerup2"), ("raider/powerup3", "RAIDER/powerup3"), ("raider/powerup4", "RAIDER/powerup4"), ("raider/max_power", "RAIDER/max_power"),
        ("brawler/hit_light", "BRAWLER/hit_light"), ("brawler/hit_light2", "BRAWLER/hit_light2"), ("brawler/hit_heavy", "BRAWLER/hit_heavy"), ("brawler/block", "BRAWLER/block"), ("brawler/whoosh", "BRAWLER/whoosh"), ("brawler/land", "BRAWLER/land"), ("brawler/ko", "BRAWLER/ko"), ("brawler/swing", "BRAWLER/swing"), ("brawler/bell", "BRAWLER/bell"),
        ("bubbler/blow", "BUBBLER/blow"), ("bubbler/pop", "BUBBLER/pop"), ("bubbler/trap", "BUBBLER/trap"), ("bubbler/defeated", "BUBBLER/defeated"), ("bubbler/fruit", "BUBBLER/fruit"), ("bubbler/jump", "BUBBLER/jump"), ("bubbler/lost", "BUBBLER/lost"), ("bubbler/bounce", "BUBBLER/bounce"), ("bubbler/sparkle", "BUBBLER/sparkle"),
    ];
    let mut sfx_source = format!("pub const SFX_NAMES: [&str; {}] = [", sound_names.len());
    let mut sfx_bytes = format!("pub const SFX_BYTES: [&[u8]; {}] = [\n", sound_names.len());
    for (name, key) in sound_names {
        let path = sfx.get(key).unwrap_or_else(|| panic!("missing generated sound {key}"));
        sfx_source.push_str(&format!("{name:?},"));
        sfx_bytes.push_str(&format!("include_bytes!({path:?}),\n"));
    }
    sfx_source.push_str("];\n");
    sfx_source.push_str(&sfx_bytes);
    sfx_source.push_str("];\n");
    fs::write(out.join("rom_sfx.rs"), sfx_source).expect("write sound registry");

    let mut json = String::from("{\n");
    for (index, (game, files)) in manifest.iter().enumerate() {
        let entries = files.iter().map(|file| format!("\"{file}\"")).collect::<Vec<_>>().join(", ");
        json.push_str(&format!("  \"{game}\": [{entries}]{}\n", if index + 1 == manifest.len() { "" } else { "," }));
    }
    json.push_str("}\n");
    fs::write(root.join("web/TAPE/GFX/manifest.json"), json).expect("write GFX manifest");
}

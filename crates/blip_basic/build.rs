use std::{collections::{BTreeMap, HashMap}, env, fs, path::PathBuf};

fn main() {
    let root = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").expect("manifest dir"))
        .parent().expect("crates dir").parent().expect("workspace root").to_path_buf();
    let out = PathBuf::from(env::var_os("OUT_DIR").expect("OUT_DIR"));
    let assets_dir = out.join("assets");
    println!("cargo:rerun-if-changed={}", root.join("crates/blip_assets/src").display());
    println!("cargo:rerun-if-changed=build.rs");

    let games: [(&str, Vec<blip_assets::Asset>); 9] = [
        ("ADDER", blip_assets::adder::generate()),
        ("BOUNCER", blip_assets::bouncer::generate()),
        ("BRAWLER", blip_assets::brawler::generate()),
        ("BUBBLER", blip_assets::bubbler::generate()),
        ("GALACTIC_DEFENDER", blip_assets::galactic_defender::generate()),
        ("METEORS", blip_assets::meteors::generate()),
        ("RALLY", blip_assets::rally::generate()),
        ("SERPENT", blip_assets::serpent::generate()),
        ("SKY_RAIDER", blip_assets::sky_raider::generate()),
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
    for (name, index) in [("blow", 0), ("pop", 1), ("trap", 2), ("defeated", 3), ("fruit", 4), ("jump", 5), ("lost", 6), ("bounce", 7), ("sparkle", 8)] {
        let bytes = blip_assets::bubbler::sfx_wav(index);
        let path = assets_dir.join("bubbler").join(format!("{name}.wav"));
        fs::create_dir_all(path.parent().expect("asset parent")).expect("create asset directory");
        fs::write(&path, bytes).expect("write generated Bubbler sound");
        sfx.insert(format!("BUBBLER/{name}"), path.to_string_lossy().into_owned());
    }
    gfx_source.push_str("];\n");
    fs::write(out.join("tape_gfx.rs"), gfx_source).expect("write GFX registry");

    let sound_names = [
        ("wall_hit", "BOUNCER/wall_hit"), ("paddle_hit_0", "BOUNCER/paddle_hit_0"), ("paddle_hit_1", "BOUNCER/paddle_hit_1"),
        ("paddle_hit_2", "BOUNCER/paddle_hit_2"), ("brick_break_0", "BOUNCER/brick_break_0"), ("brick_break_1", "BOUNCER/brick_break_1"),
        ("brick_break_2", "BOUNCER/brick_break_2"), ("life_lost", "BOUNCER/life_lost"), ("win", "BOUNCER/win"), ("bomb", "BOUNCER/bomb"),
        ("shoot", "GALACTIC_DEFENDER/shoot"), ("explosion", "GALACTIC_DEFENDER/explosion"), ("dive", "GALACTIC_DEFENDER/dive"),
        ("game_over", "GALACTIC_DEFENDER/game_over"), ("march1", "GALACTIC_DEFENDER/march1"), ("march2", "GALACTIC_DEFENDER/march2"),
        ("march3", "GALACTIC_DEFENDER/march3"), ("march4", "GALACTIC_DEFENDER/march4"), ("level_clear", "GALACTIC_DEFENDER/level_clear"),
        ("ufo_siren", "GALACTIC_DEFENDER/ufo_siren"), ("pickup_good", "BOUNCER/pickup_good"), ("pickup_bad", "BOUNCER/pickup_bad"),
        ("pickup_life", "BOUNCER/pickup_life"), ("brick_hit_0", "BOUNCER/brick_hit_0"), ("brick_hit_1", "BOUNCER/brick_hit_1"),
        ("brick_hit_2", "BOUNCER/brick_hit_2"), ("laser_charge", "GALACTIC_DEFENDER/laser_charge"), ("laser_blast", "GALACTIC_DEFENDER/laser_blast"),
        ("adder/eat", "ADDER/eat"), ("adder/through", "ADDER/through"), ("adder/strike", "ADDER/strike"), ("adder/pit", "ADDER/pit"), ("adder/game_over", "ADDER/game_over"),
        ("serpent/eat", "SERPENT/eat"), ("serpent/bonus", "SERPENT/bonus"), ("serpent/bonus_eat", "SERPENT/bonus_eat"), ("serpent/level", "SERPENT/level"), ("serpent/game_over", "SERPENT/game_over"),
        ("rally/wall", "RALLY/wall"), ("rally/hit_l", "RALLY/hit_l"), ("rally/hit_r", "RALLY/hit_r"), ("rally/score_l", "RALLY/score_l"), ("rally/score_r", "RALLY/score_r"),
        ("meteors/fire", "METEORS/fire"), ("meteors/thrust", "METEORS/thrust"), ("meteors/bang_small", "METEORS/bang_small"), ("meteors/bang_medium", "METEORS/bang_medium"), ("meteors/bang_large", "METEORS/bang_large"), ("meteors/hyperspace", "METEORS/hyperspace"), ("meteors/ship_explosion", "METEORS/ship_explosion"), ("meteors/saucer_big", "METEORS/saucer_big"), ("meteors/saucer_small", "METEORS/saucer_small"),
        ("sky_raider/burst1_1", "SKY_RAIDER/burst1_1"), ("sky_raider/burst2_1", "SKY_RAIDER/burst2_1"), ("sky_raider/burst3_1", "SKY_RAIDER/burst3_1"), ("sky_raider/burst4_1", "SKY_RAIDER/burst4_1"), ("sky_raider/burst5_1", "SKY_RAIDER/burst5_1"), ("sky_raider/enemy_explode0", "SKY_RAIDER/enemy_explode0"), ("sky_raider/enemy_gun", "SKY_RAIDER/enemy_gun"), ("sky_raider/player_explode0", "SKY_RAIDER/player_explode0"), ("sky_raider/player_hit", "SKY_RAIDER/player_hit"), ("sky_raider/stage_clear", "SKY_RAIDER/stage_clear"), ("sky_raider/boss_warning", "SKY_RAIDER/boss_warning"), ("sky_raider/game_over", "SKY_RAIDER/game_over"), ("sky_raider/powerup2", "SKY_RAIDER/powerup2"), ("sky_raider/powerup3", "SKY_RAIDER/powerup3"), ("sky_raider/powerup4", "SKY_RAIDER/powerup4"), ("sky_raider/max_power", "SKY_RAIDER/max_power"),
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

use super::*;

const TAPE: &str = include_str!("../../../../web/TAPE/BRAWLER.BLIP");

fn row(name: &str) -> Vec<&str> {
    TAPE.lines().find_map(|line| {
        let (_, data) = line.split_once(" DATA ")?;
        data.starts_with(&format!("\"{name}\","))
            .then(|| data.split(',').map(|s| s.trim().trim_matches('"')).collect())
    }).unwrap_or_else(|| panic!("BASIC has no row for {name}"))
}

fn title_moves(side: &str) -> [usize; 6] {
    let prefix = format!("TITLE_{side}(");
    let mut ids = [0; 6];
    for line in TAPE.lines() {
        let Some((_, source)) = line.split_once(' ') else { continue };
        let Some(rest) = source.strip_prefix(&prefix) else { continue };
        let Some((index, id)) = rest.split_once(")=") else { continue };
        ids[index.parse::<usize>().unwrap() - 1] = id.parse().unwrap();
    }
    ids
}

fn basic_move_id(id: MoveId) -> usize {
    match id {
        MoveId::LowPunch => 1,
        MoveId::HighPunch => 2,
        MoveId::LowKick => 3,
        MoveId::HighKick => 4,
        MoveId::Special => 5,
        MoveId::JumpKick => 6,
        MoveId::Sweep => 7,
        MoveId::Uppercut => 8,
        _ => panic!("title attract uses an unsupported move: {id:?}"),
    }
}

#[test]
fn basic_and_rust_title_attracts_share_their_six_move_cycles() {
    let rust = |moves: &[MoveId; 6]| moves.map(basic_move_id);
    assert_eq!(title_moves("LEFT"), rust(&draw::TITLE_LEFT_MOVES));
    assert_eq!(title_moves("RIGHT"), rust(&draw::TITLE_RIGHT_MOVES));
}

#[test]
fn basic_and_rust_title_attracts_rotate_the_same_fighter_pairs() {
    assert_eq!(draw::TITLE_TURN, 4.0);
    assert_eq!(FIGHTERS.len() / 2, 7);
    for source in ["TURN=INT(NOW/4)", "W=TURN MOD FIGHTERS+1", "V=(W+6) MOD FIGHTERS+1"] {
        assert!(TAPE.lines().any(|line| line.split_once(' ').is_some_and(|(_, code)| code.trim() == source)),
            "BASIC title is missing shared attract timing: {source}");
    }
}

#[test]
fn basic_cassette_ships_the_original_walkout_and_hood_music() {
    const MANIFEST: &str = include_str!("../../../../web/TAPE/BRAWLER/ASSETS/manifest.json");
    let assets = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../web/TAPE/BRAWLER/ASSETS");
    for name in ["WALKOUT.WAV", "WALKOUT.XM", "HOOD.WAV", "HOOD.XM"] {
        assert!(MANIFEST.contains(&format!("\"{name}\"")), "missing {name} from BASIC's asset manifest");
        let bytes = std::fs::read(assets.join(name)).unwrap_or_else(|_| panic!("BASIC cassette is missing {name}"));
        let header = if name.ends_with(".WAV") { &b"RIFF"[..] } else { &b"Extended Module: "[..] };
        assert!(bytes.starts_with(header), "{name} has an invalid header");
    }
    assert!(TAPE.contains("PLAY_MUSIC ASSETS$+\"WALKOUT.WAV\""));
    assert!(TAPE.contains("PLAY_MUSIC ASSETS$+THEME$(STAGE)+\".WAV\""));
}

#[test]
fn basic_uses_the_alternate_kick_and_whiff_stumble_pose_rows() {
    assert!(TAPE.contains("IF ALT_KICK(P)=1 AND MV(P)=4 THEN ROW=18"));
    assert!(TAPE.contains("STUMBLE_T(P)=18"));
    assert!(TAPE.contains("ROW=4:CELL=MIN(5,INT((18-STUMBLE_T(P))*6/18))"));
    assert_eq!(crate::export::POSES.len(), 19);
    assert_eq!(crate::export::POSES[0], (Act::Idle, MoveId::LowPunch));
    assert_eq!(crate::export::POSES[18], (Act::Attack, MoveId::HighKick));
    assert!(crate::export::sample(0, 18, 3).alt_kick);
}

#[test]
fn basic_and_rust_share_the_roster_and_move_data() {
    for (who, a) in FIGHTERS.iter().enumerate() {
        let values = row(a.name);
        assert_eq!(values[7], a.special_name);
        assert_eq!(values[8], a.power_desc);
        for (i, expected) in [(1, a.health as f32), (2, a.walk), (3, a.power), (4, a.reach),
            (5, a.jump_scale), (9, home_of(who) as f32 + 1.0), (10, a.size), (16, a.bulk),
            (17, a.boss as u8 as f32)] {
            let actual: f32 = values[i].parse().unwrap();
            assert!((actual - expected).abs() < 0.001, "{} field {i}: {actual} != {expected}", a.name);
        }
    }
    for (name, id) in [("LOW PUNCH", MoveId::LowPunch), ("HIGH PUNCH", MoveId::HighPunch),
        ("LOW KICK", MoveId::LowKick), ("HIGH KICK", MoveId::HighKick), ("SPECIAL", MoveId::Special),
        ("JUMP KICK", MoveId::JumpKick), ("SWEEP", MoveId::Sweep), ("UPPERCUT", MoveId::Uppercut)] {
        let values = row(name);
        let m = move_data(id);
        for (i, expected) in [(1, m.startup), (2, m.active), (3, m.recovery), (4, m.damage as f32),
            (5, m.hitstun), (6, m.blockstun), (7, m.reach), (8, m.height)] {
            let actual: f32 = values[i].parse().unwrap();
            assert!((actual - expected).abs() < 0.001, "{name} field {i}: {actual} != {expected}");
        }
    }
}

#[test]
fn chooser_copy_fits_basic_and_rust_stats_panels() {
    let basic_width = 268.0 - 16.0;
    assert!(TAPE.contains("FILL_RECT(298,100,268,136"), "BASIC needs a bounded stats panel");
    assert!(222.0 + 7.0 * 2.0 <= 100.0 + 136.0,
        "the last BASIC stat row must stay above the panel's bottom border");
    assert!(TAPE.contains("CHOOSE YOUR FIGHTER"));
    assert!(TAPE.contains(r#"DRAW_TEXT(NAME$(I),CX+37,CY+30,1,TEXT$,"CENTER")"#));
    assert!(TAPE.contains("POWER_TEXT$+\"X\""));
    assert!(TAPE.contains("POWER_TENTH=INT(POWER(PICK)*10+0.499999)"));
    for (who, a) in FIGHTERS.iter().enumerate() {
        let stage = STAGE_NAMES[home_of(who)];
        let desc_size = if a.power_desc.len() > 21 { 1.0 } else { 2.0 };
        for (label, size) in [(a.name, 3.0), (a.special_name, 2.0), (a.power_desc, desc_size),
            (stage, 2.0), ("PWR", 2.0), ("SPD", 2.0), ("HP", 2.0)] {
            assert!(draw::text_w(label, size) <= basic_width,
                "{} BASIC label {label:?} exceeds its stats panel", a.name);
        }
        for value in [format!("{:.1}X", a.power), format!("{}", a.walk as i32), format!("{}", a.health)] {
            assert!(draw::text_w(&value, 2.0) <= basic_width,
                "{} BASIC stat {value:?} exceeds its stats panel", a.name);
        }
        assert!(draw::text_w(a.name, 1.0) <= 70.0, "{} BASIC roster name exceeds its card", a.name);
        for plate_w in [268.0, 184.0] {
            let inner_w = plate_w - 16.0;
            for label in [a.special_name, a.power_desc, stage] {
                let size = draw::select_info_size(label, inner_w);
                assert!(draw::text_w(label, size) <= inner_w,
                    "{} Rust label {label:?} exceeds its {plate_w}px chooser plate", a.name);
            }
        }
    }
}

#[test]
fn every_exported_pose_has_a_body_and_unclipped_transparent_edges() {
    let mut texture_bytes = 0;
    for a in FIGHTERS {
        let values = row(a.name);
        let cw: usize = values[12].parse().unwrap();
        let ch: usize = values[13].parse().unwrap();
        let file = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("art").join(format!("{}.png", a.name.to_ascii_lowercase()));
        let mut reader = png::Decoder::new(std::fs::File::open(file).unwrap()).read_info().unwrap();
        let mut pixels = vec![0; reader.output_buffer_size()];
        let info = reader.next_frame(&mut pixels).unwrap();
        let w = info.width as usize;
        assert_eq!(info.color_type, png::ColorType::Rgba);
        assert_eq!(w, cw * crate::export::FRAMES, "{} sheet width", a.name);
        assert_eq!(info.height as usize, ch * crate::export::POSES.len(), "{} sheet height", a.name);
        assert!(info.width <= 4096 && info.height <= 4096, "{} exceeds a phone's texture budget", a.name);
        texture_bytes += pixels.len();
        for pose in 0..crate::export::POSES.len() {
            for frame in 0..crate::export::FRAMES {
                let alpha = |x: usize, y: usize| pixels[((pose * ch + y) * w + frame * cw + x) * 4 + 3];
                let border = (0..cw).any(|x| alpha(x, 0) > 0 || alpha(x, ch - 1) > 0)
                    || (0..ch).any(|y| alpha(0, y) > 0 || alpha(cw - 1, y) > 0);
                assert!(!border, "{} pose {pose} frame {frame} is clipped", a.name);
                assert!((0..ch).any(|y| (0..cw).any(|x| alpha(x, y) > 0)), "{} pose {pose} frame {frame} is empty", a.name);
            }
        }
        let cell = |pose: usize, frame: usize| {
            (0..ch).flat_map(|y| {
                let start = (pose * ch + y) * w * 4 + frame * cw * 4;
                pixels[start..start + cw * 4].iter().copied()
            }).collect::<Vec<_>>()
        };
        for frame in 0..crate::export::FRAMES {
            assert_ne!(cell(10, frame), cell(18, frame),
                "{} high-kick frame {frame} has no alternate-leg pose", a.name);
        }
    }
    assert!(texture_bytes < 160 * 1024 * 1024, "fighter sheets use {} MiB", texture_bytes / 1024 / 1024);
}

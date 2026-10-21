use super::*;
use super::{
    color::value_color,
    syntax::{assignment_eq, method_call, split_args},
};
use macroquad::color::Color;

#[test]
fn separators_ignore_quoted_text_and_nested_expressions() {
    assert_eq!(
        split_args("DISTANCE(0,0,3,4),\"A,B\",\"SAY \"\"HI,THERE\"\"\""),
        ["DISTANCE(0,0,3,4)", "\"A,B\"", "\"SAY \"\"HI,THERE\"\"\""]
    );
    assert_eq!(
        split_statements("PRINT \"A:B\":IF X THEN PRINT \"C:D\":X=2"),
        ["PRINT \"A:B\"", "IF X THEN PRINT \"C:D\":X=2"]
    );
    let assignment = "A(\"É=1\")=3";
    assert_eq!(assignment_eq(assignment), assignment.find("=3"));
    for comparison in ["A<=B", "A>=B", "A==B", "A(1=2)"] {
        assert_eq!(assignment_eq(comparison), None, "{comparison}");
    }
}
#[test]
fn comments_own_colons_quotes_and_executable_looking_text() {
    for comment in [
        "REM INPUT: B=2:GOTO 800",
        "rem DON'T SPLIT: PRINT \"HI\"",
        "' STATE: A=1",
        "REM:END",
    ] {
        assert_eq!(split_statements(comment), [comment]);
        assert_eq!(
            split_statements(&format!("A=1:{comment}")),
            ["A=1", comment]
        );
    }
    assert_eq!(split_statements("REMAINDER=1:A=2"), ["REMAINDER=1", "A=2"]);
}

#[test]
fn conditional_after_an_assignment_keeps_the_rest_of_the_line() {
    assert_eq!(
        split_statements("FRAME_TIME=0:IF QN>0 THEN DIR_X=QX(1):QN=QN-1"),
        ["FRAME_TIME=0", "IF QN>0 THEN DIR_X=QX(1):QN=QN-1"]
    );
    assert_eq!(
        split_statements("A=1:IF B THEN C=2:IF D THEN E=3:F=4"),
        ["A=1", "IF B THEN C=2:IF D THEN E=3:F=4"]
    );
    assert_eq!(
        split_statements("DRAW_TEXT(\"IF A:B\",1,2,3,\"WHITE\"):WAIT 16"),
        ["DRAW_TEXT(\"IF A:B\",1,2,3,\"WHITE\")", "WAIT 16"]
    );
}

#[test]
fn random_reaches_every_spawn_slot_without_exceeding_its_limit() {
    let mut program = Program::new();
    let mut slots = [0; 20];
    for _ in 0..20_000 {
        let value = program.random(20.0);
        assert!((0.0..20.0).contains(&value));
        slots[value.floor() as usize] += 1;
    }
    assert!(
        slots.iter().all(|count| (800..1200).contains(count)),
        "{slots:?}"
    );
}

#[test]
fn bouncer_source_loads_as_numbered_program() {
    let mut p = Program::new();
    p.load(include_str!("../../../../web/TAPE/BOUNCER.BLIP"));
    assert!(p.lines.len() > 80);
    assert_eq!(p.lines[0].0, 1);
    assert_eq!(p.lines.last().unwrap().0, 2510);
    assert!(p.lines.windows(2).all(|pair| pair[0].0 < pair[1].0));
    let source = p
        .lines
        .iter()
        .map(|(_, line)| line.as_str())
        .collect::<Vec<_>>()
        .join(" ");
    assert!(
        source.contains("QUIET>5")
            && source.contains("TARGETX=6+C*30+14.5")
            && source.contains("GOSUB 2400")
            && source.contains("ANGLE=0.024")
    );
}
#[test]
fn defender_source_loads_as_numbered_program() {
    let mut p = Program::new();
    p.load(include_str!("../../../../web/TAPE/DEFENDER.BLIP"));
    assert!(p.lines.len() > 80);
    assert_eq!(p.lines[0].0, 1);
    assert_eq!(p.lines.last().unwrap().0, 1660);
}
#[test]
fn sky_raider_source_loads_as_numbered_program() {
    let mut p = Program::new();
    p.load(include_str!("../../../../web/TAPE/SKY_RAIDER.BLIP"));
    assert!(p.lines.len() > 100);
    assert_eq!(p.lines[0].0, 1);
    assert_eq!(p.lines.last().unwrap().0, 1970);
    assert!(p.lines.windows(2).all(|pair| pair[0].0 < pair[1].0));
    let source = p
        .lines
        .iter()
        .map(|(_, line)| line.as_str())
        .collect::<Vec<_>>()
        .join(" ");
    assert!(source.contains("ENEMY_WEAVER.PNG") && source.contains("ENEMY_ACE.PNG"));
    assert!(
        source.contains("FORMATION_TYPE=INT(RND(3))+1")
            && source.contains("FORMATION_X+WING*34")
            && source.contains("FORMATION_DIR")
    );
    assert!(
        source.contains("SPAWN>80")
            && source.contains("HIT_WAIT=70")
            && source.contains("SCROLL=SCROLL+1")
            && source.contains("KILLS>=60+15*(LEVEL-1)")
            && source.contains("BOSS_WARNING")
            && source.contains("BOSS_IMAGE$=")
            && source.contains("LEVEL=LEVEL+1")
            && source.contains("WEAPON=WEAPON+1")
            && source.contains("SCORE=SCORE+30*WEAPON")
            && source.contains("MAX_POWER=120")
            && source.contains("SKY_RAIDER/POWERUP3")
            && source.contains("SKY_RAIDER/POWERUP4")
            && source.contains("SKY_RAIDER/MAX_POWER")
            && source.contains("GUNS=8")
            && source.contains("/TAPE/GFX/SKY_RAIDER/POWERUP.PNG")
    );
    assert!(
        source.contains("DRAW_TEXTURE(\"/TAPE/GFX/SKY_RAIDER/PLAYER_PLANE.PNG\",PX,PY,33,29)")
            && source.contains("DRAW_HUD(SCORE,LIVES)")
    );
    assert!(
        source.contains("COLOR(0.10-0.07*DEPTH,0.24-0.13*DEPTH,0.46-0.16*DEPTH)")
            && source.contains("EBVX(Q)=DX/DIST*1.8")
            && source.contains("SKY_RAIDER/ENEMY_GUN")
    );
}
#[test]
fn brawler_source_loads_as_numbered_program() {
    let mut p = Program::new();
    p.load(include_str!("../../../../web/TAPE/BRAWLER.BLIP"));
    assert!(p.lines.len() > 35);
    assert_eq!(p.lines[0].0, 1);
    assert_eq!(p.lines.last().unwrap().0, 2150);
    assert!(p.lines.windows(2).all(|pair| pair[0].0 < pair[1].0));
    assert!(p
        .lines
        .iter()
        .any(|(_, source)| source.contains("DRAW_LINE_EX(160+COS")));
    let source = p
        .lines
        .iter()
        .map(|(_, line)| line.as_str())
        .collect::<Vec<_>>()
        .join(" ");
    assert!(
        source.contains("KEY_PRESSED(\"F\")")
            && source.contains("KEY_PRESSED(\"G\")")
            && source.contains("KEY_PRESSED(\"Z\")")
            && source.contains("PUNCH_BUFFER>0")
            && source.contains("KICK_BUFFER>0")
    );
    assert!(
        source.contains("ROUND_TIME=45")
            && source.contains("ROUND_TIME<=0")
            && source.contains("PLAYER_HP>CPU_HP")
            && source
                .contains("DRAW_TEXT(NAME$(PICK),25,27,1,\"BLIP_CYAN\",\"LEFT\",\"BLIP_BLACK\")")
    );
}
#[test]
fn bubbler_source_loads_as_numbered_program() {
    let mut p = Program::new();
    p.load(include_str!("../../../../web/TAPE/BUBBLER.BLIP"));
    assert!(p.lines.len() > 75);
    assert_eq!(p.lines[0].0, 1);
    assert_eq!(p.lines.last().unwrap().0, 2083);
    assert!(p.lines.windows(2).all(|pair| pair[0].0 < pair[1].0));
    let source = p
        .lines
        .iter()
        .map(|(_, line)| line.as_str())
        .collect::<Vec<_>>()
        .join(" ");
    assert!(
        source.contains("TITLE_LETTER_")
            && source.contains("EVY(E)=EVY(E)+0.09")
            && source.contains("EVY(E)=0-2.6")
    );
    assert!(
        source.contains("PYLINE(P)=8+PROW*8")
            && source.contains("PXLINE(P)=PCOL*12")
            && source.contains("ENEMY_COUNT=15")
    );
    assert!(
        source.contains("EY_OLD(E)+16<=PYLINE(P)")
            && source.contains("ETYPE(E)=EREST-ECOL*4")
            && source.contains("EHOME(E)+SIN(T*3+E)*4")
    );
    assert!(
        source.contains("ROUND>8 THEN GOTO 690")
            && source.contains("ROUND_TIME>HURRY_AT")
            && source.contains("BT(B)=600-30*ROUND")
            && source.contains("EANGRY(TRAPPED)=1")
            && source.contains("BUNNY_ANGRY.PNG")
            && source.contains("ROBOT.PNG")
            && source.contains(r#"TILE_LOW_"+STR$(ROUND)"#)
            && source.contains(r#"BORDER_LOW_"+STR$(ROUND)"#)
            && source.contains("DRAW_NUMBER(SCORE,8,8")
            && !source.contains("DRAW_HUD(SCORE,LIVES)")
    );
    assert!(
        source.contains("SHOT_KIND(BSLOT)=2")
            && source.contains("ROUND>=4 OR EANGRY(E)=1")
            && source.contains("ETYPE(E)=3 AND FIRE_CD(E)<=0")
            && source.contains("BUBBLER/SPARKLE")
    );
}
#[test]
fn primitive_calls_use_the_crate_method_name() {
    let expected = Some(("fill_rect".into(), "1,2,3,4,color".into()));
    assert_eq!(method_call("fill_rect(1,2,3,4,color)"), expected);
    assert_eq!(method_call("FILL_RECT(1,2,3,4,color)"), expected);
    assert_eq!(method_call("BLIP.fill_rect(1,2,3,4,color)"), None);
    assert_eq!(
        method_call("CANVAS(480,540)"),
        Some(("canvas".into(), "480,540".into()))
    );
}
#[test]
fn hex_colors_support_rgb_and_rgba() {
    assert_eq!(
        value_color(&Value::Str("#336699".into())),
        Color::new(
            0x33 as f32 / 255.0,
            0x66 as f32 / 255.0,
            0x99 as f32 / 255.0,
            1.0
        )
    );
    assert_eq!(
        value_color(&Value::Str("#33669980".into())).a,
        128.0 / 255.0
    );
}

#[test]
fn procedures_are_found_with_their_parameters() {
    let mut p = Program::new();
    p.load("10 CALL BOX(1,2,\"RED\")\n20 END\n100 SUB BOX(X,Y,C$) STATIC\n110 LOCAL I\n120 END SUB\n130 SUB BEEP:END SUB");
    assert_eq!(p.error, None);
    assert!(p.is_procedure("BOX") && p.is_procedure("BEEP"));
    let boxed = &p.procedures["BOX"];
    assert_eq!(boxed.params, ["X", "Y", "C$"]);
    assert_eq!((boxed.body, boxed.end), ((2, 1), (4, 0)));
    assert_eq!(p.procedures["BEEP"].end, (5, 1));
}

#[test]
fn procedure_structure_errors_stop_the_load() {
    for (source, error) in [
        ("10 SUB A\n20 PRINT 1", "? SUB A needs END SUB at line 10"),
        ("10 END SUB", "? END SUB without SUB at line 10"),
        ("10 SUB A\n20 SUB B\n30 END SUB", "? SUB inside SUB A at line 20"),
        ("10 SUB A:END SUB\n20 SUB A:END SUB", "? SUB A is defined twice"),
        ("10 SUB PRINT:END SUB", "? SUB needs a name, not PRINT at line 10"),
        ("10 SUB A(1):END SUB", "? SUB A has a bad parameter 1 at line 10"),
    ] {
        let mut p = Program::new();
        p.load(source);
        assert_eq!(p.error.as_deref(), Some(error), "{source}");
        assert!(!p.active);
    }
}

#[test]
fn block_if_structure_errors_stop_the_load() {
    for (source, error) in [
        ("10 IF X THEN\n20 PRINT 1", "? IF needs END IF at line 10"),
        ("10 END IF", "? END IF without IF at line 10"),
        ("10 ELSE", "? ELSE without IF at line 10"),
        ("10 IF X THEN\n20 ELSE\n30 ELSEIF Y THEN\n40 END IF", "? ELSEIF after ELSE at line 30"),
        ("10 IF X THEN\n20 ELSEIF Y\n30 END IF", "? ELSEIF needs THEN at the end of line 20"),
        ("10 WHILE X\n20 PRINT 1", "? WHILE needs WEND at line 10"),
        ("10 WEND", "? WEND without WHILE at line 10"),
    ] {
        let mut p = Program::new();
        p.load(source);
        assert_eq!(p.error.as_deref(), Some(error), "{source}");
    }
    let mut p = Program::new();
    p.load("10 IF A THEN\n20 IF B THEN\n30 END IF\n40 ELSE\n50 END IF\n60 IF C THEN PRINT \"THEN\"");
    assert_eq!(p.error, None);
    assert_eq!(p.blocks.len(), 2);
}

#[test]
fn else_belongs_to_the_nearest_if() {
    use super::syntax::matching_else;
    assert_eq!(matching_else(" A ELSE B"), Some(3));
    assert_eq!(matching_else(" IF B THEN X ELSE Y"), None);
    let nested = " IF B THEN X ELSE Y ELSE Z";
    assert_eq!(matching_else(nested), nested.rfind("ELSE"));
    assert_eq!(matching_else(" PRINT \"ELSE\""), None);
    assert_eq!(matching_else(" ELSEWHERE=1"), None);
}

/// The tapes rewritten in the house layout, one by one.
const HOUSE_TAPES: [(&str, &str); 4] = [
    ("METEORS", include_str!("../../../../web/TAPE/METEORS.BLIP")),
    ("ADDER", include_str!("../../../../web/TAPE/ADDER.BLIP")),
    ("RALLY", include_str!("../../../../web/TAPE/RALLY.BLIP")),
    ("SERPENT", include_str!("../../../../web/TAPE/SERPENT.BLIP")),
];

/// Every tape reads the same way: what the game is, SETUP, one MAIN LOOP that
/// updates and draws the current screen, and a named SUB for each step.
#[test]
fn tapes_follow_the_house_layout() {
    for (name, source) in HOUSE_TAPES {
        let mut p = Program::new();
        p.load(source);
        assert_eq!(p.error, None, "{name}");
        assert!(p.lines.windows(2).all(|pair| pair[0].0 < pair[1].0), "{name}: line order");
        assert_eq!(p.lines[1], (2, format!("REM GAME: {name}")), "{name}");
        for heading in ["10 REM --- SETUP ---", "100 REM --- MAIN LOOP"] {
            assert!(source.contains(heading), "{name}: {heading}");
        }
        for sub in ["START_GAME", "UPDATE_TITLE", "DRAW_TITLE", "UPDATE_PLAY", "DRAW_PLAY", "UPDATE_OVER", "DRAW_OVER"] {
            assert!(p.is_procedure(sub), "{name}: SUB {sub}");
        }
        assert!(!source.contains("GOSUB") && !source.contains("CANVAS("), "{name}: SUBs and the 640x400 screen");
        // Bodies indent two spaces a level; a line holds one statement, or a short pair.
        let mut depth = 0usize;
        for line in source.lines() {
            let body = line.split_once(' ').map_or("", |(_, body)| body);
            let code = body.trim_start();
            let (word, _) = super::procedure::leading_name(code);
            let closes = matches!(word.as_str(), "NEXT" | "ELSE" | "ELSEIF" | "WEND") || code.starts_with("END SUB") || code.starts_with("END IF");
            if closes { depth = depth.checked_sub(1).unwrap_or_else(|| panic!("{name}: unopened {line}")); }
            assert_eq!(body.len() - code.len(), 2 * depth, "{name}: indent {line}");
            let one_line_for = word == "FOR" && code.contains(":NEXT");
            let opens = matches!(word.as_str(), "SUB" | "ELSE" | "ELSEIF" | "WHILE") || word == "FOR" && !one_line_for
                || super::block::block_condition(code).is_some();
            if opens { depth += 1; }
            let statements = split_statements(code);
            assert!(statements.len() <= 1 || statements.len() <= 3 && statements.iter().all(|s| s.len() <= 24),
                "{name}: one statement a line: {line}");
        }
        assert_eq!(depth, 0, "{name}: unclosed block");
    }
}

#[test]
fn data_is_collected_in_order_and_checked_at_load() {
    let mut p = Program::new();
    p.load("10 DATA 1, -2.5\n20 PRINT 1:DATA \"A,B\"");
    assert_eq!(p.error, None);
    assert_eq!(p.data, [(10, Value::Num(1.0)), (10, Value::Num(-2.5)), (20, Value::Str("A,B".into()))]);
    p.load("10 DATA X+1");
    assert_eq!(p.error.as_deref(), Some("? DATA needs numbers or \"text\", not X+1 at line 10"));
}

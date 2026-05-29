/* ════════════════════════════════════════════════════════════════════
   docs.js — flip-and-grow "SEE INSIDE" source tour overlay
   ════════════════════════════════════════════════════════════════════ */
(function () {
  'use strict';

  // ── Accent colors (must match index.html palette) ────────────────────
  var ACCENT = {
    rally:    '#dc3232',
    bouncer:  '#00c8c8',
    galactic: '#c832c8',
    serpent:  '#32c832',
    canaris:  '#3296c8',
    library:  '#d4d4d4',
  };

  // ── Syntax highlighting (very minimal — for Rust excerpts) ───────────
  function rs(src) {
    // Order matters: comments → strings → keywords → idents
    var KW = /\b(fn|let|mut|pub|use|mod|match|if|else|loop|for|while|return|self|Self|struct|enum|impl|trait|as|async|await|move|in|where|const|static|true|false)\b/g;
    var TY = /\b([A-Z][A-Za-z0-9_]*)\b/g;
    // Escape HTML first.
    var s = src.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;');
    // Comments
    s = s.replace(/(\/\/[^\n]*)/g, '<span class="cmt">$1</span>');
    // Strings
    s = s.replace(/("([^"\\]|\\.)*")/g, '<span class="str">$1</span>');
    // Numbers
    s = s.replace(/\b(\d+\.?\d*)\b/g, '<span class="num">$1</span>');
    // Types (CamelCase)
    s = s.replace(TY, '<span class="ty">$1</span>');
    // Keywords (after types so 'Self' wins as kw via span overwrite — but keep order)
    s = s.replace(KW, '<span class="kw">$1</span>');
    // Function calls — name followed by `(`
    s = s.replace(/\b([a-z_][a-z0-9_]*)(?=\()/g, '<span class="fn">$1</span>');
    return s;
  }

  function code(path, src) {
    return '<div class="path">' + path + '</div><pre>' + rs(src) + '</pre>';
  }

  function diagram(text) {
    return { kind: 'diagram', text: text };
  }

  function shot(src) {
    return { kind: 'shot', src: src };
  }

  function xref(target, label) {
    return '<a class="crossref" data-xref="' + target + '">' + (label || ('→ ' + target.toUpperCase() + ' LIBRARY')) + '</a>';
  }

  // ── Card content data ────────────────────────────────────────────────
  var CARDS = {

    rally: {
      title: 'RALLY — SOURCE TOUR',
      pages: [
        {
          shot: shot('rally/screenshot.png'),
          html:
            '<h3>TWO-PADDLE PONG</h3>' +
            '<p>Rally is the simplest game in the cabinet: two paddles, one ball, first to seven. ' +
            'It exists to validate the shared <code>blip</code> input + score pipeline against ' +
            'the smallest possible state machine.</p>' +
            '<p>The whole game lives in a single <code>main.rs</code> with a ~10-field <code>Game</code> ' +
            'struct. Player 1 uses <code>W/S</code>, Player 2 uses <code>↑/↓</code>.</p>' +
            xref('library', '→ How input/score work in the library')
        },
        {
          shot: diagram('TITLE\n  ↓ btn1\nPLAY ⇄ SERVE\n  ↓ score == 7\nGAME OVER\n  ↓ btn1\nTITLE'),
          html:
            '<h3>STATE MACHINE</h3>' +
            '<p>Four states. <code>Serve</code> exists so the ball pauses for a beat after a point, ' +
            'giving players time to reset their hands.</p>' +
            code('crates/rally/src/main.rs',
              'enum State { Title, Serve, Play, GameOver }\n\n' +
              'match g.state {\n' +
              '    State::Title    => update_title(&mut g),\n' +
              '    State::Serve    => update_serve(&mut g, dt),\n' +
              '    State::Play     => update_play(&mut g, dt, &sfx),\n' +
              '    State::GameOver => update_gameover(&mut g),\n' +
              '}')
        },
        {
          shot: diagram('VY ← -VY  (paddle hit)\n  +\nVY += (hit_y - paddle_y) * 4'),
          html:
            '<h3>BALL PHYSICS</h3>' +
            '<p>The "english" is just a position offset: where the ball hits the paddle skews ' +
            'its vertical velocity. No angles, no trig — pure linear nudge.</p>' +
            code('crates/rally/src/main.rs',
              'if ball_intersects(&g.ball, &g.p1) {\n' +
              '    g.ball.vx = g.ball.vx.abs();\n' +
              '    g.ball.vy += (g.ball.y - g.p1.y - PADDLE_H * 0.5) * 4.0;\n' +
              '    play_sfx(&sfx.blip);\n' +
              '}') +
            xref('library')
        },
      ]
    },

    bouncer: {
      title: 'BOUNCER — SOURCE TOUR',
      pages: [
        {
          shot: shot('bouncer/screenshot.png'),
          html:
            '<h3>BREAKOUT WITH A NEON BEAT</h3>' +
            '<p>Reflect the ball with the paddle below; clear bricks above. Three music ' +
            'variations cross-fade as you progress — driven by the shared ' +
            '<code>MusicTracks</code> helper in the blip library.</p>' +
            '<p>Controls: <code>←/→</code> move, <code>Space</code> launch.</p>'
        },
        {
          shot: diagram('TRACK 0 → TRACK 1 → TRACK 2\n   (level 1)   (level 2)   (level 3+)'),
          html:
            '<h3>MUSIC LAYERING</h3>' +
            '<p>Bouncer asks the library to remember which track is currently playing, ' +
            'then switches at level boundaries. No fades, no manual indexing — ' +
            '<code>MusicTracks::switch_to</code> handles the swap.</p>' +
            code('crates/bouncer/src/main.rs',
              'let mut music = MusicTracks::start(&[\n' +
              '    sfx.theme_a, sfx.theme_b, sfx.theme_c,\n' +
              ']);\n\n' +
              '// on level up:\n' +
              'music.switch_to((g.level - 1).min(2));') +
            xref('library', '→ MusicTracks in the library')
        },
        {
          shot: shot('bouncer/screenshot.png'),
          html:
            '<h3>BRICK GRID</h3>' +
            '<p>A flat <code>Vec&lt;Brick&gt;</code> with row/col packed into the entity. ' +
            'Collision is AABB-vs-AABB; the side of impact decides whether to flip ' +
            '<code>vx</code> or <code>vy</code>.</p>' +
            code('crates/bouncer/src/main.rs',
              'for brick in g.bricks.iter_mut().filter(|b| b.alive) {\n' +
              '    if let Some(side) = aabb_hit(&g.ball, &brick.rect) {\n' +
              '        brick.alive = false;\n' +
              '        if side.horizontal { g.ball.vx = -g.ball.vx; }\n' +
              '        else               { g.ball.vy = -g.ball.vy; }\n' +
              '        g.score += brick.value;\n' +
              '    }\n' +
              '}')
        },
      ]
    },

    galactic: {
      title: 'GALACTIC DEFENDER — SOURCE TOUR',
      pages: [
        {
          shot: shot('galactic_defender/screenshot.png'),
          html:
            '<h3>FIXED SHOOTER</h3>' +
            '<p>A grid of aliens marches sideways, drops a row, and repeats — accelerating ' +
            'as their numbers thin. You move on a single axis at the bottom and shoot up. ' +
            'Mystery saucers occasionally streak across the top.</p>' +
            '<p>Single dominant <code>Play</code> scene with a <code>~177-line update_play</code>; ' +
            'the file deliberately stays one piece so the marching/firing/collision rhythm is ' +
            'readable top-to-bottom.</p>'
        },
        {
          shot: diagram('ALIENS:\n  step_t += dt\n  if step_t > step_period:\n    step_period *= 0.97\n    march →\n    if edge: drop_row + flip_dir'),
          html:
            '<h3>MARCH TIMING</h3>' +
            '<p>The signature "tick … tick … tick-tick-tick" acceleration is just a ' +
            'period that shrinks every time the swarm steps. Fewer aliens → no special ' +
            'logic needed; the period is decoupled from the count.</p>' +
            code('crates/galactic_defender/src/main.rs',
              'g.step_t += dt;\n' +
              'if g.step_t >= g.step_period {\n' +
              '    g.step_t = 0.0;\n' +
              '    g.step_period = (g.step_period * 0.97).max(0.08);\n' +
              '    march_swarm(&mut g);\n' +
              '}')
        },
        {
          shot: shot('galactic_defender/screenshot.png'),
          html:
            '<h3>BULLET POOL</h3>' +
            '<p>Player and alien bullets share a fixed-size pool from the library, so ' +
            'allocation is zero per frame. Inactive slots are reused on the next fire.</p>' +
            xref('library', '→ Pool in the library')
        },
      ]
    },

    serpent: {
      title: 'SERPENT — SOURCE TOUR',
      pages: [
        {
          shot: shot('serpent/screenshot.png'),
          html:
            '<h3>GRID MOVER</h3>' +
            '<p>Eat food to grow longer. Avoid the walls and yourself. The grid is ' +
            'rendered as discrete cells; ticks fire every <code>TICK_PERIOD</code> seconds ' +
            '— input is buffered between ticks so directional turns feel responsive.</p>'
        },
        {
          shot: diagram('HEAD ← buffered input\nTAIL ← pop unless ate food'),
          html:
            '<h3>BODY AS DEQUE</h3>' +
            '<p>The snake is a <code>VecDeque&lt;(i32,i32)&gt;</code>. Each tick: push a new head ' +
            'in the current direction; pop the tail unless the head landed on food.</p>' +
            code('crates/serpent/src/main.rs',
              'let new_head = step(g.body.front().copied().unwrap(), g.dir);\n' +
              'g.body.push_front(new_head);\n' +
              'if new_head == g.food { spawn_food(&mut g); g.score += 10; }\n' +
              'else                  { g.body.pop_back(); }')
        },
        {
          shot: shot('serpent/screenshot.png'),
          html:
            '<h3>INPUT BUFFER</h3>' +
            '<p>Pressing Up immediately after Left would be a 180° reversal on the same tick — ' +
            'illegal. The buffer stores the *next* direction; the tick consumes it only if it ' +
            'isn\'t a reversal.</p>' +
            code('crates/serpent/src/main.rs',
              'if key_pressed(BLIP_KEY_UP)    { g.next_dir = Dir::Up; }\n' +
              'if key_pressed(BLIP_KEY_DOWN)  { g.next_dir = Dir::Down; }\n' +
              '// on tick:\n' +
              'if !g.next_dir.is_opposite(g.dir) { g.dir = g.next_dir; }')
        },
      ]
    },

    canaris: {
      title: 'CANARIS — SOURCE TOUR',
      pages: [
        {
          shot: shot('canaris/screenshot.png?v=2'),
          html:
            '<h3>OPEN-WORLD SAILOR</h3>' +
            '<p>Inspired by <em>Kaptajn Kaper i Kattegat</em> (1985). Sail the Kattegat, ' +
            'spot enemy ships, choose your engagement: cannon duel, board for plunder, or ' +
            'run for port to repair and re-provision.</p>' +
            '<p>The game is built as <strong>seven scenes</strong>, one per state, each in its ' +
            'own module. The <code>main.rs</code> is a tiny dispatcher.</p>'
        },
        {
          shot: diagram('TITLE → SEA ⇄ COMBAT\n         ↕      ↓\n        PORT ⇄ MAP\n         ↑      ↓\n        DEAD ← BOARDING\n         ↓\n      GAMEOVER'),
          html:
            '<h3>SCENE DISPATCH</h3>' +
            '<p>Every scene module exports <code>update_X</code> and <code>draw_X</code>. ' +
            '<code>main.rs</code> picks one based on the current <code>State</code>.</p>' +
            code('crates/canaris/src/main.rs',
              'match g.state {\n' +
              '    State::Title    => update_title(&mut g, dt),\n' +
              '    State::Sea      => update_sea(&mut g, dt, &sfx),\n' +
              '    State::Combat   => update_combat(&mut g, dt, &sfx),\n' +
              '    State::Boarding => update_boarding(&mut g, dt, &sfx),\n' +
              '    State::Port     => update_port(&mut g, dt, &sfx),\n' +
              '    State::Map      => update_map(&mut g, dt, &sfx),\n' +
              '    State::Dead     => update_dead(&mut g, dt, &sfx),\n' +
              '    State::GameOver => update_gameover(&mut g),\n' +
              '}')
        },
        {
          shot: shot('canaris/screenshot.png?v=2'),
          html:
            '<h3>SEA — FREE ROAM</h3>' +
            '<p>Top-down sailing with a two-frame wave animation underneath. Enemy ships ' +
            'wander the world; getting close enough triggers a transition to <code>Combat</code>.</p>' +
            code('crates/canaris/src/sea.rs',
              'pub fn update_sea(g: &mut Game, dt: f32, sfx: &Sounds) {\n' +
              '    // wave anim, ship movement, hunger tick...\n' +
              '    for (i, e) in g.enemies.iter().enumerate() {\n' +
              '        if !e.active { continue; }\n' +
              '        if dist(g.player.pos, e.pos) < COMBAT_RANGE {\n' +
              '            g.enter_combat(i);\n' +
              '            return;\n' +
              '        }\n' +
              '    }\n' +
              '}')
        },
        {
          shot: shot('canaris/screenshot.png?v=2'),
          html:
            '<h3>COMBAT — CANNON DUEL</h3>' +
            '<p>Side-by-side broadsides. Cannonballs arc with a fixed vertical impulse ' +
            'and gravity; hull-hit splashes feed back via shared sfx slots.</p>' +
            code('crates/canaris/src/combat.rs',
              'g.cannonballs[i] = Cannonball {\n' +
              '    active: true,\n' +
              '    x:  COMBAT_PLAYER_X + PLAYER_W,\n' +
              '    y:  g.player.y + PLAYER_H * 0.55,\n' +
              '    vx: CANNON_SPEED,\n' +
              '    vy: -CANNON_ARC_VY,\n' +
              '    player: true,\n' +
              '};')
        },
        {
          shot: diagram('YOUR CREW  │  ENEMY CREW\n  ▓ ▓ ▓    │    ▓ ▓ ▓\n   ↑ attack rightmost player\n   ↑ player attacks leftmost enemy'),
          html:
            '<h3>BOARDING — CREW SLOTS</h3>' +
            '<p>The duel is modeled as three slots per side. Pressing <code>[1]</code> attacks ' +
            'the frontmost enemy slot; the enemy auto-ticks against your rightmost slot. ' +
            'When a slot drops to zero, the attacker captures it (with 2 HP).</p>' +
            code('crates/canaris/src/boarding.rs',
              'if btn1_pressed() {\n' +
              '    for i in 0..BOARDING_SLOTS {\n' +
              '        if g.slots[i].owner == SlotOwner::Enemy {\n' +
              '            g.slots[i].hp -= 1;\n' +
              '            if g.slots[i].hp <= 0 {\n' +
              '                g.slots[i].owner = SlotOwner::Empty;\n' +
              '            }\n' +
              '            break;\n' +
              '        }\n' +
              '    }\n' +
              '}')
        },
        {
          shot: shot('canaris/screenshot.png?v=2'),
          html:
            '<h3>PORT & MAP</h3>' +
            '<p>Port is a six-item shop menu (Sail, Map, Repair, Crew, Cannons, Food). ' +
            'The Map is a crosshair zone selector over a static Kattegat backdrop — picking ' +
            'a zone seeds <code>spawn_enemies_n</code> with that zone\'s ship count and ' +
            'difficulty level.</p>' +
            code('crates/canaris/src/port.rs',
              'if btn1_pressed() {\n' +
              '    let z = &ZONES[g.map_cursor];\n' +
              '    g.level   = z.level_eq;\n' +
              '    g.spawn_enemies_n(z.ships);\n' +
              '    g.state = State::Sea;\n' +
              '}') +
            xref('library', '→ How saves & i18n live in the library')
        },
      ]
    },

    library: {
      title: 'BLIP — SHARED LIBRARY',
      pages: [
        {
          shot: diagram('crates/blip/\n├ lib.rs\n├ session.rs\n├ audio.rs\n├ web.rs\n├ texture.rs\n└ input.rs'),
          html:
            '<h3>WHAT THE LIBRARY DOES</h3>' +
            '<p>Every game in the cabinet links one crate: <code>blip</code>. It owns the ' +
            'window setup, the input map, score persistence, audio playback, and the ' +
            'asset-loading macros — so each game\'s <code>main.rs</code> stays about the ' +
            'mechanic, not the plumbing.</p>' +
            '<p>The five game cards each cross-link here when they touch a shared piece.</p>'
        },
        {
          shot: diagram('Session\n ├ game_id\n ├ hi_score\n └ notify_game_over()'),
          html:
            '<h3>SESSION + HI-SCORE</h3>' +
            '<p>Each game constructs a <code>Session</code> with its own game id. The session ' +
            'remembers the current run and forwards score updates to the JS bridge ' +
            '(Supabase-backed) on game over.</p>' +
            code('crates/blip/src/session.rs',
              'pub struct Session {\n' +
              '    pub game_id: &\'static str,\n' +
              '    pub hi_score: i32,\n' +
              '}\n\n' +
              'impl Session {\n' +
              '    pub fn notify_game_over(&self, score: i32) {\n' +
              '        web::save_hi_score(self.game_id, score);\n' +
              '    }\n' +
              '}')
        },
        {
          shot: diagram('MusicTracks::start(&[a, b, c])\n          ↓\n  switch_to(idx) — stops old, plays new'),
          html:
            '<h3>AUDIO — MusicTracks</h3>' +
            '<p>Bouncer needed three theme variations to swap at level boundaries; the ' +
            'pattern got pulled into the library so any future game can do the same with ' +
            'two lines.</p>' +
            code('crates/blip/src/audio.rs',
              'pub struct MusicTracks<\'a> {\n' +
              '    tracks: &\'a [BlipSound],\n' +
              '    current: usize,\n' +
              '}\n\n' +
              'impl<\'a> MusicTracks<\'a> {\n' +
              '    pub fn start(tracks: &\'a [BlipSound]) -> Self { /* ... */ }\n' +
              '    pub fn switch_to(&mut self, idx: usize) { /* stop+play */ }\n' +
              '    pub fn current(&self) -> usize { self.current }\n' +
              '}')
        },
        {
          shot: diagram('blip_image!("foo.png")\n  → include_bytes!(concat!(OUT_DIR, "/assets/images/foo.png"))'),
          html:
            '<h3>ASSET MACROS</h3>' +
            '<p>Each game has a <code>build.rs</code> that copies its PNGs and WAVs into ' +
            '<code>$OUT_DIR/assets/</code>. The library exposes two macros that turn a bare ' +
            'filename into an <code>&amp;[u8]</code> at compile time — no path strings ' +
            'sprinkled through game code.</p>' +
            code('crates/blip/src/lib.rs',
              '#[macro_export]\n' +
              'macro_rules! blip_image {\n' +
              '    ($name:literal) => {\n' +
              '        include_bytes!(concat!(\n' +
              '            env!("OUT_DIR"), "/assets/images/", $name\n' +
              '        ))\n' +
              '    };\n' +
              '}')
        },
        {
          shot: diagram('Pool<T, N>\n ├ slice of slots\n ├ acquire() → &mut T\n └ active() iterator'),
          html:
            '<h3>POOL & TIMER</h3>' +
            '<p>Fixed-capacity arrays with an <code>active</code> flag — no allocation in the ' +
            'frame loop. Galactic Defender uses one for bullets; Canaris uses several (' +
            '<code>cannonballs</code>, <code>splashes</code>, <code>explosions</code>).</p>' +
            '<p><code>blip::load_png</code> wraps macroquad\'s loader with the pixel-art ' +
            'filter mode preset — every game would otherwise repeat the same three lines.</p>'
        },
      ]
    },
  };

  // ── Overlay state ────────────────────────────────────────────────────
  var openCardId = null;
  var pageIdx = 0;
  var backdrop = null;
  var flipper = null;
  var sourceCardEl = null;

  function buildOverlay() {
    backdrop = document.createElement('div');
    backdrop.className = 'docs-backdrop';
    backdrop.addEventListener('click', close);
    document.body.appendChild(backdrop);
  }

  function pageView(cardId, page) {
    var shotHtml;
    if (page.shot.kind === 'diagram') {
      shotHtml = '<div class="docs-shot diagram">' + page.shot.text.replace(/\n/g, '<br>') + '</div>';
    } else {
      shotHtml = '<div class="docs-shot"><img src="' + page.shot.src + '" alt=""></div>';
    }
    return shotHtml + '<div class="docs-prose">' + page.html + '</div>';
  }

  function renderBack(cardId) {
    var data = CARDS[cardId];
    var pages = data.pages;
    var page = pages[pageIdx];
    var dots = pages.map(function (_, i) {
      return '<span class="' + (i === pageIdx ? 'on' : '') + '" data-go="' + i + '"></span>';
    }).join('');
    return (
      '<div class="docs-head">' +
        '<div class="title">' + data.title + '</div>' +
        '<div class="page-indicator">' + (pageIdx + 1) + ' / ' + pages.length + '</div>' +
        '<button class="docs-close" aria-label="Close">✕</button>' +
      '</div>' +
      '<div class="docs-body">' + pageView(cardId, page) + '</div>' +
      '<div class="docs-nav">' +
        '<button class="docs-prev"' + (pageIdx === 0 ? ' disabled' : '') + '>◄ PREV</button>' +
        '<div class="docs-dots">' + dots + '</div>' +
        '<button class="docs-next"' + (pageIdx === pages.length - 1 ? ' disabled' : '') + '>NEXT ►</button>' +
      '</div>'
    );
  }

  function wireBack() {
    var back = flipper.querySelector('.docs-face.back');
    back.querySelector('.docs-close').addEventListener('click', close);
    var prev = back.querySelector('.docs-prev');
    var next = back.querySelector('.docs-next');
    if (prev) prev.addEventListener('click', function () { goPage(pageIdx - 1); });
    if (next) next.addEventListener('click', function () { goPage(pageIdx + 1); });
    back.querySelectorAll('.docs-dots span').forEach(function (d) {
      d.addEventListener('click', function () { goPage(parseInt(d.getAttribute('data-go'), 10)); });
    });
    back.querySelectorAll('.crossref').forEach(function (a) {
      a.addEventListener('click', function (e) {
        e.preventDefault();
        var target = a.getAttribute('data-xref');
        crossNavigate(target);
      });
    });
  }

  function goPage(i) {
    var pages = CARDS[openCardId].pages;
    if (i < 0 || i >= pages.length) return;
    pageIdx = i;
    var back = flipper.querySelector('.docs-face.back');
    back.innerHTML = renderBack(openCardId);
    wireBack();
  }

  function crossNavigate(target) {
    if (!CARDS[target]) return;
    // Re-flip: close current, then open target.
    var nextId = target;
    close(function () {
      // Find the target's card in the grid (or use the library card).
      var el = document.querySelector('[data-card-id="' + nextId + '"]');
      if (el) open(nextId, el);
    });
  }

  // ── Open / close ─────────────────────────────────────────────────────
  function open(cardId, cardEl) {
    if (openCardId) return;
    if (!CARDS[cardId]) return;
    openCardId = cardId;
    pageIdx = 0;
    sourceCardEl = cardEl;

    var accent = ACCENT[cardId] || '#888';
    var rect = cardEl.getBoundingClientRect();

    flipper = document.createElement('div');
    flipper.className = 'docs-flipper';
    flipper.style.setProperty('--c', accent);
    flipper.style.top    = rect.top  + 'px';
    flipper.style.left   = rect.left + 'px';
    flipper.style.width  = rect.width  + 'px';
    flipper.style.height = rect.height + 'px';

    // Front: clone of the card. Back: docs UI.
    var clone = cardEl.cloneNode(true);
    // Strip interaction surface on the clone.
    clone.removeAttribute('href');
    clone.style.position = 'absolute';
    clone.style.inset = '0';
    clone.style.margin = '0';
    clone.style.width = '100%';
    clone.style.height = '100%';

    flipper.innerHTML =
      '<div class="docs-card">' +
        '<div class="docs-face front"></div>' +
        '<div class="docs-face back">' + renderBack(cardId) + '</div>' +
      '</div>';
    flipper.querySelector('.docs-face.front').appendChild(clone);

    document.body.appendChild(flipper);
    // Force layout, then trigger the grow + flip on next frame.
    // eslint-disable-next-line no-unused-expressions
    flipper.offsetHeight;
    requestAnimationFrame(function () {
      backdrop.classList.add('open');
      flipper.classList.add('open');
      var vw = window.innerWidth;
      var vh = window.innerHeight;
      var w = Math.min(vw * 0.86, 1100);
      var h = Math.min(vh * 0.86, 760);
      flipper.style.top    = ((vh - h) / 2) + 'px';
      flipper.style.left   = ((vw - w) / 2) + 'px';
      flipper.style.width  = w + 'px';
      flipper.style.height = h + 'px';
    });

    wireBack();
  }

  function close(afterFn) {
    if (!openCardId || !flipper) {
      if (typeof afterFn === 'function') afterFn();
      return;
    }
    var rect = sourceCardEl ? sourceCardEl.getBoundingClientRect() : null;
    backdrop.classList.remove('open');
    flipper.classList.remove('open');
    if (rect) {
      flipper.style.top    = rect.top  + 'px';
      flipper.style.left   = rect.left + 'px';
      flipper.style.width  = rect.width  + 'px';
      flipper.style.height = rect.height + 'px';
    }
    var cleanup = function () {
      if (flipper && flipper.parentNode) flipper.parentNode.removeChild(flipper);
      flipper = null;
      sourceCardEl = null;
      openCardId = null;
      pageIdx = 0;
      if (typeof afterFn === 'function') afterFn();
    };
    setTimeout(cleanup, 560);
  }

  // ── Wire badges + keyboard ───────────────────────────────────────────
  function init() {
    buildOverlay();
    document.querySelectorAll('.badge-see').forEach(function (badge) {
      badge.addEventListener('click', function (e) {
        // Cards are anchors — without BOTH calls, navigation fires under the flip.
        e.stopPropagation();
        e.preventDefault();
        var cardId = badge.getAttribute('data-see');
        var cardEl = badge.closest('[data-card-id]');
        if (cardEl) open(cardId, cardEl);
      });
    });
    document.addEventListener('keydown', function (e) {
      if (!openCardId) return;
      if (e.key === 'Escape')     { e.preventDefault(); close(); }
      else if (e.key === 'ArrowRight') { e.preventDefault(); goPage(pageIdx + 1); }
      else if (e.key === 'ArrowLeft')  { e.preventDefault(); goPage(pageIdx - 1); }
    });
  }

  if (document.readyState === 'loading') {
    document.addEventListener('DOMContentLoaded', init);
  } else {
    init();
  }
})();

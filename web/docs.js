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

  var GH_BASE = 'https://github.com/jacobandresen/blip/blob/main/';

  function code(path, src, lines) {
    var anchor = lines ? ('#L' + lines.replace('-', '-L')) : '';
    var href = GH_BASE + path + anchor;
    var label = path + (lines ? (' · L' + lines) : '');
    return (
      '<a class="path" href="' + href + '" target="_blank" rel="noopener">' +
        '<span class="path-file">' + label + '</span>' +
        '<span class="path-gh">VIEW ON GITHUB ↗</span>' +
      '</a>' +
      '<pre>' + rs(src) + '</pre>'
    );
  }

  function diagram(text) {
    return { kind: 'diagram', text: text };
  }

  function shot(src) {
    return { kind: 'shot', src: src };
  }

  function gh(path, label) {
    return '<a class="gh-inline" href="' + GH_BASE + path + '" target="_blank" rel="noopener">' +
           (label || path) + ' ↗</a>';
  }

  function ghList(items) {
    var rows = items.map(function (it) {
      return '<li><a class="gh-file" href="' + GH_BASE + it.path + '" target="_blank" rel="noopener">' +
             '<span class="gh-file-name">' + it.path.split('/').pop() + '</span>' +
             '<span class="gh-file-desc">' + it.desc + '</span>' +
             '</a></li>';
    }).join('');
    return '<ul class="gh-files">' + rows + '</ul>';
  }

  function xref(target, label) {
    return '<a class="crossref" data-xref="' + target + '">' + (label || ('→ ' + target.toUpperCase() + ' LIBRARY')) + '</a>';
  }

  // ── Language detection ───────────────────────────────────────────────
  function currentLang() {
    var l = (document.documentElement.getAttribute('lang') || 'en').toLowerCase();
    if (l.indexOf('da') === 0) return 'da';
    if (l.indexOf('ja') === 0) return 'ja';
    return 'en';
  }

  // ── HOWTO pages (appended to the library card, localized) ────────────
  function howtoPagesEn() {
    return [
      {
        shot: diagram('YOU + AGENT\n   ↓\nONE AFTERNOON\n   ↓\nNEW CARD ON THE GRID'),
        html:
          '<h3>SHIP YOUR OWN GAME</h3>' +
          '<p>This cabinet started with one game. Now there are five. ' +
          '<strong>You can add the sixth.</strong></p>' +
          '<p>You don\'t need to be a Rust wizard. You don\'t need to know macroquad. ' +
          'You need an <strong>idea you can describe in two sentences</strong> and a coding ' +
          'agent / AI buddy (' +
          '<a class="gh-inline" href="https://github.com/features/copilot" target="_blank" rel="noopener">Copilot</a>, ' +
          '<a class="gh-inline" href="https://claude.com/claude-code" target="_blank" rel="noopener">Claude Code</a>, ' +
          '<a class="gh-inline" href="https://cursor.com" target="_blank" rel="noopener">Cursor</a> — whatever) sitting next to you.</p>' +
          '<p>The library does the boring stuff — window, input, audio, hi-scores, the ' +
          'web bridge. You write the <em>fun</em>. The next pages are the setup, the ' +
          'agent prompt, and a list of one-afternoon classics to start from.</p>'
      },
      {
        shot: diagram('AGENT INSTALLS:\n  rustup\n  cargo\n  wasm32 target\n  (you do nothing)'),
        html:
          '<h3>STEP 0 — FIRST TIME ONLY</h3>' +
          '<p>You need some nerd stuff on your machine for this to work — a Rust toolchain ' +
          'and the WebAssembly target. <strong>Don\'t install ' +
          'anything yourself.</strong> Open your coding agent in a fresh terminal sitting in ' +
          'the cloned repo and paste this:</p>' +
          '<pre>' +
          '<span class="cmt">// Setup prompt — paste this into your agent on a fresh machine</span>\n\n' +
          'Set up everything I need to build and run this repo.\n\n' +
          'Check what is missing on this machine, then install:\n' +
          '  - rustup (the Rust toolchain installer)\n' +
          '  - the stable Rust compiler + cargo\n' +
          '  - the wasm32-unknown-unknown target (rustup target add ...)\n' +
          '  - python3 (for the local web server)\n\n' +
          'Then run: cargo build --workspace\n' +
          'Then run: ./build_web.sh\n' +
          'Then start the dev server: python3 -m http.server -d web 8080\n' +
          '\n' +
          'Tell me the URL to open in my browser when you are done.\n' +
          '</pre>' +
          '<p>The agent will <em>actually run</em> the install commands ' +
          '(<code>curl https://sh.rustup.rs -sSf | sh</code>, ' +
          '<code>rustup target add wasm32-unknown-unknown</code>, etc.) and confirm each ' +
          'step compiles. On macOS it may ask you to allow Xcode command-line tools. ' +
          'On Linux it may need <code>build-essential</code>. The agent handles all of that.</p>' +
          '<p>When it finishes, open ' + '<code>http://localhost:8080</code>' +
          ' and you should see this very cabinet. <strong>Now you\'re ready to add your game.</strong></p>'
      },
      {
        shot: diagram('YOU →  "build me a game where ___"\nAGENT → reads serpent, copies it,\n         renames it, writes the loop,\n         runs cargo check, ships'),
        html:
          '<h3>TELL THE AGENT WHAT YOU WANT</h3>' +
          '<p>Paste something like this into your coding agent. Replace the <code>[brackets]</code>:</p>' +
          '<pre>' +
          '<span class="cmt">// Prompt template — works in Copilot, Claude Code, Cursor, Cline...</span>\n\n' +
          'I want to add a new game to the blip arcade.\n\n' +
          '<span class="ty">Concept</span>: [one sentence — e.g. "Asteroids but the\n' +
          '  player ship is a fish and the asteroids are jellyfish."]\n\n' +
          '<span class="ty">Controls</span>: [arrow keys + space, or whatever]\n\n' +
          '<span class="ty">Win/lose</span>: [score to X / lives = 0 / time runs out]\n\n' +
          'Use the serpent crate as a template.\n' +
          '<span class="ty">NO external assets</span> — draw all sprites in code with\n' +
          'fill_rect (golden-age sprites are tiny pixel grids) and\n' +
          'synthesize sound effects into WAV buffers in build.rs.\n' +
          'Wire it into the workspace and the web grid.\n' +
          'Run cargo build --workspace when done.\n' +
          '</pre>' +
          '<p>The agent will copy ' + gh('crates/serpent', 'serpent') + ', rewrite the loop, ' +
          'plug in your assets, and verify it compiles. <strong>Your job is the idea and the ' +
          'art.</strong> The agent does the wiring.</p>' +
          '<p><em>Pro move:</em> ask the agent to read ' +
          gh('crates/canaris/src/main.rs', 'canaris/main.rs') + ' first if your game has ' +
          'more than one screen — that\'s the multi-scene pattern to copy.</p>'
      },
      {
        shot: diagram('DRAW THE PIXELS\n        ↓\nMAKE THE BLEEPS\n        ↓\nLET THE AGENT GLUE IT\n        ↓\nSHIP TODAY'),
        html:
          '<h3>WHAT TO MAKE FIRST</h3>' +
          '<p>Stuck? Pick one. These are all one-afternoon games with shapes the agent ' +
          'already understands:</p>' +
          '<ul class="gh-files">' +
          '  <li><a class="gh-file"><span class="gh-file-name">FROGGER</span><span class="gh-file-desc">cross the road, lane-by-lane timing</span></a></li>' +
          '  <li><a class="gh-file"><span class="gh-file-name">TETRIS-LITE</span><span class="gh-file-desc">grid + falling pieces + line clears</span></a></li>' +
          '  <li><a class="gh-file"><span class="gh-file-name">FLAPPY</span><span class="gh-file-desc">one button, gravity, gap pipes</span></a></li>' +
          '  <li><a class="gh-file"><span class="gh-file-name">ASTEROIDS</span><span class="gh-file-desc">thrust + wrap-around + bullets</span></a></li>' +
          '  <li><a class="gh-file"><span class="gh-file-name">DUCK HUNT</span><span class="gh-file-desc">mouse aim + spawn timers</span></a></li>' +
          '</ul>' +
          '<p><strong>Skip the art apps — generate everything in code.</strong> ' +
          'A golden-age sprite is an 8×8 or 16×16 grid of colored squares; your agent will ' +
          'happily emit it as a <code>const PIXELS: [[u8; 16]; 16]</code> and draw it with ' +
          '<code>fill_rect</code> in a nested loop. Sound effects? A handful of ' +
          'sine/square samples summed into a WAV buffer — the agent writes the math. ' +
          'It\'s faster, it\'s free, and the whole game stays inside one Rust crate.</p>' +
          '<p>Want references? ' +
          '<a class="gh-inline" href="https://en.wikipedia.org/wiki/Pac-Man" target="_blank" rel="noopener">Pac-Man on Wikipedia ↗</a>' +
          ' has the original sprite charts in the gallery. ' +
          '<a class="gh-inline" href="https://www.spriters-resource.com/arcade/" target="_blank" rel="noopener">Spriters Resource — Arcade ↗</a>' +
          ' shows the exact pixel grids you\'re trying to reproduce.</p>' +
          '<p>The repo is right here. ' +
          '<a class="gh-inline" href="https://github.com/jacobandresen/blip" target="_blank" rel="noopener">Fork it ↗</a>' +
          ', name your game, and ' +
          'tell the agent to go. <strong>The next time someone walks past the cabinet, it ' +
          'could be your game they play.</strong></p>'
      },
    ];
  }

  function howtoPagesDa() {
    return [
      {
        shot: diagram('DIG + AGENT\n   ↓\nÉN EFTERMIDDAG\n   ↓\nNYT KORT PÅ GITTERET'),
        html:
          '<h3>BYG DIT EGET SPIL</h3>' +
          '<p>Skabet startede med ét spil. Nu er der fem. ' +
          '<strong>Du kan tilføje det sjette.</strong></p>' +
          '<p>Du behøver ikke være Rust-troldmand. Du behøver ikke kende macroquad. ' +
          'Du skal bare have en <strong>idé du kan beskrive på to sætninger</strong> og en ' +
          'AI-makker (' +
          '<a class="gh-inline" href="https://github.com/features/copilot" target="_blank" rel="noopener">Copilot</a>, ' +
          '<a class="gh-inline" href="https://claude.com/claude-code" target="_blank" rel="noopener">Claude Code</a>, ' +
          '<a class="gh-inline" href="https://cursor.com" target="_blank" rel="noopener">Cursor</a> — hvad du nu bruger) ved siden af dig.</p>' +
          '<p>Biblioteket klarer kedsomheden — vindue, input, lyd, highscores, web-broen. ' +
          'Du skriver det <em>sjove</em>. De næste sider er opsætningen, agent-prompten ' +
          'og en liste af eftermiddags-klassikere at starte fra.</p>'
      },
      {
        shot: diagram('AGENTEN INSTALLERER:\n  rustup\n  cargo\n  wasm32 target\n  (du gør ingenting)'),
        html:
          '<h3>TRIN 0 — KUN FØRSTE GANG</h3>' +
          '<p>Du har brug for noget nørdet kram på maskinen for at det her virker — en ' +
          'Rust-toolchain og WebAssembly-targetet. ' +
          '<strong>Installer ikke noget selv.</strong> Åbn din kode-agent i en frisk terminal ' +
          'inde i det klonede repo og indsæt denne prompt:</p>' +
          '<pre>' +
          '<span class="cmt">// Opsætnings-prompt — indsæt i agenten på en ny maskine</span>\n\n' +
          'Set up everything I need to build and run this repo.\n\n' +
          'Check what is missing on this machine, then install:\n' +
          '  - rustup (the Rust toolchain installer)\n' +
          '  - the stable Rust compiler + cargo\n' +
          '  - the wasm32-unknown-unknown target (rustup target add ...)\n' +
          '  - python3 (for the local web server)\n\n' +
          'Then run: cargo build --workspace\n' +
          'Then run: ./build_web.sh\n' +
          'Then start the dev server: python3 -m http.server -d web 8080\n' +
          '\n' +
          'Tell me the URL to open in my browser when you are done.\n' +
          '</pre>' +
          '<p>Agenten kører <em>rigtigt</em> installations-kommandoerne ' +
          '(<code>curl https://sh.rustup.rs -sSf | sh</code>, ' +
          '<code>rustup target add wasm32-unknown-unknown</code> osv.) og bekræfter at hvert ' +
          'trin kompilerer. På macOS spørger den måske om at tillade Xcode command-line tools. ' +
          'På Linux skal den måske bruge <code>build-essential</code>. Agenten klarer det.</p>' +
          '<p>Når den er færdig, åbn <code>http://localhost:8080</code> og du burde se selve ' +
          'dette skab. <strong>Nu er du klar til at tilføje dit spil.</strong></p>'
      },
      {
        shot: diagram('DU →  "byg et spil hvor ___"\nAGENT → læser serpent, kopierer,\n         omdøber, skriver løkken,\n         kører cargo check, sender'),
        html:
          '<h3>FORTÆL AGENTEN HVAD DU VIL</h3>' +
          '<p>Indsæt noget i stil med dette i din kode-agent. Udskift <code>[parenteserne]</code>:</p>' +
          '<pre>' +
          '<span class="cmt">// Prompt-skabelon — virker i Copilot, Claude Code, Cursor, Cline...</span>\n\n' +
          'I want to add a new game to the blip arcade.\n\n' +
          '<span class="ty">Concept</span>: [én sætning — fx "Asteroids, men spillerens\n' +
          '  skib er en fisk og asteroiderne er gopler."]\n\n' +
          '<span class="ty">Controls</span>: [piletaster + mellemrum, eller hvad du vil]\n\n' +
          '<span class="ty">Win/lose</span>: [score til X / liv = 0 / tid løber ud]\n\n' +
          'Use the serpent crate as a template.\n' +
          '<span class="ty">NO external assets</span> — draw all sprites in code with\n' +
          'fill_rect (golden-age sprites are tiny pixel grids) and\n' +
          'synthesize sound effects into WAV buffers in build.rs.\n' +
          'Wire it into the workspace and the web grid.\n' +
          'Run cargo build --workspace when done.\n' +
          '</pre>' +
          '<p>Agenten kopierer ' + gh('crates/serpent', 'serpent') + ', omskriver løkken, ' +
          'sætter dine assets ind og tjekker at det kompilerer. <strong>Dit job er idéen og ' +
          'grafikken.</strong> Agenten klarer ledningsføringen.</p>' +
          '<p><em>Pro-trick:</em> bed agenten om at læse ' +
          gh('crates/canaris/src/main.rs', 'canaris/main.rs') + ' først, hvis dit spil har ' +
          'flere skærme — det er multi-scene-mønsteret du skal kopiere.</p>'
      },
      {
        shot: diagram('TEGN PIXLERNE\n        ↓\nLAV BLEEPLYDENE\n        ↓\nLAD AGENTEN LIME DET\n        ↓\nSEND AFSTED I DAG'),
        html:
          '<h3>HVAD SKAL DU LAVE FØRST</h3>' +
          '<p>Sidder du fast? Vælg ét. Det er alt sammen eftermiddags-spil med former ' +
          'agenten allerede forstår:</p>' +
          '<ul class="gh-files">' +
          '  <li><a class="gh-file"><span class="gh-file-name">FROGGER</span><span class="gh-file-desc">kryds vejen, timing bane for bane</span></a></li>' +
          '  <li><a class="gh-file"><span class="gh-file-name">TETRIS-LITE</span><span class="gh-file-desc">gitter + faldende klodser + linje-clears</span></a></li>' +
          '  <li><a class="gh-file"><span class="gh-file-name">FLAPPY</span><span class="gh-file-desc">én knap, tyngdekraft, rør med huller</span></a></li>' +
          '  <li><a class="gh-file"><span class="gh-file-name">ASTEROIDS</span><span class="gh-file-desc">fremdrift + wrap-around + skud</span></a></li>' +
          '  <li><a class="gh-file"><span class="gh-file-name">DUCK HUNT</span><span class="gh-file-desc">muse-sigte + spawn-timere</span></a></li>' +
          '</ul>' +
          '<p><strong>Drop tegneprogrammerne — generér alt i kode.</strong> ' +
          'En guldalder-sprite er et 8×8 eller 16×16 gitter af farvede firkanter; din agent ' +
          'spytter det glad ud som en <code>const PIXELS: [[u8; 16]; 16]</code> og tegner det ' +
          'med <code>fill_rect</code> i en indre løkke. Lydeffekter? En håndfuld sinus-/' +
          'firkant-sampler summet ind i en WAV-buffer — agenten skriver matematikken. ' +
          'Hurtigere, gratis, og hele spillet bliver inde i én Rust-crate.</p>' +
          '<p>Vil du have referencer? ' +
          '<a class="gh-inline" href="https://en.wikipedia.org/wiki/Pac-Man" target="_blank" rel="noopener">Pac-Man på Wikipedia ↗</a>' +
          ' har de originale sprite-skemaer i galleriet. ' +
          '<a class="gh-inline" href="https://www.spriters-resource.com/arcade/" target="_blank" rel="noopener">Spriters Resource — Arcade ↗</a>' +
          ' viser de præcise pixelgitre du skal genskabe.</p>' +
          '<p>Repoet er lige her. ' +
          '<a class="gh-inline" href="https://github.com/jacobandresen/blip" target="_blank" rel="noopener">Fork det ↗</a>' +
          ', giv dit spil et navn, og sig til agenten at den skal gå i gang. ' +
          '<strong>Næste gang nogen går forbi skabet kan det være dit spil de spiller.</strong></p>'
      },
    ];
  }

  function howtoPagesJa() {
    return [
      {
        shot: diagram('きみ + エージェント\n   ↓\nひと午後\n   ↓\nグリッドに新カード'),
        html:
          '<h3>じぶんのゲームをつくろう</h3>' +
          '<p>このキャビネットは1つのゲームから始まった。今は5つ。' +
          '<strong>6つ目はきみが追加できる。</strong></p>' +
          '<p>Rustの達人じゃなくていい。macroquadを知らなくていい。' +
          '<strong>2文で説明できるアイデア</strong>と、横で動くコーディング・' +
          'AIの相棒（' +
          '<a class="gh-inline" href="https://github.com/features/copilot" target="_blank" rel="noopener">Copilot</a>、' +
          '<a class="gh-inline" href="https://claude.com/claude-code" target="_blank" rel="noopener">Claude Code</a>、' +
          '<a class="gh-inline" href="https://cursor.com" target="_blank" rel="noopener">Cursor</a> — なんでもいい）があればいい。</p>' +
          '<p>ライブラリが面倒なところを全部やってくれる — ウィンドウ、入力、音、' +
          'ハイスコア、ウェブ橋渡し。きみは<em>面白い部分</em>を書く。' +
          '次のページはセットアップ、エージェント・プロンプト、そして午後1回で作れる' +
          '名作のリストだ。</p>'
      },
      {
        shot: diagram('エージェントが入れる：\n  rustup\n  cargo\n  wasm32 target\n  （きみは何もしない）'),
        html:
          '<h3>ステップ0 — 初回だけ</h3>' +
          '<p>これを動かすにはマシンに少しオタクっぽいものが必要だ — ' +
          'Rustのツールチェーンと、WebAssemblyのターゲット。' +
          '<strong>自分では何もインストールしないでいい。</strong>' +
          'クローン済みのリポジトリのフォルダで、新しいターミナルからコーディング・' +
          'エージェントを開いて、このプロンプトを貼り付けよう：</p>' +
          '<pre>' +
          '<span class="cmt">// セットアップ・プロンプト — 新しいマシンで貼る</span>\n\n' +
          'Set up everything I need to build and run this repo.\n\n' +
          'Check what is missing on this machine, then install:\n' +
          '  - rustup (the Rust toolchain installer)\n' +
          '  - the stable Rust compiler + cargo\n' +
          '  - the wasm32-unknown-unknown target (rustup target add ...)\n' +
          '  - python3 (for the local web server)\n\n' +
          'Then run: cargo build --workspace\n' +
          'Then run: ./build_web.sh\n' +
          'Then start the dev server: python3 -m http.server -d web 8080\n' +
          '\n' +
          'Tell me the URL to open in my browser when you are done.\n' +
          '</pre>' +
          '<p>エージェントは<em>実際に</em>インストール・コマンドを実行する' +
          '（<code>curl https://sh.rustup.rs -sSf | sh</code>、' +
          '<code>rustup target add wasm32-unknown-unknown</code> など）。' +
          '各ステップでコンパイルが通るかも確認してくれる。macOSではXcode command-line ' +
          'toolsの許可を求められるかもしれない。Linuxでは<code>build-essential</code>が' +
          '必要かもしれない。全部エージェントが扱ってくれる。</p>' +
          '<p>終わったら<code>http://localhost:8080</code>を開けば、このキャビネット自体が' +
          '表示されるはず。<strong>これで自分のゲームを追加する準備ができた。</strong></p>'
      },
      {
        shot: diagram('きみ →  「___のゲームを作って」\nエージェント → serpentを読んで\n         コピー、リネーム、ループを書き\n         cargo check、出荷'),
        html:
          '<h3>エージェントに伝えよう</h3>' +
          '<p>こんな感じでコーディング・エージェントに貼り付けよう。<code>[ブラケット]</code>を埋めるだけ：</p>' +
          '<pre>' +
          '<span class="cmt">// プロンプト・テンプレート — Copilot / Claude Code / Cursor / Cline で使える</span>\n\n' +
          'I want to add a new game to the blip arcade.\n\n' +
          '<span class="ty">Concept</span>: [1文で — 例「アステロイドだけど、自機は\n' +
          '  魚で、隕石はクラゲ」]\n\n' +
          '<span class="ty">Controls</span>: [矢印キー＋スペース、など]\n\n' +
          '<span class="ty">Win/lose</span>: [スコアX到達 / 残機0 / 時間切れ]\n\n' +
          'Use the serpent crate as a template.\n' +
          '<span class="ty">NO external assets</span> — draw all sprites in code with\n' +
          'fill_rect (golden-age sprites are tiny pixel grids) and\n' +
          'synthesize sound effects into WAV buffers in build.rs.\n' +
          'Wire it into the workspace and the web grid.\n' +
          'Run cargo build --workspace when done.\n' +
          '</pre>' +
          '<p>エージェントは' + gh('crates/serpent', 'serpent') + 'をコピーして、ループを書きなおし、' +
          'アセットをつないで、コンパイルが通るか確認する。' +
          '<strong>きみの仕事はアイデアとアート。</strong>配線はエージェントがやる。</p>' +
          '<p><em>裏ワザ：</em> 画面が複数あるゲームなら、最初に' +
          gh('crates/canaris/src/main.rs', 'canaris/main.rs') + 'を読ませよう — ' +
          'マルチシーンの参考パターンになっている。</p>'
      },
      {
        shot: diagram('ピクセルを描く\n        ↓\nブリープを作る\n        ↓\n配線はエージェントに\n        ↓\n今日出荷'),
        html:
          '<h3>なにから作る？</h3>' +
          '<p>迷ったら1つ選ぼう。どれも午後1回で組めて、エージェントが形をすでに知っているものだ：</p>' +
          '<ul class="gh-files">' +
          '  <li><a class="gh-file"><span class="gh-file-name">FROGGER</span><span class="gh-file-desc">道を渡る、車線ごとのタイミング</span></a></li>' +
          '  <li><a class="gh-file"><span class="gh-file-name">TETRIS-LITE</span><span class="gh-file-desc">グリッド＋落下ブロック＋ライン消し</span></a></li>' +
          '  <li><a class="gh-file"><span class="gh-file-name">FLAPPY</span><span class="gh-file-desc">1ボタン、重力、隙間のあるパイプ</span></a></li>' +
          '  <li><a class="gh-file"><span class="gh-file-name">ASTEROIDS</span><span class="gh-file-desc">推進＋画面ループ＋弾</span></a></li>' +
          '  <li><a class="gh-file"><span class="gh-file-name">DUCK HUNT</span><span class="gh-file-desc">マウスで狙う＋スポーン・タイマー</span></a></li>' +
          '</ul>' +
          '<p><strong>お絵かきアプリはスキップ — 全部コードで生成。</strong> ' +
          '黄金時代のスプライトは8×8や16×16の色付き格子だ。エージェントは' +
          '<code>const PIXELS: [[u8; 16]; 16]</code>で出して、入れ子ループの' +
          '<code>fill_rect</code>で描いてくれる。サウンドエフェクトは？' +
          'サイン波／矩形波のサンプルをWAVバッファに足し合わせるだけ — 計算式はエージェントが書く。' +
          '速いし、無料だし、ゲーム全体が1つのRustクレートに収まる。</p>' +
          '<p>参考が欲しい？ ' +
          '<a class="gh-inline" href="https://en.wikipedia.org/wiki/Pac-Man" target="_blank" rel="noopener">Pac-Man（Wikipedia） ↗</a>' +
          'のギャラリーにオリジナルのスプライト表がある。' +
          '<a class="gh-inline" href="https://www.spriters-resource.com/arcade/" target="_blank" rel="noopener">Spriters Resource — Arcade ↗</a>' +
          'には再現すべきピクセル格子そのものが並んでいる。</p>' +
          '<p>リポジトリはここ。' +
          '<a class="gh-inline" href="https://github.com/jacobandresen/blip" target="_blank" rel="noopener">Forkしよう ↗</a>。' +
          '名前をつけて、エージェントに「やって」と伝えるだけ。' +
          '<strong>次にだれかがこのキャビネットの前を通ったとき、遊んでいるのはきみのゲームかもしれない。</strong></p>'
      },
    ];
  }

  function howtoPages(lang) {
    if (lang === 'da') return howtoPagesDa();
    if (lang === 'ja') return howtoPagesJa();
    return howtoPagesEn();
  }

  function getPages(cardId) {
    var card = CARDS[cardId];
    var lang = currentLang();
    var base = (typeof card.pages === 'function') ? card.pages(lang) : card.pages;
    if (cardId === 'library') return howtoPages(lang).concat(base);
    return base;
  }

  function getTitle(cardId) {
    var t = CARDS[cardId].title;
    if (typeof t === 'string') return t;
    return t[currentLang()] || t.en;
  }

  // ── Card content data ────────────────────────────────────────────────
  var CARDS = {

    rally: {
      title: { en: 'RALLY — SOURCE TOUR', da: 'RALLY — KILDEKODE-TUR', ja: 'RALLY — ソースコード・ツアー' },
      pages: function (lang) {
        var SM_CODE =
          'enum State { Title, Serve, Play, GameOver }\n\n' +
          'match g.state {\n' +
          '    State::Title    => update_title(&mut g),\n' +
          '    State::Serve    => update_serve(&mut g, dt),\n' +
          '    State::Play     => update_play(&mut g, dt, &sfx),\n' +
          '    State::GameOver => update_gameover(&mut g),\n' +
          '}';
        var PHYS_CODE =
          'if ball_intersects(&g.ball, &g.p1) {\n' +
          '    g.ball.vx = g.ball.vx.abs();\n' +
          '    g.ball.vy += (g.ball.y - g.p1.y - PADDLE_H * 0.5) * 4.0;\n' +
          '    play_sfx(&sfx.blip);\n' +
          '}';
        if (lang === 'da') return [
          { shot: shot('rally/screenshot.png'),
            html: '<h3>TO-PADEL PONG</h3>' +
              '<p>Rally er det simpleste spil i skabet: to padler, én bold, først til syv. ' +
              'Det findes for at validere den fælles <code>blip</code> input- og score-pipeline ' +
              'mod den mindst mulige tilstandsmaskine.</p>' +
              '<p>Hele spillet ligger i én <code>main.rs</code> med en ~10-felts <code>Game</code>-' +
              'struct. Spiller 1 bruger <code>W/S</code>, spiller 2 bruger <code>↑/↓</code>.</p>' +
              xref('library', '→ Sådan virker input/score i biblioteket') },
          { shot: diagram('TITLE\n  ↓ btn1\nPLAY ⇄ SERVE\n  ↓ score == 7\nGAME OVER\n  ↓ btn1\nTITLE'),
            html: '<h3>TILSTANDSMASKINE</h3>' +
              '<p>Fire tilstande. <code>Serve</code> findes så bolden pauser et øjeblik efter et ' +
              'point — spillerne får tid til at samle sig.</p>' +
              code('crates/rally/src/main.rs', SM_CODE) },
          { shot: diagram('VY ← -VY  (padel-hit)\n  +\nVY += (hit_y - padel_y) * 4'),
            html: '<h3>BOLDFYSIK</h3>' +
              '<p>"Engelsken" er bare en position-offset: hvor bolden rammer padlen skævvrider ' +
              'dens lodrette hastighed. Ingen vinkler, ingen trigonometri — ren lineær nuf.</p>' +
              code('crates/rally/src/main.rs', PHYS_CODE) + xref('library') },
        ];
        if (lang === 'ja') return [
          { shot: shot('rally/screenshot.png'),
            html: '<h3>2パドル・ポン</h3>' +
              '<p>Rallyはこのキャビネットでいちばんシンプルなゲームだ：パドル2枚、ボール1個、' +
              '先に7点取った方の勝ち。最小の状態マシンで、共有の<code>blip</code>入力／' +
              'スコアパイプラインを検証するために存在している。</p>' +
              '<p>ゲーム全体は1つの<code>main.rs</code>と約10フィールドの<code>Game</code>' +
              '構造体に収まっている。プレイヤー1は<code>W/S</code>、プレイヤー2は<code>↑/↓</code>。</p>' +
              xref('library', '→ ライブラリの入力／スコア') },
          { shot: diagram('TITLE\n  ↓ btn1\nPLAY ⇄ SERVE\n  ↓ score == 7\nGAME OVER\n  ↓ btn1\nTITLE'),
            html: '<h3>状態マシン</h3>' +
              '<p>4つの状態がある。<code>Serve</code>があるのは、得点後にボールを少し止めて' +
              'プレイヤーが手を整える時間を作るためだ。</p>' +
              code('crates/rally/src/main.rs', SM_CODE) },
          { shot: diagram('VY ← -VY  (パドル衝突)\n  +\nVY += (hit_y - paddle_y) * 4'),
            html: '<h3>ボール物理</h3>' +
              '<p>「イングリッシュ」と呼ばれる回転効果は単なる位置オフセットだ：パドルのどこに' +
              '当たったかで縦速度が変わる。角度なし、三角関数なし — 純粋な線形補正だけ。</p>' +
              code('crates/rally/src/main.rs', PHYS_CODE) + xref('library') },
        ];
        return [
          { shot: shot('rally/screenshot.png'),
            html: '<h3>TWO-PADDLE PONG</h3>' +
              '<p>Rally is the simplest game in the cabinet: two paddles, one ball, first to seven. ' +
              'It exists to validate the shared <code>blip</code> input + score pipeline against ' +
              'the smallest possible state machine.</p>' +
              '<p>The whole game lives in a single <code>main.rs</code> with a ~10-field <code>Game</code> ' +
              'struct. Player 1 uses <code>W/S</code>, Player 2 uses <code>↑/↓</code>.</p>' +
              xref('library', '→ How input/score work in the library') },
          { shot: diagram('TITLE\n  ↓ btn1\nPLAY ⇄ SERVE\n  ↓ score == 7\nGAME OVER\n  ↓ btn1\nTITLE'),
            html: '<h3>STATE MACHINE</h3>' +
              '<p>Four states. <code>Serve</code> exists so the ball pauses for a beat after a point, ' +
              'giving players time to reset their hands.</p>' +
              code('crates/rally/src/main.rs', SM_CODE) },
          { shot: diagram('VY ← -VY  (paddle hit)\n  +\nVY += (hit_y - paddle_y) * 4'),
            html: '<h3>BALL PHYSICS</h3>' +
              '<p>The "english" is just a position offset: where the ball hits the paddle skews ' +
              'its vertical velocity. No angles, no trig — pure linear nudge.</p>' +
              code('crates/rally/src/main.rs', PHYS_CODE) + xref('library') },
        ];
      }
    },

    bouncer: {
      title: { en: 'BOUNCER — SOURCE TOUR', da: 'BOUNCER — KILDEKODE-TUR', ja: 'BOUNCER — ソースコード・ツアー' },
      pages: function (lang) {
        var MUSIC_CODE =
          'let mut music = MusicTracks::start(&[\n' +
          '    sfx.theme_a, sfx.theme_b, sfx.theme_c,\n' +
          ']);\n\n' +
          '// on level up:\n' +
          'music.switch_to((g.level - 1).min(2));';
        var BRICK_CODE =
          'for brick in g.bricks.iter_mut().filter(|b| b.alive) {\n' +
          '    if let Some(side) = aabb_hit(&g.ball, &brick.rect) {\n' +
          '        brick.alive = false;\n' +
          '        if side.horizontal { g.ball.vx = -g.ball.vx; }\n' +
          '        else               { g.ball.vy = -g.ball.vy; }\n' +
          '        g.score += brick.value;\n' +
          '    }\n' +
          '}';
        if (lang === 'da') return [
          { shot: shot('bouncer/screenshot.png'),
            html: '<h3>BREAKOUT MED NEON-BEAT</h3>' +
              '<p>Reflektér bolden med padlen nedenunder; ryd mursten ovenover. Tre musik-' +
              'variationer crossfader efterhånden — drevet af den fælles <code>MusicTracks</code>-' +
              'hjælper i blip-biblioteket.</p>' +
              '<p>Kontroller: <code>←/→</code> bevæg, <code>Space</code> skyd af.</p>' },
          { shot: diagram('SPOR 0 → SPOR 1 → SPOR 2\n  (niveau 1)  (niveau 2)  (niveau 3+)'),
            html: '<h3>MUSIK-LAGRING</h3>' +
              '<p>Bouncer beder biblioteket huske hvilket spor der kører, og skifter ved niveau-' +
              'grænserne. Ingen fades, ingen manuel indeksering — <code>MusicTracks::switch_to</code> ' +
              'klarer ombytningen.</p>' +
              code('crates/bouncer/src/main.rs', MUSIC_CODE) +
              xref('library', '→ MusicTracks i biblioteket') },
          { shot: shot('bouncer/screenshot.png'),
            html: '<h3>MURSTENS-GITTER</h3>' +
              '<p>En flad <code>Vec&lt;Brick&gt;</code> med række/kolonne pakket ind i entiteten. ' +
              'Kollision er AABB-mod-AABB; siden af stødet afgør om <code>vx</code> eller ' +
              '<code>vy</code> skal vendes.</p>' +
              code('crates/bouncer/src/main.rs', BRICK_CODE) },
        ];
        if (lang === 'ja') return [
          { shot: shot('bouncer/screenshot.png'),
            html: '<h3>ネオン・ビートのブロック崩し</h3>' +
              '<p>下のパドルでボールを跳ね返し、上のブロックを消す。3種類のBGMが進行に応じて' +
              'クロスフェードする — blipライブラリの共有<code>MusicTracks</code>ヘルパーが動かしている。</p>' +
              '<p>操作：<code>←/→</code>移動、<code>Space</code>発射。</p>' },
          { shot: diagram('TRACK 0 → TRACK 1 → TRACK 2\n   (Lv 1)     (Lv 2)     (Lv 3+)'),
            html: '<h3>BGMのレイヤー切り替え</h3>' +
              '<p>Bouncerはライブラリに現在再生中の曲を覚えさせ、レベルの境目で切り替える。' +
              'フェードもインデックス管理も自分でしなくていい — <code>MusicTracks::switch_to</code>が' +
              'やってくれる。</p>' +
              code('crates/bouncer/src/main.rs', MUSIC_CODE) +
              xref('library', '→ ライブラリのMusicTracks') },
          { shot: shot('bouncer/screenshot.png'),
            html: '<h3>ブロック・グリッド</h3>' +
              '<p>フラットな<code>Vec&lt;Brick&gt;</code>で、行と列はエンティティに埋め込まれている。' +
              '当たり判定はAABB対AABB。衝突した辺によって、<code>vx</code>を反転するか' +
              '<code>vy</code>を反転するかが決まる。</p>' +
              code('crates/bouncer/src/main.rs', BRICK_CODE) },
        ];
        return [
          { shot: shot('bouncer/screenshot.png'),
            html: '<h3>BREAKOUT WITH A NEON BEAT</h3>' +
              '<p>Reflect the ball with the paddle below; clear bricks above. Three music ' +
              'variations cross-fade as you progress — driven by the shared ' +
              '<code>MusicTracks</code> helper in the blip library.</p>' +
              '<p>Controls: <code>←/→</code> move, <code>Space</code> launch.</p>' },
          { shot: diagram('TRACK 0 → TRACK 1 → TRACK 2\n   (level 1)   (level 2)   (level 3+)'),
            html: '<h3>MUSIC LAYERING</h3>' +
              '<p>Bouncer asks the library to remember which track is currently playing, ' +
              'then switches at level boundaries. No fades, no manual indexing — ' +
              '<code>MusicTracks::switch_to</code> handles the swap.</p>' +
              code('crates/bouncer/src/main.rs', MUSIC_CODE) +
              xref('library', '→ MusicTracks in the library') },
          { shot: shot('bouncer/screenshot.png'),
            html: '<h3>BRICK GRID</h3>' +
              '<p>A flat <code>Vec&lt;Brick&gt;</code> with row/col packed into the entity. ' +
              'Collision is AABB-vs-AABB; the side of impact decides whether to flip ' +
              '<code>vx</code> or <code>vy</code>.</p>' +
              code('crates/bouncer/src/main.rs', BRICK_CODE) },
        ];
      }
    },

    galactic: {
      title: { en: 'GALACTIC DEFENDER — SOURCE TOUR', da: 'GALACTIC DEFENDER — KILDEKODE-TUR', ja: 'GALACTIC DEFENDER — ソースコード・ツアー' },
      pages: function (lang) {
        var STEP_CODE =
          'g.step_t += dt;\n' +
          'if g.step_t >= g.step_period {\n' +
          '    g.step_t = 0.0;\n' +
          '    g.step_period = (g.step_period * 0.97).max(0.08);\n' +
          '    march_swarm(&mut g);\n' +
          '}';
        if (lang === 'da') return [
          { shot: shot('galactic_defender/screenshot.png'),
            html: '<h3>FIXED SHOOTER</h3>' +
              '<p>Et gitter af rumvæsner marcherer til siderne, falder en række, og gentager — ' +
              'mens de bliver færre accelererer de. Du flytter dig på én akse i bunden og skyder ' +
              'opad. Mystery-tallerkner stryger lejlighedsvis hen over toppen.</p>' +
              '<p>Én dominerende <code>Play</code>-scene med en <code>~177-linjers update_play</code>; ' +
              'filen forbliver bevidst i ét stykke så marchen/skydning/kollision-rytmen kan ' +
              'læses fra top til bund.</p>' },
          { shot: diagram('RUMVÆSNER:\n  step_t += dt\n  if step_t > step_period:\n    step_period *= 0.97\n    march →\n    if kant: ned + skift retning'),
            html: '<h3>MARCH-TIMING</h3>' +
              '<p>Den ikoniske "tik … tik … tik-tik-tik"-acceleration er bare en periode der ' +
              'skrumper hver gang sværmen tager et skridt. Færre rumvæsner → ingen særlig logik ' +
              'nødvendig; perioden er afkoblet fra antallet.</p>' +
              code('crates/galactic_defender/src/main.rs', STEP_CODE) },
          { shot: shot('galactic_defender/screenshot.png'),
            html: '<h3>SKUDS-POOL</h3>' +
              '<p>Spiller- og rumvæsen-skud deler en fast pool fra biblioteket, så allokeringen ' +
              'er nul pr. frame. Inaktive pladser genbruges ved næste skud.</p>' +
              xref('library', '→ Pool i biblioteket') },
        ];
        if (lang === 'ja') return [
          { shot: shot('galactic_defender/screenshot.png'),
            html: '<h3>固定シューター</h3>' +
              '<p>エイリアンの格子が横に行進し、1段降りる、を繰り返す — 数が減るほど加速する。' +
              'プレイヤーは下で1軸だけ移動して上に撃つ。たまにミステリー円盤が上を横切る。</p>' +
              '<p><code>Play</code>シーンが支配的で、<code>update_play</code>は約177行。' +
              '行進・発射・衝突のリズムを上から下へ読めるように、わざと1つのファイルに収めている。</p>' },
          { shot: diagram('エイリアン：\n  step_t += dt\n  if step_t > step_period:\n    step_period *= 0.97\n    行進 →\n    端：下降＋反転'),
            html: '<h3>行進タイミング</h3>' +
              '<p>あの「カチ…カチ…カチカチカチ」という加速は、群れがステップするたびに縮む' +
              '周期にすぎない。残り数が減れば自然に速くなる — 周期は数とは独立だ。</p>' +
              code('crates/galactic_defender/src/main.rs', STEP_CODE) },
          { shot: shot('galactic_defender/screenshot.png'),
            html: '<h3>弾プール</h3>' +
              '<p>プレイヤーとエイリアンの弾はライブラリの固定サイズのプールを共有していて、' +
              'フレームごとのアロケーションはゼロ。非アクティブなスロットは次の発射で再利用される。</p>' +
              xref('library', '→ ライブラリのPool') },
        ];
        return [
          { shot: shot('galactic_defender/screenshot.png'),
            html: '<h3>FIXED SHOOTER</h3>' +
              '<p>A grid of aliens marches sideways, drops a row, and repeats — accelerating ' +
              'as their numbers thin. You move on a single axis at the bottom and shoot up. ' +
              'Mystery saucers occasionally streak across the top.</p>' +
              '<p>Single dominant <code>Play</code> scene with a <code>~177-line update_play</code>; ' +
              'the file deliberately stays one piece so the marching/firing/collision rhythm is ' +
              'readable top-to-bottom.</p>' },
          { shot: diagram('ALIENS:\n  step_t += dt\n  if step_t > step_period:\n    step_period *= 0.97\n    march →\n    if edge: drop_row + flip_dir'),
            html: '<h3>MARCH TIMING</h3>' +
              '<p>The signature "tick … tick … tick-tick-tick" acceleration is just a ' +
              'period that shrinks every time the swarm steps. Fewer aliens → no special ' +
              'logic needed; the period is decoupled from the count.</p>' +
              code('crates/galactic_defender/src/main.rs', STEP_CODE) },
          { shot: shot('galactic_defender/screenshot.png'),
            html: '<h3>BULLET POOL</h3>' +
              '<p>Player and alien bullets share a fixed-size pool from the library, so ' +
              'allocation is zero per frame. Inactive slots are reused on the next fire.</p>' +
              xref('library', '→ Pool in the library') },
        ];
      }
    },

    serpent: {
      title: { en: 'SERPENT — SOURCE TOUR', da: 'SERPENT — KILDEKODE-TUR', ja: 'SERPENT — ソースコード・ツアー' },
      pages: function (lang) {
        var DEQUE_CODE =
          'let new_head = step(g.body.front().copied().unwrap(), g.dir);\n' +
          'g.body.push_front(new_head);\n' +
          'if new_head == g.food { spawn_food(&mut g); g.score += 10; }\n' +
          'else                  { g.body.pop_back(); }';
        var INPUT_CODE =
          'if key_pressed(BLIP_KEY_UP)    { g.next_dir = Dir::Up; }\n' +
          'if key_pressed(BLIP_KEY_DOWN)  { g.next_dir = Dir::Down; }\n' +
          '// on tick:\n' +
          'if !g.next_dir.is_opposite(g.dir) { g.dir = g.next_dir; }';
        if (lang === 'da') return [
          { shot: shot('serpent/screenshot.png'),
            html: '<h3>GITTER-FLYTTER</h3>' +
              '<p>Spis mad for at vokse. Undgå væggene og dig selv. Gitteret tegnes som diskrete ' +
              'celler; tik affyres hver <code>TICK_PERIOD</code> sekund — input bufres mellem ' +
              'tik, så retningsvalg føles responsive.</p>' },
          { shot: diagram('HOVED ← bufret input\nHALE ← pop medmindre den åd mad'),
            html: '<h3>KROP SOM DEQUE</h3>' +
              '<p>Slangen er en <code>VecDeque&lt;(i32,i32)&gt;</code>. Hvert tik: push et nyt ' +
              'hoved i nuværende retning; pop halen medmindre hovedet landede på mad.</p>' +
              code('crates/serpent/src/main.rs', DEQUE_CODE) },
          { shot: shot('serpent/screenshot.png'),
            html: '<h3>INPUT-BUFFER</h3>' +
              '<p>Tryk Op lige efter Venstre ville være en 180°-omvending på samme tik — ulovlig. ' +
              'Bufferen gemmer *næste* retning; tikket bruger den kun, hvis den ikke er en omvending.</p>' +
              code('crates/serpent/src/main.rs', INPUT_CODE) },
        ];
        if (lang === 'ja') return [
          { shot: shot('serpent/screenshot.png'),
            html: '<h3>グリッド・ムーバー</h3>' +
              '<p>エサを食べて伸びる。壁と自分の体を避ける。グリッドは離散セルとして描画され、' +
              'ティックは<code>TICK_PERIOD</code>秒ごとに発火する — ティックの間は入力がバッファされる' +
              'ので、方向転換が機敏に感じられる。</p>' },
          { shot: diagram('頭 ← バッファされた入力\n尾 ← 食べていなければpop'),
            html: '<h3>体はデックで持つ</h3>' +
              '<p>蛇は<code>VecDeque&lt;(i32,i32)&gt;</code>だ。毎ティック：現在の方向に新しい頭を' +
              'pushし、頭がエサ上でなければ尾をpopする。</p>' +
              code('crates/serpent/src/main.rs', DEQUE_CODE) },
          { shot: shot('serpent/screenshot.png'),
            html: '<h3>入力バッファ</h3>' +
              '<p>左の直後に上を押すと同じティックで180°反転になる — それは不正だ。' +
              'バッファは*次の*方向を覚えておき、反転でなければティックがそれを採用する。</p>' +
              code('crates/serpent/src/main.rs', INPUT_CODE) },
        ];
        return [
          { shot: shot('serpent/screenshot.png'),
            html: '<h3>GRID MOVER</h3>' +
              '<p>Eat food to grow longer. Avoid the walls and yourself. The grid is ' +
              'rendered as discrete cells; ticks fire every <code>TICK_PERIOD</code> seconds ' +
              '— input is buffered between ticks so directional turns feel responsive.</p>' },
          { shot: diagram('HEAD ← buffered input\nTAIL ← pop unless ate food'),
            html: '<h3>BODY AS DEQUE</h3>' +
              '<p>The snake is a <code>VecDeque&lt;(i32,i32)&gt;</code>. Each tick: push a new head ' +
              'in the current direction; pop the tail unless the head landed on food.</p>' +
              code('crates/serpent/src/main.rs', DEQUE_CODE) },
          { shot: shot('serpent/screenshot.png'),
            html: '<h3>INPUT BUFFER</h3>' +
              '<p>Pressing Up immediately after Left would be a 180° reversal on the same tick — ' +
              'illegal. The buffer stores the *next* direction; the tick consumes it only if it ' +
              'isn\'t a reversal.</p>' +
              code('crates/serpent/src/main.rs', INPUT_CODE) },
        ];
      }
    },

    canaris: {
      title: { en: 'CANARIS — SOURCE TOUR', da: 'CANARIS — KILDEKODE-TUR', ja: 'CANARIS — ソースコード・ツアー' },
      pages: function (lang) {
        var DISPATCH_CODE =
          'match g.state {\n' +
          '    State::Title    => update_title(&mut g, dt),\n' +
          '    State::Sea      => update_sea(&mut g, dt, &sfx),\n' +
          '    State::Combat   => update_combat(&mut g, dt, &sfx),\n' +
          '    State::Boarding => update_boarding(&mut g, dt, &sfx),\n' +
          '    State::Port     => update_port(&mut g, dt, &sfx),\n' +
          '    State::Map      => update_map(&mut g, dt, &sfx),\n' +
          '    State::Dead     => update_dead(&mut g, dt, &sfx),\n' +
          '    State::GameOver => update_gameover(&mut g),\n' +
          '}';
        var SEA_CODE =
          'pub fn update_sea(g: &mut Game, dt: f32, sfx: &Sounds) {\n' +
          '    // wave anim, ship movement, hunger tick...\n' +
          '    for (i, e) in g.enemies.iter().enumerate() {\n' +
          '        if !e.active { continue; }\n' +
          '        if dist(g.player.pos, e.pos) < COMBAT_RANGE {\n' +
          '            g.enter_combat(i);\n' +
          '            return;\n' +
          '        }\n' +
          '    }\n' +
          '}';
        var COMBAT_CODE =
          'g.cannonballs[i] = Cannonball {\n' +
          '    active: true,\n' +
          '    x:  COMBAT_PLAYER_X + PLAYER_W,\n' +
          '    y:  g.player.y + PLAYER_H * 0.55,\n' +
          '    vx: CANNON_SPEED,\n' +
          '    vy: -CANNON_ARC_VY,\n' +
          '    player: true,\n' +
          '};';
        var BOARD_CODE =
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
          '}';
        var PORT_CODE =
          'if btn1_pressed() {\n' +
          '    let z = &ZONES[g.map_cursor];\n' +
          '    g.level   = z.level_eq;\n' +
          '    g.spawn_enemies_n(z.ships);\n' +
          '    g.state = State::Sea;\n' +
          '}';
        if (lang === 'da') return [
          { shot: shot('canaris/screenshot.png?v=2'),
            html: '<h3>ÅBEN-VERDENS SØMAND</h3>' +
              '<p>Inspireret af <em>Kaptajn Kaper i Kattegat</em> (1985). Sejl Kattegat, ' +
              'spot fjendtlige skibe, vælg din strid: kanon-duel, bord for plyndring, eller ' +
              'flygt til havn for reparation og forsyning.</p>' +
              '<p>Spillet er bygget som <strong>syv scener</strong>, én pr. tilstand, hver i sit ' +
              'eget modul. <code>main.rs</code> er en lille dispatcher.</p>' },
          { shot: diagram('TITLE → SEA ⇄ COMBAT\n         ↕      ↓\n        PORT ⇄ MAP\n         ↑      ↓\n        DEAD ← BOARDING\n         ↓\n      GAMEOVER'),
            html: '<h3>SCENE-DISPATCH</h3>' +
              '<p>Hvert scene-modul eksporterer <code>update_X</code> og <code>draw_X</code>. ' +
              '<code>main.rs</code> vælger én baseret på den aktuelle <code>State</code>.</p>' +
              code('crates/canaris/src/main.rs', DISPATCH_CODE, '133-142') },
          { shot: shot('canaris/screenshot.png?v=2'),
            html: '<h3>SEA — FRI SEJLADS</h3>' +
              '<p>Topdown-sejlads med en to-frame-bølgeanimation nedenunder. Fjendeskibe vandrer ' +
              'verden rundt; bliver du tæt nok, udløses overgangen til <code>Combat</code>.</p>' +
              code('crates/canaris/src/sea.rs', SEA_CODE) },
          { shot: shot('canaris/screenshot.png?v=2'),
            html: '<h3>COMBAT — KANON-DUEL</h3>' +
              '<p>Side-om-side bredsider. Kanonkugler buer med en fast lodret impuls og tyngdekraft; ' +
              'skrog-træf-sprøjt bliver tilbagemeldt via fælles sfx-slots.</p>' +
              code('crates/canaris/src/combat.rs', COMBAT_CODE) },
          { shot: diagram('DIN BESÆTNING│FJENDE-BESÆT.\n  ▓ ▓ ▓     │    ▓ ▓ ▓\n   ↑ angriber yderste spiller-slot\n   ↑ spiller angriber yderste fjende-slot'),
            html: '<h3>BOARDING — BESÆTNINGS-PLADSER</h3>' +
              '<p>Duellen modelleres som tre pladser pr. side. Tryk <code>[1]</code> angriber ' +
              'den forreste fjende-plads; fjenden auto-tikker mod din yderste plads. Når en ' +
              'plads når nul, indtager angriberen den (med 2 HP).</p>' +
              code('crates/canaris/src/boarding.rs', BOARD_CODE, '20-35') },
          { shot: shot('canaris/screenshot.png?v=2'),
            html: '<h3>HAVN & KORT</h3>' +
              '<p>Havnen er en seks-felts butiksmenu (Sejl, Kort, Reparér, Besætning, Kanoner, ' +
              'Mad). Kortet er en sigtekorn-zonevælger over et statisk Kattegat-baggrundsbillede — ' +
              'at vælge en zone seeder <code>spawn_enemies_n</code> med zonens skibstal og ' +
              'sværhedsgrad.</p>' +
              code('crates/canaris/src/port.rs', PORT_CODE, '140-147') +
              xref('library', '→ Sådan ligger saves & i18n i biblioteket') },
        ];
        if (lang === 'ja') return [
          { shot: shot('canaris/screenshot.png?v=2'),
            html: '<h3>オープンワールド航海</h3>' +
              '<p><em>Kaptajn Kaper i Kattegat</em>（1985）にインスパイアされた作品。' +
              'カテガット海峡を航行し、敵船を見つけたら戦い方を選ぼう：大砲で撃ち合う、' +
              '乗り込んで略奪する、あるいは港まで逃げて修理と補給をする。</p>' +
              '<p>このゲームは<strong>7つのシーン</strong>として組まれていて、状態ごとに1つの' +
              'モジュールに分かれている。<code>main.rs</code>は小さなディスパッチャだ。</p>' },
          { shot: diagram('TITLE → SEA ⇄ COMBAT\n         ↕      ↓\n        PORT ⇄ MAP\n         ↑      ↓\n        DEAD ← BOARDING\n         ↓\n      GAMEOVER'),
            html: '<h3>シーン・ディスパッチ</h3>' +
              '<p>各シーンモジュールは<code>update_X</code>と<code>draw_X</code>を公開する。' +
              '<code>main.rs</code>が現在の<code>State</code>に応じて1つを選ぶ。</p>' +
              code('crates/canaris/src/main.rs', DISPATCH_CODE, '133-142') },
          { shot: shot('canaris/screenshot.png?v=2'),
            html: '<h3>SEA — 自由航行</h3>' +
              '<p>真上から見下ろす航行画面の下に、2フレームの波アニメーションが流れる。' +
              '敵船は世界をうろつき、十分に近づくと<code>Combat</code>へ遷移する。</p>' +
              code('crates/canaris/src/sea.rs', SEA_CODE) },
          { shot: shot('canaris/screenshot.png?v=2'),
            html: '<h3>COMBAT — 大砲の決闘</h3>' +
              '<p>横並びの舷側砲撃。砲弾は固定の縦インパルスと重力で弧を描く。船体への命中時の' +
              '水しぶきは共通のsfxスロット経由でフィードバックされる。</p>' +
              code('crates/canaris/src/combat.rs', COMBAT_CODE) },
          { shot: diagram('じぶんの乗組員│敵の乗組員\n  ▓ ▓ ▓     │    ▓ ▓ ▓\n   ↑ 敵が一番右のプレイヤースロットを攻撃\n   ↑ プレイヤーが一番左の敵スロットを攻撃'),
            html: '<h3>BOARDING — 乗組員スロット</h3>' +
              '<p>白兵戦は片側3スロットでモデル化されている。<code>[1]</code>で一番手前の敵スロットを' +
              '攻撃；敵は自動ティックで一番右のプレイヤースロットを攻撃する。スロットが0になると、' +
              '攻撃側がそれを占拠する（HP2で）。</p>' +
              code('crates/canaris/src/boarding.rs', BOARD_CODE, '20-35') },
          { shot: shot('canaris/screenshot.png?v=2'),
            html: '<h3>港 & 地図</h3>' +
              '<p>港は6項目の店メニュー（出航、地図、修理、乗組員、大砲、食料）。地図は静止画の' +
              'カテガット海峡の上にある十字カーソル・ゾーン選択画面 — ゾーンを選ぶと、その難易度' +
              'と船数で<code>spawn_enemies_n</code>がシードされる。</p>' +
              code('crates/canaris/src/port.rs', PORT_CODE, '140-147') +
              xref('library', '→ ライブラリの保存とi18n') },
        ];
        return [
          { shot: shot('canaris/screenshot.png?v=2'),
            html: '<h3>OPEN-WORLD SAILOR</h3>' +
              '<p>Inspired by <em>Kaptajn Kaper i Kattegat</em> (1985). Sail the Kattegat, ' +
              'spot enemy ships, choose your engagement: cannon duel, board for plunder, or ' +
              'run for port to repair and re-provision.</p>' +
              '<p>The game is built as <strong>seven scenes</strong>, one per state, each in its ' +
              'own module. The <code>main.rs</code> is a tiny dispatcher.</p>' },
          { shot: diagram('TITLE → SEA ⇄ COMBAT\n         ↕      ↓\n        PORT ⇄ MAP\n         ↑      ↓\n        DEAD ← BOARDING\n         ↓\n      GAMEOVER'),
            html: '<h3>SCENE DISPATCH</h3>' +
              '<p>Every scene module exports <code>update_X</code> and <code>draw_X</code>. ' +
              '<code>main.rs</code> picks one based on the current <code>State</code>.</p>' +
              code('crates/canaris/src/main.rs', DISPATCH_CODE, '133-142') },
          { shot: shot('canaris/screenshot.png?v=2'),
            html: '<h3>SEA — FREE ROAM</h3>' +
              '<p>Top-down sailing with a two-frame wave animation underneath. Enemy ships ' +
              'wander the world; getting close enough triggers a transition to <code>Combat</code>.</p>' +
              code('crates/canaris/src/sea.rs', SEA_CODE) },
          { shot: shot('canaris/screenshot.png?v=2'),
            html: '<h3>COMBAT — CANNON DUEL</h3>' +
              '<p>Side-by-side broadsides. Cannonballs arc with a fixed vertical impulse ' +
              'and gravity; hull-hit splashes feed back via shared sfx slots.</p>' +
              code('crates/canaris/src/combat.rs', COMBAT_CODE) },
          { shot: diagram('YOUR CREW  │  ENEMY CREW\n  ▓ ▓ ▓    │    ▓ ▓ ▓\n   ↑ attack rightmost player\n   ↑ player attacks leftmost enemy'),
            html: '<h3>BOARDING — CREW SLOTS</h3>' +
              '<p>The duel is modeled as three slots per side. Pressing <code>[1]</code> attacks ' +
              'the frontmost enemy slot; the enemy auto-ticks against your rightmost slot. ' +
              'When a slot drops to zero, the attacker captures it (with 2 HP).</p>' +
              code('crates/canaris/src/boarding.rs', BOARD_CODE, '20-35') },
          { shot: shot('canaris/screenshot.png?v=2'),
            html: '<h3>PORT & MAP</h3>' +
              '<p>Port is a six-item shop menu (Sail, Map, Repair, Crew, Cannons, Food). ' +
              'The Map is a crosshair zone selector over a static Kattegat backdrop — picking ' +
              'a zone seeds <code>spawn_enemies_n</code> with that zone\'s ship count and ' +
              'difficulty level.</p>' +
              code('crates/canaris/src/port.rs', PORT_CODE, '140-147') +
              xref('library', '→ How saves & i18n live in the library') },
        ];
      }
    },

    library: {
      title: { en: 'BLIP — SHARED LIBRARY', da: 'BLIP — FÆLLES BIBLIOTEK', ja: 'BLIP — 共有ライブラリ' },
      pages: function (lang) {
        var SESSION_CODE =
          'pub struct Session {\n' +
          '    pub game_id: &\'static str,\n' +
          '    pub hi_score: i32,\n' +
          '}\n\n' +
          'impl Session {\n' +
          '    pub fn notify_game_over(&self, score: i32) {\n' +
          '        web::save_hi_score(self.game_id, score);\n' +
          '    }\n' +
          '}';
        var MUSIC_CODE =
          'pub struct MusicTracks<\'a> {\n' +
          '    tracks: &\'a [BlipSound],\n' +
          '    current: usize,\n' +
          '}\n\n' +
          'impl<\'a> MusicTracks<\'a> {\n' +
          '    pub fn start(tracks: &\'a [BlipSound]) -> Self { /* ... */ }\n' +
          '    pub fn switch_to(&mut self, idx: usize) { /* stop+play */ }\n' +
          '    pub fn current(&self) -> usize { self.current }\n' +
          '}';
        var MACRO_CODE =
          '#[macro_export]\n' +
          'macro_rules! blip_image {\n' +
          '    ($name:literal) => {\n' +
          '        include_bytes!(concat!(\n' +
          '            env!("OUT_DIR"), "/assets/images/", $name\n' +
          '        ))\n' +
          '    };\n' +
          '}';
        var TREE = 'crates/blip/\n├ lib.rs\n├ session.rs\n├ audio.rs\n├ web.rs\n├ texture.rs\n└ input.rs';
        if (lang === 'da') return [
          { shot: diagram(TREE),
            html: '<h3>HVAD BIBLIOTEKET LAVER</h3>' +
              '<p>Hvert spil i skabet linker mod én crate: <code>blip</code>. Den ejer ' +
              'vinduesopsætningen, input-kortet, score-persistens, lydafspilning og asset-' +
              'macroerne — så hvert spils <code>main.rs</code> kan handle om mekanikken, ' +
              'ikke om rørene.</p>' +
              '<p><strong>Modul-indeks</strong> — klik for at læse på GitHub:</p>' +
              ghList([
                { path: 'crates/blip/src/lib.rs',      desc: 'vinduesopsætning, re-eksport, blip_image! / blip_sound!' },
                { path: 'crates/blip/src/session.rs',  desc: 'spil-session + highscore-persistens' },
                { path: 'crates/blip/src/audio.rs',    desc: 'musik, sfx, ambient; MusicTracks-hjælper' },
                { path: 'crates/blip/src/input.rs',    desc: 'samlet BLIP_KEY_* + btn1/btn2 (tastatur + gamepad)' },
                { path: 'crates/blip/src/texture.rs',  desc: 'load_png med pixel-art-filter-preset' },
                { path: 'crates/blip/src/web.rs',      desc: 'Supabase highscore-bro (WASM ↔ JS)' },
              ]) },
          { shot: diagram('Session\n ├ game_id\n ├ hi_score\n └ notify_game_over()'),
            html: '<h3>SESSION + HIGHSCORE</h3>' +
              '<p>Hvert spil konstruerer en <code>Session</code> med sit eget spil-id. Sessionen ' +
              'husker den aktuelle runde og videresender score-opdateringer til JS-broen ' +
              '(Supabase-backed) ved game over. Se ' + gh('crates/blip/src/session.rs', 'session.rs') +
              ' og JS-siden i ' + gh('web/blip_bridge.js', 'blip_bridge.js') + '.</p>' +
              code('crates/blip/src/session.rs', SESSION_CODE) },
          { shot: diagram('MusicTracks::start(&[a, b, c])\n          ↓\n  switch_to(idx) — stop gammel, start ny'),
            html: '<h3>LYD — MusicTracks</h3>' +
              '<p>' + gh('crates/bouncer/src/main.rs', 'Bouncer') + ' havde brug for tre tema-' +
              'variationer der skiftede ved niveau-grænser; mønsteret blev trukket ind i ' +
              gh('crates/blip/src/audio.rs', 'audio.rs') + ', så ethvert fremtidigt spil kan ' +
              'gøre det samme med to linjer.</p>' +
              code('crates/blip/src/audio.rs', MUSIC_CODE) },
          { shot: diagram('blip_image!("foo.png")\n  → include_bytes!(concat!(OUT_DIR, "/assets/images/foo.png"))'),
            html: '<h3>ASSET-MACROER</h3>' +
              '<p>Hvert spil har en <code>build.rs</code> (fx ' +
              gh('crates/canaris/build.rs', 'canaris/build.rs') + ') der kopierer PNG\'er og ' +
              'WAV\'er ind i <code>$OUT_DIR/assets/</code>. Biblioteket eksponerer to macroer der ' +
              'gør et bart filnavn til en <code>&amp;[u8]</code> ved compile-tid — ingen sti-' +
              'strenge dryssende rundt i spil-koden. Den fælles asset-pipeline ligger i ' +
              gh('crates/blip_assets/src/lib.rs', 'blip_assets') + '.</p>' +
              code('crates/blip/src/lib.rs', MACRO_CODE) },
          { shot: diagram('Pool<T, N>\n ├ slice af pladser\n ├ acquire() → &mut T\n └ active() iterator'),
            html: '<h3>POOL & TIMER</h3>' +
              '<p>Faste-størrelses arrays med et <code>active</code>-flag — nul allokering i ' +
              'frame-loopen. Galactic Defender bruger én til skud; Canaris bruger flere ' +
              '(<code>cannonballs</code>, <code>splashes</code>, <code>explosions</code>).</p>' +
              '<p><code>blip::load_png</code> i ' + gh('crates/blip/src/texture.rs', 'texture.rs') +
              ' wrapper macroquads loader med pixel-art-filter-presettet — ellers ville hvert ' +
              'spil gentage de samme tre linjer.</p>' +
              '<p><strong>Hvor man skal kigge:</strong></p>' +
              ghList([
                { path: 'crates/blip/src/lib.rs',     desc: 'Pool<T,N> + Timer (re-eksporteret på crate-roden)' },
                { path: 'crates/blip/src/texture.rs', desc: 'load_png pixel-art-hjælper' },
                { path: 'crates/galactic_defender/src/main.rs', desc: 'Pool i brug — skud-array' },
                { path: 'crates/canaris/src/state.rs', desc: 'Pool i brug — cannonballs / splashes / explosions' },
              ]) },
        ];
        if (lang === 'ja') return [
          { shot: diagram(TREE),
            html: '<h3>ライブラリは何をするか</h3>' +
              '<p>このキャビネットの全ゲームが1つのクレート<code>blip</code>とリンクしている。' +
              'ウィンドウ初期化、入力マップ、スコア保存、音声再生、アセット読み込みマクロ — ' +
              '全部ここに入っているので、各ゲームの<code>main.rs</code>はメカニクスだけに集中できる。</p>' +
              '<p><strong>モジュール索引</strong> — クリックでGitHubを開く：</p>' +
              ghList([
                { path: 'crates/blip/src/lib.rs',      desc: 'ウィンドウ設定、re-export、blip_image! / blip_sound!' },
                { path: 'crates/blip/src/session.rs',  desc: 'ゲーム別セッション＋ハイスコア保存' },
                { path: 'crates/blip/src/audio.rs',    desc: '音楽・SE・環境音；MusicTracksヘルパー' },
                { path: 'crates/blip/src/input.rs',    desc: '統一されたBLIP_KEY_* + btn1/btn2（キーボード＋ゲームパッド）' },
                { path: 'crates/blip/src/texture.rs',  desc: 'load_pngのピクセルアート用プリセット' },
                { path: 'crates/blip/src/web.rs',      desc: 'Supabaseハイスコア橋（WASM ↔ JS）' },
              ]) },
          { shot: diagram('Session\n ├ game_id\n ├ hi_score\n └ notify_game_over()'),
            html: '<h3>セッションとハイスコア</h3>' +
              '<p>各ゲームは自分のゲームIDを持つ<code>Session</code>を作る。セッションは現在の' +
              'プレイを記憶し、ゲームオーバー時にJS橋（Supabaseバックエンド）へスコア更新を' +
              '送信する。詳細は' + gh('crates/blip/src/session.rs', 'session.rs') + 'と、' +
              'JS側の' + gh('web/blip_bridge.js', 'blip_bridge.js') + 'を参照。</p>' +
              code('crates/blip/src/session.rs', SESSION_CODE) },
          { shot: diagram('MusicTracks::start(&[a, b, c])\n          ↓\n  switch_to(idx) — 古いのを止めて新しいのを再生'),
            html: '<h3>音声 — MusicTracks</h3>' +
              '<p>' + gh('crates/bouncer/src/main.rs', 'Bouncer') + 'はレベル境界で曲を切り替える' +
              '3バリエーションが必要だった。そのパターンを' +
              gh('crates/blip/src/audio.rs', 'audio.rs') + 'に引き上げたので、今後どのゲームも' +
              '2行で同じことができる。</p>' +
              code('crates/blip/src/audio.rs', MUSIC_CODE) },
          { shot: diagram('blip_image!("foo.png")\n  → include_bytes!(concat!(OUT_DIR, "/assets/images/foo.png"))'),
            html: '<h3>アセット・マクロ</h3>' +
              '<p>各ゲームは<code>build.rs</code>（例：' +
              gh('crates/canaris/build.rs', 'canaris/build.rs') + '）でPNGやWAVを' +
              '<code>$OUT_DIR/assets/</code>へコピーする。ライブラリは2つのマクロを公開していて、' +
              'ファイル名だけを渡せばコンパイル時に<code>&amp;[u8]</code>になる — ' +
              'ゲームコードにパス文字列を撒き散らさなくていい。共有のアセットパイプラインは' +
              gh('crates/blip_assets/src/lib.rs', 'blip_assets') + 'にある。</p>' +
              code('crates/blip/src/lib.rs', MACRO_CODE) },
          { shot: diagram('Pool<T, N>\n ├ スロットのスライス\n ├ acquire() → &mut T\n └ active() イテレータ'),
            html: '<h3>POOL & TIMER</h3>' +
              '<p>固定サイズの配列に<code>active</code>フラグを付けたもの — フレームループ中の' +
              'アロケーションはゼロ。Galactic Defenderは弾用に1つ使い、Canarisは複数使っている' +
              '（<code>cannonballs</code>、<code>splashes</code>、<code>explosions</code>）。</p>' +
              '<p>' + gh('crates/blip/src/texture.rs', 'texture.rs') + 'にある' +
              '<code>blip::load_png</code>は、macroquadのローダーをピクセルアート用のフィルター' +
              '設定でラップしている — でなければ各ゲームが同じ3行を繰り返すことになる。</p>' +
              '<p><strong>見るべき場所：</strong></p>' +
              ghList([
                { path: 'crates/blip/src/lib.rs',     desc: 'Pool<T,N> + Timer（クレート直下からre-export）' },
                { path: 'crates/blip/src/texture.rs', desc: 'load_pngピクセルアート用ヘルパー' },
                { path: 'crates/galactic_defender/src/main.rs', desc: '使用例 — 弾の配列' },
                { path: 'crates/canaris/src/state.rs', desc: '使用例 — cannonballs / splashes / explosions' },
              ]) },
        ];
        return [
          { shot: diagram(TREE),
            html: '<h3>WHAT THE LIBRARY DOES</h3>' +
              '<p>Every game in the cabinet links one crate: <code>blip</code>. It owns the ' +
              'window setup, the input map, score persistence, audio playback, and the ' +
              'asset-loading macros — so each game\'s <code>main.rs</code> stays about the ' +
              'mechanic, not the plumbing.</p>' +
              '<p><strong>Module index</strong> — click to read on GitHub:</p>' +
              ghList([
                { path: 'crates/blip/src/lib.rs',      desc: 'window setup, re-exports, blip_image! / blip_sound! macros' },
                { path: 'crates/blip/src/session.rs',  desc: 'per-game session + hi-score persistence' },
                { path: 'crates/blip/src/audio.rs',    desc: 'music, sfx, ambient; MusicTracks helper' },
                { path: 'crates/blip/src/input.rs',    desc: 'unified BLIP_KEY_* + btn1/btn2 (keyboard + gamepad)' },
                { path: 'crates/blip/src/texture.rs',  desc: 'load_png with pixel-art filter preset' },
                { path: 'crates/blip/src/web.rs',      desc: 'Supabase hi-score bridge (WASM ↔ JS)' },
              ]) },
          { shot: diagram('Session\n ├ game_id\n ├ hi_score\n └ notify_game_over()'),
            html: '<h3>SESSION + HI-SCORE</h3>' +
              '<p>Each game constructs a <code>Session</code> with its own game id. The session ' +
              'remembers the current run and forwards score updates to the JS bridge ' +
              '(Supabase-backed) on game over. See ' + gh('crates/blip/src/session.rs', 'session.rs') +
              ' and the JS side in ' + gh('web/blip_bridge.js', 'blip_bridge.js') + '.</p>' +
              code('crates/blip/src/session.rs', SESSION_CODE) },
          { shot: diagram('MusicTracks::start(&[a, b, c])\n          ↓\n  switch_to(idx) — stops old, plays new'),
            html: '<h3>AUDIO — MusicTracks</h3>' +
              '<p>' + gh('crates/bouncer/src/main.rs', 'Bouncer') + ' needed three theme ' +
              'variations to swap at level boundaries; the pattern got pulled into ' +
              gh('crates/blip/src/audio.rs', 'audio.rs') + ' so any future game can do the ' +
              'same with two lines.</p>' +
              code('crates/blip/src/audio.rs', MUSIC_CODE) },
          { shot: diagram('blip_image!("foo.png")\n  → include_bytes!(concat!(OUT_DIR, "/assets/images/foo.png"))'),
            html: '<h3>ASSET MACROS</h3>' +
              '<p>Each game has a <code>build.rs</code> (e.g. ' +
              gh('crates/canaris/build.rs', 'canaris/build.rs') + ') that copies its PNGs and ' +
              'WAVs into <code>$OUT_DIR/assets/</code>. The library exposes two macros that ' +
              'turn a bare filename into an <code>&amp;[u8]</code> at compile time — no path ' +
              'strings sprinkled through game code. The shared asset pipeline lives in ' +
              gh('crates/blip_assets/src/lib.rs', 'blip_assets') + '.</p>' +
              code('crates/blip/src/lib.rs', MACRO_CODE) },
          { shot: diagram('Pool<T, N>\n ├ slice of slots\n ├ acquire() → &mut T\n └ active() iterator'),
            html: '<h3>POOL & TIMER</h3>' +
              '<p>Fixed-capacity arrays with an <code>active</code> flag — no allocation in the ' +
              'frame loop. Galactic Defender uses one for bullets; Canaris uses several ' +
              '(<code>cannonballs</code>, <code>splashes</code>, <code>explosions</code>).</p>' +
              '<p><code>blip::load_png</code> in ' + gh('crates/blip/src/texture.rs', 'texture.rs') +
              ' wraps macroquad\'s loader with the pixel-art filter preset — every game would ' +
              'otherwise repeat the same three lines.</p>' +
              '<p><strong>Where to look:</strong></p>' +
              ghList([
                { path: 'crates/blip/src/lib.rs',     desc: 'Pool<T,N> + Timer (re-exported at crate root)' },
                { path: 'crates/blip/src/texture.rs', desc: 'load_png pixel-art helper' },
                { path: 'crates/galactic_defender/src/main.rs', desc: 'Pool in use — bullet array' },
                { path: 'crates/canaris/src/state.rs', desc: 'Pool in use — cannonballs / splashes / explosions' },
              ]) },
        ];
      }
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
    var pages = getPages(cardId);
    var page = pages[pageIdx];
    var dots = pages.map(function (_, i) {
      return '<span class="' + (i === pageIdx ? 'on' : '') + '" data-go="' + i + '"></span>';
    }).join('');
    return (
      '<div class="docs-head">' +
        '<div class="title">' + getTitle(cardId) + '</div>' +
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
    var pages = getPages(openCardId);
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
    // Free the state immediately so a subsequent badge click can re-open.
    // (The flipper element keeps animating until cleanup; we just stop guarding open().)
    var closingFlipper = flipper;
    var rect = sourceCardEl ? sourceCardEl.getBoundingClientRect() : null;
    backdrop.classList.remove('open');
    closingFlipper.classList.remove('open');
    closingFlipper.style.pointerEvents = 'none';
    if (rect) {
      closingFlipper.style.top    = rect.top  + 'px';
      closingFlipper.style.left   = rect.left + 'px';
      closingFlipper.style.width  = rect.width  + 'px';
      closingFlipper.style.height = rect.height + 'px';
    }
    flipper = null;
    sourceCardEl = null;
    openCardId = null;
    pageIdx = 0;
    setTimeout(function () {
      if (closingFlipper.parentNode) closingFlipper.parentNode.removeChild(closingFlipper);
      if (typeof afterFn === 'function') afterFn();
    }, 560);
  }

  // ── Wire badges + keyboard ───────────────────────────────────────────
  function init() {
    buildOverlay();
    document.querySelectorAll('.badge-see, .see-how').forEach(function (trigger) {
      trigger.addEventListener('click', function (e) {
        // Cards are anchors — without BOTH calls, navigation fires under the flip.
        e.stopPropagation();
        e.preventDefault();
        var cardId = trigger.getAttribute('data-see');
        var cardEl = trigger.closest('[data-card-id]');
        if (cardEl) open(cardId, cardEl);
      });
    });
    // The library card has no game to launch — clicking anywhere opens the tour.
    var libCard = document.querySelector('[data-card-id="library"]');
    if (libCard) {
      libCard.style.cursor = 'pointer';
      libCard.addEventListener('click', function (e) {
        // Let nested buttons/badges/links handle their own clicks.
        if (e.target.closest('a, button, .badge-see, .see-how')) return;
        open('library', libCard);
      });
    }
    document.addEventListener('keydown', function (e) {
      if (!openCardId) return;
      if (e.key === 'Escape')     { e.preventDefault(); close(); }
      else if (e.key === 'ArrowRight') { e.preventDefault(); goPage(pageIdx + 1); }
      else if (e.key === 'ArrowLeft')  { e.preventDefault(); goPage(pageIdx - 1); }
    });
    // Re-render the open card if the user switches site language mid-tour.
    new MutationObserver(function () {
      if (!openCardId || !flipper) return;
      var pages = getPages(openCardId);
      if (pageIdx >= pages.length) pageIdx = pages.length - 1;
      var back = flipper.querySelector('.docs-face.back');
      back.innerHTML = renderBack(openCardId);
      wireBack();
    }).observe(document.documentElement, { attributes: true, attributeFilter: ['lang'] });
  }

  if (document.readyState === 'loading') {
    document.addEventListener('DOMContentLoaded', init);
  } else {
    init();
  }
})();

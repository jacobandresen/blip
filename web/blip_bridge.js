// Wires Rust (wasm) FFI imports to JS callbacks defined in shell.js.
// Loaded BEFORE mq_js_bundle.js so `miniquad_add_plugin` is available.

// ---------------------------------------------------------------------------
// Supabase hi-score store — replace these two values after creating a project
// ---------------------------------------------------------------------------
const SUPABASE_URL     = 'https://zramohohqnhmfzhlpvok.supabase.co';
const SUPABASE_ANON_KEY = 'sb_publishable_rc_Dp-WhasQoj7gisVbhCg_1XGspuJe';

// Must match GAME_* constants in crates/blip/src/web.rs
const GAME_NAMES = ['bouncer', 'serpent', 'galactic_defender', 'canaris'];
const hiScoreCache = [0, 0, 0, 0];
let lastFetch = 0;

async function fetchHiScores() {
    lastFetch = Date.now();
    try {
        const res = await fetch(
            `${SUPABASE_URL}/rest/v1/hi_scores?select=game,score`,
            { headers: { apikey: SUPABASE_ANON_KEY, Authorization: `Bearer ${SUPABASE_ANON_KEY}` } }
        );
        if (res.ok) {
            const rows = await res.json();
            for (const row of rows) {
                const idx = GAME_NAMES.indexOf(row.game);
                if (idx >= 0) hiScoreCache[idx] = row.score;
            }
        }
    } catch (_) {}
}

fetchHiScores();

// ---- Top-10 leaderboard -----------------------------------------------

const lastGameOverScore = [-1, -1, -1, -1, -1]; // debounce: only check once per game-over

async function checkTop10(game_id, score) {
    if (score <= 0) return;
    const game = GAME_NAMES[game_id];
    try {
        const res = await fetch(
            `${SUPABASE_URL}/rest/v1/scores?game=eq.${game}&select=score&order=score.desc&limit=10`,
            { headers: { apikey: SUPABASE_ANON_KEY, Authorization: `Bearer ${SUPABASE_ANON_KEY}` } }
        );
        if (!res.ok) return;
        const rows = await res.json();
        const qualifies = rows.length < 10 || score > rows[rows.length - 1].score;
        if (qualifies) showInitialsOverlay(game, score);
    } catch (_) {}
}

function showInitialsOverlay(game, score) {
    const overlay = document.getElementById('initials-overlay');
    const form    = document.getElementById('initials-form');
    const scoreEl = document.getElementById('initials-score-val');
    const input   = document.getElementById('initials-input');
    const btn     = document.getElementById('initials-submit');
    if (!overlay) return;

    // reset to form view
    form.style.display = '';
    document.getElementById('initials-leaderboard').style.display = 'none';

    scoreEl.textContent = 'SCORE ' + score;
    input.value = '';
    overlay.classList.add('visible');
    // Desktop: focus so the user can type immediately.
    // iOS: keyboard only opens on a direct user tap — no-op but harmless.
    setTimeout(() => input.focus(), 100);

    let submitted = false;
    async function submit() {
        const initials = input.value.trim().toUpperCase();
        if (!initials || submitted) return;
        submitted = true;
        btn.disabled = true;

        await fetch(`${SUPABASE_URL}/rest/v1/rpc/submit_score`, {
            method: 'POST',
            headers: {
                apikey: SUPABASE_ANON_KEY,
                Authorization: `Bearer ${SUPABASE_ANON_KEY}`,
                'Content-Type': 'application/json',
            },
            body: JSON.stringify({ p_game: game, p_initials: initials, p_score: score }),
        }).catch(() => {});

        try {
            const res = await fetch(
                `${SUPABASE_URL}/rest/v1/scores?game=eq.${game}&select=initials,score&order=score.desc&limit=10`,
                { headers: { apikey: SUPABASE_ANON_KEY, Authorization: `Bearer ${SUPABASE_ANON_KEY}` } }
            );
            const rows = await res.json();
            showLeaderboard(game, rows, initials, score);
        } catch (_) {
            overlay.classList.remove('visible');
        }
        btn.disabled = false;
    }

    btn.onclick = submit;
    input.onkeydown = (e) => { if (e.key === 'Enter') submit(); };
    // Auto-submit after 3 characters so the OK button needn't be tapped on mobile.
    input.oninput = () => { if (input.value.trim().length === 3) setTimeout(submit, 120); };
}

function showLeaderboard(game, rows, newInitials, newScore) {
    const form = document.getElementById('initials-form');
    const lb   = document.getElementById('initials-leaderboard');
    const title = document.getElementById('lb-title');
    const list  = document.getElementById('lb-list');
    const close = document.getElementById('lb-close');

    title.textContent = game.replace(/_/g, ' ').toUpperCase();
    list.innerHTML = rows.map((r, i) => {
        const isNew = r.initials === newInitials && r.score === newScore;
        return `<li class="${isNew ? 'lb-new' : ''}">` +
            `<span class="lb-rank">${i + 1}</span>` +
            `<span class="lb-initials">${r.initials}</span>` +
            `<span class="lb-score">${r.score.toLocaleString()}</span>` +
            `</li>`;
    }).join('');

    form.style.display = 'none';
    lb.style.display = 'flex';

    close.onclick = () => document.getElementById('initials-overlay').classList.remove('visible');
}

register_plugin = function (importObject) {
    importObject.env.blip_spend_coin = function () {
        if (typeof window.blipSpendCoin === 'function') {
            window.blipSpendCoin();
        }
    };
    importObject.env.blip_set_mode = function (mode) {
        if (typeof window.blipSetMode === 'function') {
            window.blipSetMode(mode);
        }
    };
    importObject.env.blip_load_hi_score = function (game_id) {
        if (Date.now() - lastFetch > 60000) fetchHiScores();
        return hiScoreCache[game_id] || 0;
    };
    importObject.env.blip_game_over = function (game_id, score) {
        if (score === lastGameOverScore[game_id]) return;
        lastGameOverScore[game_id] = score;
        checkTop10(game_id, score);
    };
    importObject.env.blip_save_hi_score = function (game_id, score) {
        if (game_id < 0 || game_id >= GAME_NAMES.length) return;
        if (score <= hiScoreCache[game_id]) return;
        hiScoreCache[game_id] = score;
        fetch(`${SUPABASE_URL}/rest/v1/rpc/set_hi_score`, {
            method: 'POST',
            headers: {
                apikey: SUPABASE_ANON_KEY,
                Authorization: `Bearer ${SUPABASE_ANON_KEY}`,
                'Content-Type': 'application/json',
            },
            body: JSON.stringify({ p_game: GAME_NAMES[game_id], p_score: score }),
        }).catch(() => {});
    };
};

miniquad_add_plugin({ register_plugin: register_plugin });

if (new URLSearchParams(location.search).has('__blip_test')) {
    window.__blip_test = {
        triggerGameOver: (gameId, score) => checkTop10(gameId, score),
    };
}

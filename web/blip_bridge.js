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

(async function fetchHiScores() {
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
})();

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
        return hiScoreCache[game_id] || 0;
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

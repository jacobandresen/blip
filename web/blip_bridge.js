// Wires Rust (wasm) FFI imports to JS callbacks defined in shell.js.
// Loaded BEFORE mq_js_bundle.js so `miniquad_add_plugin` is available.

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
    importObject.env.blip_paddles = function (left, right) {
        if (typeof window.blipPaddles === 'function') {
            window.blipPaddles(left, right);
        }
    };
    importObject.env.blip_game_over = function (score) {
        if (typeof window.blipGameOver === 'function') {
            window.blipGameOver(score);
        }
    };
    importObject.env.blip_high_score = function () {
        if (typeof window.blipHighScore === 'function') {
            return window.blipHighScore() | 0;
        }
        return 0;
    };
    // Rust hands us a scratch buffer (ptr + capacity) in wasm memory; we
    // write the record holder's handle as UTF-8 and return the byte count.
    importObject.env.blip_high_name = function (ptr, cap) {
        cap = cap | 0;
        if (cap <= 0 || typeof window.blipHighName !== 'function') return 0;
        var name = window.blipHighName();
        if (!name) return 0;
        var bytes = new TextEncoder().encode(String(name));
        var n = Math.min(bytes.length, cap);
        new Uint8Array(wasm_memory.buffer, ptr, n).set(bytes.subarray(0, n));
        return n;
    };
    // Two-device multiplayer (docs/multiplayer.md, web/blip_net.js).
    importObject.env.blip_net_role = function () {
        if (typeof window.blipNetRole === 'function') {
            return window.blipNetRole() | 0;
        }
        return 0;
    };
    // Rust hands us a buffer it already wrote `len` bytes into (its own
    // packed state, see crates/rally/src/net.rs) — copy them OUT of wasm
    // memory before handing off to blipNetSend, since that may hold onto
    // them past this call and wasm memory can move/grow on the next
    // allocation.
    importObject.env.blip_net_send = function (ptr, len) {
        len = len | 0;
        if (len <= 0 || typeof window.blipNetSend !== 'function') return;
        var bytes = new Uint8Array(wasm_memory.buffer, ptr, len).slice();
        window.blipNetSend(bytes);
    };
    // Mirror of blip_high_name's pattern: Rust hands us a scratch buffer,
    // we write the latest inbound state packet (if any) into it.
    importObject.env.blip_net_poll = function (ptr, cap) {
        cap = cap | 0;
        if (cap <= 0 || typeof window.blipNetPoll !== 'function') return 0;
        var data = window.blipNetPoll();
        if (!data || !data.length) return 0;
        var n = Math.min(data.length, cap);
        new Uint8Array(wasm_memory.buffer, ptr, n).set(data.subarray(0, n));
        return n;
    };
};

miniquad_add_plugin({ register_plugin: register_plugin });

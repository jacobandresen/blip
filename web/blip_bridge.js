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
    // Touch play (shell.js): is a finger down in this slot, and where, as a
    // fraction of the canvas (axis 0 = x, 1 = y).
    importObject.env.blip_touch_down = function (slot) {
        return typeof window.blipTouchDown === 'function' ? window.blipTouchDown(slot) | 0 : 0;
    };
    importObject.env.blip_touch_pos = function (slot, axis) {
        return typeof window.blipTouchPos === 'function' ? +window.blipTouchPos(slot, axis) : 0;
    };
    importObject.env.blip_picture = function (w, h) {
        window.blipPicture = { w: w, h: h };
        window.dispatchEvent(new Event('blip-picture'));
    };
    // What the player holds: 0 a keyboard, 1 the deck's pad or stick, 2 the
    // touch surface. A title screen words its prompts by it.
    importObject.env.blip_controls = function () {
        var root = document.documentElement;
        if (root.hasAttribute('data-touch')) return 2;
        return root.hasAttribute('data-has-touch') ? 1 : 0;
    };
    importObject.env.blip_haptic = function () {
        if (typeof window.blipHaptic === 'function') window.blipHaptic();
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
};

miniquad_add_plugin({ register_plugin: register_plugin });

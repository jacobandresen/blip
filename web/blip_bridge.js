// Wires the imported `blip_spend_coin` symbol from Rust (wasm) to the
// page's `window.blipSpendCoin` implementation defined in shell.js.
//
// Loaded BEFORE mq_js_bundle.js so `miniquad_add_plugin` is available.

register_plugin = function (importObject) {
    importObject.env.blip_spend_coin = function () {
        if (typeof window.blipSpendCoin === 'function') {
            window.blipSpendCoin();
        }
    };
};

miniquad_add_plugin({ register_plugin: register_plugin });

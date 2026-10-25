var CACHE = "blip-v780";

var ASSETS = [
  "/blip/",
  "/blip/index.html",
  "/blip/home.html",
  "/blip/home.css",
  "/blip/home.js",
  "/blip/blip-computer-icon.svg",
  "/blip/blip-basic-cover.svg",
  "/blip/basic.html",
  "/blip/basic-book.html",
  "/blip/book.css",
  "/blip/book.js",
  "/blip/basic-book.css",
  "/blip/basic-book.js",
  "/blip/basic-runtime.html",
  "/blip/basic-runtime.wasm",
  "/blip/basic.css",
  "/blip/paint.css",
  "/blip/tune.css",
  "/blip/basic.js",
  "/blip/basic-language.js",
  "/blip/paint.js",
  "/blip/tune.js",
  "/blip/ROM/TUNES/manifest.json",
  "/blip/ROM/TUNES/BLUE_INTERMISSION.XM",
  "/blip/ROM/TUNES/ATTRIBUTION.txt",
  "/blip/TAPE/BOUNCER.BLIP",
  "/blip/TAPE/BRAWLER.BLIP",
  "/blip/TAPE/BUBBLER.BLIP",
  "/blip/TAPE/ADDER.BLIP",
  "/blip/TAPE/DEFENDER.BLIP",
  "/blip/TAPE/METEORS.BLIP",
  "/blip/TAPE/RALLY.BLIP",
  "/blip/TAPE/SERPENT.BLIP",
  "/blip/TAPE/RAIDER.BLIP",
  "/blip/TAPE/manifest.json",
  "/blip/blip-tape-load.mp3",
  "/blip/blip-wordmark.svg",
  "/blip/guide.html",
  "/blip/guide.css",
  "/blip/guide.js",
  "/blip/guide-manifest.json",
  "/blip/guide-icon-180.png",
  "/blip/guide-icon-192.png",
  "/blip/guide-icon-512.png",
  "/blip/guide-icon-maskable-512.png",
  "/blip/about.html",
  "/blip/api.html",
  "/blip/history.html",
  "/blip/controls.html",
  "/blip/manifest.json",
  "/blip/kiosk.css",
  "/blip/arcade-room.css",
  "/blip/wear.css",
  "/blip/manual.css",
  "/blip/blip_core.js",
  "/blip/kiosk.js",
  "/blip/aisle.js",
  "/blip/deck.js",
  "/blip/machine-deck.html",
  "/blip/marquee.css",
  "/blip/press-start-2p.woff2",
  "/blip/caveat.woff2",
  "/blip/mrs-saint-delafield.woff2",
  "/blip/shell.css",
  "/blip/shell.html",
  "/blip/shell.js",
  "/blip/blip_controller.js",
  "/blip/blip_config.js",
  "/blip/blip_scores.js",
  "/blip/vendor/supabase.js",
  "/blip/howler.min.js",
  "/blip/mq_js_bundle.js",
  "/blip/howler_audio_plugin.js",
  "/blip/blip_bridge.js",
  "/blip/favicon.ico",
  "/blip/favicon.svg",
  "/blip/favicon-32.png",
  "/blip/icon.svg",
  "/blip/icon-192.png",
  "/blip/icon-512.png",
  "/blip/apple-touch-icon.png",
  "/blip/operator-jacob.jpg",
  "/blip/operator-bjorn.jpg",
  "/blip/claude-avatar.svg",
  "/blip/codex-avatar.svg",
  "/blip/copilot-avatar.svg",
  "/blip/history-icon.svg",
  "/blip/controls-icon.svg",
  "/blip/bouncer/index.html",
  "/blip/bouncer/index.wasm",
  "/blip/bouncer/manifest.json",
  "/blip/bouncer/icon-180.png",
  "/blip/bouncer/icon-192.png",
  "/blip/bouncer/icon-512.png",
  "/blip/bouncer/icon-maskable-512.png",
  "/blip/bouncer/screenshot.png",
  "/blip/bouncer/attract.webp",
  "/blip/bouncer/attract.png",
  "/blip/defender/index.html",
  "/blip/defender/index.wasm",
  "/blip/defender/manifest.json",
  "/blip/defender/icon-180.png",
  "/blip/defender/icon-192.png",
  "/blip/defender/icon-512.png",
  "/blip/defender/icon-maskable-512.png",
  "/blip/defender/screenshot.png",
  "/blip/defender/attract.webp",
  "/blip/defender/attract.png",
  "/blip/rally/index.html",
  "/blip/rally/index.wasm",
  "/blip/rally/manifest.json",
  "/blip/rally/icon-180.png",
  "/blip/rally/icon-192.png",
  "/blip/rally/icon-512.png",
  "/blip/rally/icon-maskable-512.png",
  "/blip/rally/screenshot.png",
  "/blip/rally/attract.webp",
  "/blip/rally/attract.png",
  "/blip/serpent/index.html",
  "/blip/serpent/index.wasm",
  "/blip/serpent/manifest.json",
  "/blip/serpent/icon-180.png",
  "/blip/serpent/icon-192.png",
  "/blip/serpent/icon-512.png",
  "/blip/serpent/icon-maskable-512.png",
  "/blip/serpent/screenshot.png",
  "/blip/serpent/attract.webp",
  "/blip/serpent/attract.png",
  "/blip/meteors/index.html",
  "/blip/meteors/index.wasm",
  "/blip/meteors/manifest.json",
  "/blip/meteors/icon-180.png",
  "/blip/meteors/icon-192.png",
  "/blip/meteors/icon-512.png",
  "/blip/meteors/icon-maskable-512.png",
  "/blip/meteors/screenshot.png",
  "/blip/meteors/attract.webp",
  "/blip/meteors/attract.png",
  "/blip/brawler/index.html",
  "/blip/brawler/index.wasm",
  "/blip/brawler/manifest.json",
  "/blip/brawler/icon-180.png",
  "/blip/brawler/icon-192.png",
  "/blip/brawler/icon-512.png",
  "/blip/brawler/icon-maskable-512.png",
  "/blip/brawler/screenshot.png",
  "/blip/brawler/attract.webp",
  "/blip/brawler/attract.png",
  "/blip/brawler/card.png",
  "/blip/bubbler/index.html",
  "/blip/bubbler/index.wasm",
  "/blip/bubbler/manifest.json",
  "/blip/bubbler/icon-180.png",
  "/blip/bubbler/icon-192.png",
  "/blip/bubbler/icon-512.png",
  "/blip/bubbler/icon-maskable-512.png",
  "/blip/bubbler/screenshot.png",
  "/blip/bubbler/attract.webp",
  "/blip/bubbler/attract.png",
  "/blip/raider/index.html",
  "/blip/raider/index.wasm",
  "/blip/raider/manifest.json",
  "/blip/raider/icon-180.png",
  "/blip/raider/icon-192.png",
  "/blip/raider/icon-512.png",
  "/blip/raider/icon-maskable-512.png",
  "/blip/raider/screenshot.png",
  "/blip/raider/attract.webp",
  "/blip/raider/attract.png",
  "/blip/adder/index.html",
  "/blip/adder/index.wasm",
  "/blip/adder/manifest.json",
  "/blip/adder/icon-180.png",
  "/blip/adder/icon-192.png",
  "/blip/adder/icon-512.png",
  "/blip/adder/icon-maskable-512.png",
  "/blip/adder/screenshot.png",
  "/blip/adder/attract.webp",
  "/blip/adder/attract.png",
];

self.addEventListener("install", function (e) {
  e.waitUntil(
    caches.open(CACHE).then(function (cache) {
      // cache: 'reload' skips the HTTP cache (Pages serves max-age=600), or a
      // new version can install the previous version's files.
      return cache.addAll(ASSETS.map(function (u) {
        return new Request(u, { cache: "reload" });
      }));
    }),
  );
  self.skipWaiting();
});

self.addEventListener("activate", function (e) {
  e.waitUntil(
    caches.keys().then(function (keys) {
      return Promise.all(
        keys
          .filter(function (k) {
            return k !== CACHE;
          })
          .map(function (k) {
            return caches.delete(k);
          }),
      );
    }),
  );
  self.clients.claim();
});

// Cache-first for same-origin assets only. Cross-origin requests bypass the
// cache entirely so they always hit the network.
self.addEventListener("fetch", function (e) {
  if (e.request.method !== "GET") return;
  var url = new URL(e.request.url);
  if (url.origin !== self.location.origin) return;
  // PR previews live inside this worker's scope; cache-first would pin a
  // preview to its first version.
  if (url.pathname.indexOf("/pr-preview/") !== -1) return;
  var key = url.origin + url.pathname;
  e.respondWith(
    caches.open(CACHE).then(function (cache) {
      return cache.match(key).then(function (cached) {
        if (cached) return cached;
        return fetch(e.request).then(function (response) {
          if (response.ok) cache.put(key, response.clone());
          return response;
        });
      });
    }),
  );
});

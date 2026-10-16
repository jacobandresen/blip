var CACHE = "blip-v447";

var ASSETS = [
  "/blip/",
  "/blip/index.html",
  "/blip/home.html",
  "/blip/home.css",
  "/blip/home.js",
  "/blip/about.html",
  "/blip/api.html",
  "/blip/history.html",
  "/blip/controls.html",
  "/blip/manifest.json",
  "/blip/kiosk.css",
  "/blip/wear.css",
  "/blip/manual.css",
  "/blip/blip_core.js",
  "/blip/kiosk.js",
  "/blip/deck.js",
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
  "/blip/galactic_defender/index.html",
  "/blip/galactic_defender/index.wasm",
  "/blip/galactic_defender/manifest.json",
  "/blip/galactic_defender/icon-180.png",
  "/blip/galactic_defender/icon-192.png",
  "/blip/galactic_defender/icon-512.png",
  "/blip/galactic_defender/icon-maskable-512.png",
  "/blip/galactic_defender/screenshot.png",
  "/blip/rally/index.html",
  "/blip/rally/index.wasm",
  "/blip/rally/manifest.json",
  "/blip/rally/icon-180.png",
  "/blip/rally/icon-192.png",
  "/blip/rally/icon-512.png",
  "/blip/rally/icon-maskable-512.png",
  "/blip/rally/screenshot.png",
  "/blip/serpent/index.html",
  "/blip/serpent/index.wasm",
  "/blip/serpent/manifest.json",
  "/blip/serpent/icon-180.png",
  "/blip/serpent/icon-192.png",
  "/blip/serpent/icon-512.png",
  "/blip/serpent/icon-maskable-512.png",
  "/blip/serpent/screenshot.png",
  "/blip/meteors/index.html",
  "/blip/meteors/index.wasm",
  "/blip/meteors/manifest.json",
  "/blip/meteors/icon-180.png",
  "/blip/meteors/icon-192.png",
  "/blip/meteors/icon-512.png",
  "/blip/meteors/icon-maskable-512.png",
  "/blip/meteors/screenshot.png",
  "/blip/brawler/index.html",
  "/blip/brawler/index.wasm",
  "/blip/brawler/manifest.json",
  "/blip/brawler/icon-180.png",
  "/blip/brawler/icon-192.png",
  "/blip/brawler/icon-512.png",
  "/blip/brawler/icon-maskable-512.png",
  "/blip/brawler/screenshot.png",
  "/blip/brawler/card.png",
  "/blip/bubbler/index.html",
  "/blip/bubbler/index.wasm",
  "/blip/bubbler/manifest.json",
  "/blip/bubbler/icon-180.png",
  "/blip/bubbler/icon-192.png",
  "/blip/bubbler/icon-512.png",
  "/blip/bubbler/icon-maskable-512.png",
  "/blip/bubbler/screenshot.png",
  "/blip/sky_raider/index.html",
  "/blip/sky_raider/index.wasm",
  "/blip/sky_raider/manifest.json",
  "/blip/sky_raider/icon-180.png",
  "/blip/sky_raider/icon-192.png",
  "/blip/sky_raider/icon-512.png",
  "/blip/sky_raider/icon-maskable-512.png",
  "/blip/sky_raider/screenshot.png",
  "/blip/adder/index.html",
  "/blip/adder/index.wasm",
  "/blip/adder/manifest.json",
  "/blip/adder/icon-180.png",
  "/blip/adder/icon-192.png",
  "/blip/adder/icon-512.png",
  "/blip/adder/icon-maskable-512.png",
  "/blip/adder/screenshot.png",
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

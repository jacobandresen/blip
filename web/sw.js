var CACHE = 'blip-v6';

var ASSETS = [
  '/blip/',
  '/blip/index.html',
  '/blip/about.html',
  '/blip/history.html',
  '/blip/manifest.json',
  '/blip/kiosk.css',
  '/blip/kiosk.js',
  '/blip/shell.css',
  '/blip/shell.html',
  '/blip/shell.js',
  '/blip/howler.min.js',
  '/blip/mq_js_bundle.js',
  '/blip/howler_audio_plugin.js',
  '/blip/blip_bridge.js',
  '/blip/favicon.ico',
  '/blip/favicon-32.png',
  '/blip/icon.svg',
  '/blip/icon-192.png',
  '/blip/icon-512.png',
  '/blip/apple-touch-icon.png',
  '/blip/claude-avatar.svg',
  '/blip/copilot-avatar.svg',
  '/blip/history-icon.svg',
  '/blip/bouncer/index.html',
  '/blip/bouncer/index.wasm',
  '/blip/bouncer/screenshot.png',
  '/blip/canaris/index.html',
  '/blip/canaris/index.wasm',
  '/blip/canaris/screenshot.png',
  '/blip/galactic_defender/index.html',
  '/blip/galactic_defender/index.wasm',
  '/blip/galactic_defender/screenshot.png',
  '/blip/rally/index.html',
  '/blip/rally/index.wasm',
  '/blip/rally/screenshot.png',
  '/blip/serpent/index.html',
  '/blip/serpent/index.wasm',
  '/blip/serpent/screenshot.png',
  '/blip/rivet/index.html',
  '/blip/rivet/index.wasm',
  '/blip/rivet/screenshot.png',
];

self.addEventListener('install', function(e) {
  e.waitUntil(
    caches.open(CACHE).then(function(cache) { return cache.addAll(ASSETS); })
  );
  self.skipWaiting();
});

self.addEventListener('activate', function(e) {
  e.waitUntil(
    caches.keys().then(function(keys) {
      return Promise.all(
        keys.filter(function(k) { return k !== CACHE; })
            .map(function(k) { return caches.delete(k); })
      );
    })
  );
  self.clients.claim();
});

// Cache-first for same-origin assets only. Cross-origin requests (e.g. Supabase API)
// bypass the cache entirely so they always hit the network.
self.addEventListener('fetch', function(e) {
  if (e.request.method !== 'GET') return;
  var url = new URL(e.request.url);
  if (url.origin !== self.location.origin) return;
  var key = url.origin + url.pathname;
  e.respondWith(
    caches.open(CACHE).then(function(cache) {
      return cache.match(key).then(function(cached) {
        if (cached) return cached;
        return fetch(e.request).then(function(response) {
          if (response.ok) cache.put(key, response.clone());
          return response;
        });
      });
    })
  );
});

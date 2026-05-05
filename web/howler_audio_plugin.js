// Replaces macroquad's built-in audio plugin with Howler.js for iOS reliability.
// Loaded after mq_js_bundle.js so this register_plugin call overwrites the macroquad_audio one.

if (typeof Howler !== 'undefined') {
  Howler.autoSuspend = false;
  // Disable Howler's built-in unlock: it calls unload() when ctx.sampleRate !== 44100,
  // which destroys all WASM-loaded sounds permanently (iOS often reports 48000 Hz).
  // kiosk.js handles unlock via a permanent gesture listener instead.
  Howler.autoUnlock = false;
}

(function () {
  'use strict';

  var sounds = new Map();
  var playbacks = new Map();
  var nextSoundId = 1;
  var nextPlaybackId = 1;

  function audio_init() {
    if (typeof Howler !== 'undefined') {
      Howler.autoSuspend = false;
      Howler.autoUnlock = false;
    }
  }

  function audio_add_buffer(ptr, len) {
    var id = nextSoundId++;
    var bytes = wasm_memory.buffer.slice(ptr, ptr + len);
    var blob = new Blob([bytes], { type: 'audio/wav' });
    var url = URL.createObjectURL(blob);
    var howl = new Howl({
      src: [url],
      format: ['wav'],
      preload: true,
      html5: false,
      onload: function () { URL.revokeObjectURL(url); },
      onloaderror: function (_, err) {
        console.error('howler: failed to load sound:', err);
        URL.revokeObjectURL(url);
      }
    });
    sounds.set(id, howl);
    return id;
  }

  function audio_source_is_loaded(id) {
    var h = sounds.get(id);
    return h ? h.state() === 'loaded' : false;
  }

  function audio_play_buffer(soundId, volume, loop) {
    var howl = sounds.get(soundId);
    if (!howl) return 0;
    var pbId = nextPlaybackId++;
    var hid = howl.play();
    howl.volume(volume, hid);
    howl.loop(!!loop, hid);
    playbacks.set(pbId, { howl: howl, hid: hid });
    return pbId;
  }

  function audio_source_set_volume(soundId, volume) {
    var h = sounds.get(soundId);
    if (h) h.volume(volume);
  }

  function audio_source_stop(soundId) {
    var h = sounds.get(soundId);
    if (h) h.stop();
  }

  function audio_source_delete(soundId) {
    var h = sounds.get(soundId);
    if (!h) return;
    h.stop();
    h.unload();
    sounds.delete(soundId);
  }

  function audio_playback_stop(pbId) {
    var pb = playbacks.get(pbId);
    if (!pb) return;
    pb.howl.stop(pb.hid);
    playbacks.delete(pbId);
  }

  function audio_playback_set_volume(pbId, volume) {
    var pb = playbacks.get(pbId);
    if (pb) pb.howl.volume(volume, pb.hid);
  }

  miniquad_add_plugin({
    register_plugin: function (importObject) {
      importObject.env.audio_init              = audio_init;
      importObject.env.audio_add_buffer        = audio_add_buffer;
      importObject.env.audio_play_buffer       = audio_play_buffer;
      importObject.env.audio_source_is_loaded  = audio_source_is_loaded;
      importObject.env.audio_source_set_volume = audio_source_set_volume;
      importObject.env.audio_source_stop       = audio_source_stop;
      importObject.env.audio_source_delete     = audio_source_delete;
      importObject.env.audio_playback_stop     = audio_playback_stop;
      importObject.env.audio_playback_set_volume = audio_playback_set_volume;
    }
  });
}());

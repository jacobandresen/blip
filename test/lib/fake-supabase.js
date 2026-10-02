/* A stand-in for vendor/supabase.js, served by test/highscore-entry.mjs: the
 * calls blip_scores.js makes, answered the way supabase/migrations answer
 * them, with the tables in localStorage ('fake-sb') so a test can read what
 * was written. One anonymous user, 'u1'. */
(function () {
  'use strict';
  var KEY = 'fake-sb';
  var GAMES = ['serpent', 'bouncer', 'galactic_defender', 'meteors', 'sky_raider', 'bubbler', 'brawler'];

  function load() {
    var d = {};
    try { d = JSON.parse(localStorage.getItem(KEY)) || {}; } catch (e) {}
    d.players = d.players || { rival: 'RIVAL' };
    d.scores = d.scores || {};
    d.calls = d.calls || [];
    return d;
  }
  function save(d) { localStorage.setItem(KEY, JSON.stringify(d)); }

  // leaderboard_top: best first, rank from 1; every game has one other row.
  function board(d, game) {
    var of = d.scores[game] || {};
    var rows = Object.keys(of).map(function (uid) { return { handle: d.players[uid], score: of[uid] }; });
    rows.push({ handle: 'RIVAL', score: 5 });
    rows.sort(function (a, b) { return b.score - a.score; });
    rows.forEach(function (r, i) { r.rank = i + 1; });
    return rows;
  }

  function submitScore(d, a) {
    if (!d.session) return { ok: false, reason: 'no_session' };
    if (GAMES.indexOf(a.p_game) < 0) return { ok: false, reason: 'bad_game' };
    if (typeof a.p_score !== 'number' || a.p_score % 1 || a.p_score < 0 || a.p_score > 10000000) {
      return { ok: false, reason: 'bad_score' };
    }
    var handle = d.players.u1;
    if (!handle) return { ok: false, reason: 'needs_handle' };
    var of = d.scores[a.p_game] = d.scores[a.p_game] || {};
    if (!(of.u1 >= a.p_score)) of.u1 = a.p_score;
    var rows = board(d, a.p_game);
    var mine = rows.filter(function (r) { return r.handle === handle; })[0];
    return { ok: true, rank: mine.rank, best: of.u1, board: rows.slice(0, 10) };
  }

  function claimHandle(d, a) {
    if (!d.session) return { ok: false, reason: 'no_session' };
    var h = String(a.p_handle || '').trim();
    if (!/^[A-Za-z0-9 _-]{2,14}$/.test(h)) return { ok: false, reason: 'bad_handle' };
    if (h.toUpperCase() === 'RIVAL') return { ok: false, reason: 'taken' };
    d.players.u1 = h;
    return { ok: true, handle: h, recovery_code: 'ABCD-EFGH-JKLM' };
  }

  window.supabase = {
    createClient: function () {
      return {
        auth: {
          getSession: function () { return Promise.resolve({ data: { session: load().session || null } }); },
          signInAnonymously: function () {
            var d = load();
            d.session = { user: { id: 'u1' } };
            save(d);
            return Promise.resolve({ data: { session: d.session }, error: null });
          }
        },
        rpc: function (name, args) {
          var d = load(), out = { ok: false, reason: 'unknown' };
          if (name === 'submit_score') out = submitScore(d, args);
          if (name === 'claim_handle') out = claimHandle(d, args);
          d.calls.push({ name: name, args: args, out: out.ok ? 'ok' : out.reason });
          save(d);
          return Promise.resolve({ data: out, error: null });
        },
        from: function () {
          var q = {};
          var api = {
            select: function () { return api; },
            eq: function (k, v) { q[k] = v; return api; },
            order: function () { return api; },
            limit: function (n) { q.n = n; return api; },
            then: function (ok, fail) {
              return Promise.resolve({ data: board(load(), q.game).slice(0, q.n || 10) }).then(ok, fail);
            }
          };
          return api;
        }
      };
    }
  };
}());

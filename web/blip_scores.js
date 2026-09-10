/* BLIP high-score leaderboard client — talks to Supabase (see
 * docs/highscores.md). Loaded on every game page after vendor/supabase.js
 * and blip_config.js.
 *
 * Identity is a Supabase ANONYMOUS user: no login, no password. The player
 * picks a handle once (stored server-side, UNIQUE) and a JWT in
 * localStorage is what proves it's them. On first claim they get a one-time
 * RECOVERY CODE — the only way to move the handle to another device or
 * recover it after clearing the browser. Everything degrades to a
 * local-only best if the backend is absent or unreachable — the games
 * never block on the network.
 */
(function () {
  'use strict';

  var CFG = window.BLIP_SUPABASE || {};
  var ENABLED = !!(CFG.url && CFG.anonKey && window.supabase);

  var HANDLE_RE = /^[A-Za-z0-9 _-]{2,14}$/;
  var LS_HANDLE = 'blip-handle';
  var AUTH_KEY = 'blip-sb-auth';

  var sb = null;
  if (ENABLED) {
    sb = window.supabase.createClient(CFG.url, CFG.anonKey, {
      auth: {
        storageKey: AUTH_KEY,
        persistSession: true,
        autoRefreshToken: true,
        detectSessionInUrl: false
      }
    });
  }

  function lsGet(k) { try { return localStorage.getItem(k); } catch (e) { return null; } }
  function lsSet(k, v) { try { localStorage.setItem(k, v); } catch (e) {} }

  function localBest(game) { return parseInt(lsGet('blip-best-' + game) || '0', 10) || 0; }
  function setLocalBest(game, score) {
    score = parseInt(score, 10) || 0;
    if (score > localBest(game)) lsSet('blip-best-' + game, String(score));
  }

  // Cache the current leaderboard #1 (handle + score) so the game's title /
  // game-over screens can show "HI  NAME  SCORE" without a round trip.
  // `top` is a leaderboard_top row, or a submit_score board[0] entry.
  function cacheTop(game, top) {
    if (!top || top.score == null) return;
    lsSet('blip-top-' + game, JSON.stringify({ handle: top.handle || '', score: top.score | 0 }));
  }
  function cacheBoardTop(game, data) {
    if (data && data.board && data.board.length) cacheTop(game, data.board[0]);
  }

  // ---- session -----------------------------------------------------------
  var sessionPromise = null;
  function ensureSession() {
    if (!ENABLED) return Promise.resolve(null);
    if (sessionPromise) return sessionPromise;
    sessionPromise = sb.auth.getSession().then(function (r) {
      if (r.data && r.data.session) return r.data.session;
      return sb.auth.signInAnonymously().then(function (r2) {
        if (r2.error) throw r2.error;
        return r2.data.session;
      });
    }).catch(function (e) {
      sessionPromise = null;          // let a later game-over retry
      throw e;
    });
    return sessionPromise;
  }

  // ---- RPC --------------------------------------------------------------
  function rpc(name, args) {
    return sb.rpc(name, args || {}).then(function (r) {
      if (r.error) throw r.error;
      return r.data;
    });
  }
  function submitScore(game, score) { return rpc('submit_score', { p_game: game, p_score: score }); }
  function claimHandle(handle) { return rpc('claim_handle', { p_handle: handle }); }
  function restoreHandle(code) { return rpc('restore_handle', { p_code: code }); }
  function rotateCode() { return rpc('new_recovery_code', {}); }

  function el(tag, cls, parent) {
    var n = document.createElement(tag);
    if (cls) n.className = cls;
    if (parent) parent.appendChild(n);
    return n;
  }
  function modal() {
    var wrap = el('div', 'blip-hs-modal', document.body);
    var panel = el('div', 'blip-hs-panel', wrap);
    return { wrap: wrap, panel: panel };
  }

  // ---- UI: recovery code ---------------------------------------------
  function showRecoveryCode(handle, code, opts) {
    opts = opts || {};
    var m = modal();
    el('div', 'blip-hs-title', m.panel).textContent =
      opts.rotated ? 'YOUR NEW CODE' : 'SAVE THIS CODE';
    el('div', 'blip-hs-sub', m.panel).textContent =
      'the only way to get ' + (handle || 'this name') +
      ' back on another device or after clearing this browser' +
      (opts.rotated ? ' — the old code no longer works' : '');
    el('div', 'blip-hs-code', m.panel).textContent = code;
    var msg = el('div', 'blip-hs-err', m.panel);
    var row = el('div', 'blip-hs-row', m.panel);

    var copy = el('button', 'blip-hs-btn', row);
    copy.type = 'button'; copy.textContent = 'COPY';
    copy.addEventListener('click', function () {
      function ok() { msg.style.color = '#6dffa8'; msg.textContent = 'copied'; }
      try {
        if (navigator.clipboard && navigator.clipboard.writeText) {
          navigator.clipboard.writeText(code).then(ok, function () { msg.textContent = code; });
        } else { msg.textContent = code; }
      } catch (e) { msg.textContent = code; }
    });

    var done = el('button', 'blip-hs-btn ghost', row);
    done.type = 'button'; done.textContent = 'I SAVED IT';
    done.addEventListener('click', function () {
      m.wrap.remove();
      if (opts.onClose) opts.onClose();
    });
  }

  // ---- UI: handle prompt --------------------------------------------
  function promptHandle(message) {
    return new Promise(function (resolve) {
      var m = modal();
      el('div', 'blip-hs-title', m.panel).textContent = 'ENTER YOUR NAME';
      el('div', 'blip-hs-sub', m.panel).textContent =
        message || 'claimed once — nobody else can use it';
      var input = document.createElement('input');
      input.className = 'blip-hs-input';
      input.maxLength = 14;
      input.autocapitalize = 'characters';
      input.spellcheck = false;
      input.setAttribute('aria-label', 'Your leaderboard name');
      input.value = (lsGet(LS_HANDLE) || '').toUpperCase();
      m.panel.appendChild(input);
      var err = el('div', 'blip-hs-err', m.panel);
      var row = el('div', 'blip-hs-row', m.panel);
      var ok = el('button', 'blip-hs-btn', row); ok.type = 'button'; ok.textContent = 'OK';
      var skip = el('button', 'blip-hs-btn ghost', row); skip.type = 'button'; skip.textContent = 'SKIP';

      // Shown only after a name comes back TAKEN — the taker might be you,
      // on a new device or a cleared browser. Enter the recovery code to
      // take the name (and its scores) back.
      var mineRow = el('div', 'blip-hs-row', m.panel);
      mineRow.style.display = 'none';   // .blip-hs-row's flex would beat [hidden]
      var mineBtn = el('button', 'blip-hs-btn ghost', mineRow);
      mineBtn.type = 'button'; mineBtn.textContent = 'THAT NAME IS MINE';

      function close(val) { m.wrap.remove(); resolve(val); }
      function submit() {
        var v = input.value.trim();
        if (!HANDLE_RE.test(v)) { err.textContent = '2-14 letters, digits, space, - or _'; return; }
        ok.disabled = true; err.textContent = '';
        claimHandle(v).then(function (res) {
          if (res && res.ok) {
            lsSet(LS_HANDLE, res.handle);
            if (res.recovery_code) {
              m.wrap.remove();
              showRecoveryCode(res.handle, res.recovery_code,
                { onClose: function () { resolve(res.handle); } });
              return;
            }
            close(res.handle);
          } else if (res && res.reason === 'taken') {
            err.textContent = v.toUpperCase() + ' is taken';
            mineRow.style.display = '';
            ok.disabled = false;
          } else {
            err.textContent = 'could not save that name'; ok.disabled = false;
          }
        }).catch(function () { err.textContent = 'network error'; ok.disabled = false; });
      }
      ok.addEventListener('click', submit);
      skip.addEventListener('click', function () { close(null); });
      // open the code prompt on top; if it restores a name, we're done,
      // otherwise it just closes and this prompt is still here to try another
      mineBtn.addEventListener('click', function () {
        promptRestore(input.value.trim()).then(function (h) { if (h) close(h); });
      });
      input.addEventListener('keydown', function (e) {
        e.stopPropagation();
        if (e.key === 'Enter') submit();
        if (e.key === 'Escape') close(null);
      });
      setTimeout(function () { try { input.focus(); } catch (e) {} }, 30);
    });
  }

  // ---- UI: restore a name from a code ------------------------------
  // `forName` (optional) is a handle to name in the copy — passed when the
  // player hit "that's my name" on a TAKEN result.
  function promptRestore(forName) {
    forName = (forName || '').trim();
    return new Promise(function (resolve) {
      var m = modal();
      el('div', 'blip-hs-title', m.panel).textContent =
        forName ? 'RESTORE ' + forName.toUpperCase() : 'RESTORE A NAME';
      el('div', 'blip-hs-sub', m.panel).textContent =
        forName
          ? 'enter the recovery code you saved for ' + forName.toUpperCase()
          : 'enter the recovery code you saved when you claimed the name';
      var input = document.createElement('input');
      input.className = 'blip-hs-input';
      input.maxLength = 20;
      input.autocapitalize = 'characters';
      input.spellcheck = false;
      input.placeholder = 'XXXX-XXXX-XXXX';
      input.setAttribute('aria-label', 'Recovery code');
      m.panel.appendChild(input);
      var err = el('div', 'blip-hs-err', m.panel);
      var row = el('div', 'blip-hs-row', m.panel);
      var ok = el('button', 'blip-hs-btn', row); ok.type = 'button'; ok.textContent = 'RESTORE';
      var cancel = el('button', 'blip-hs-btn ghost', row); cancel.type = 'button'; cancel.textContent = 'CANCEL';

      function close(v) { m.wrap.remove(); resolve(v); }
      function submit() {
        var v = input.value.trim();
        if (v.replace(/[^0-9a-z]/gi, '').length < 8) { err.textContent = 'check the code'; return; }
        ok.disabled = true; err.textContent = '';
        ensureSession().then(function () { return restoreHandle(v); }).then(function (res) {
          if (res && res.ok) { lsSet(LS_HANDLE, res.handle); close(res.handle); }
          else if (res && res.reason === 'bad_code') { err.textContent = "that code doesn't match"; ok.disabled = false; }
          else if (res && res.reason === 'rate_limited') { err.textContent = 'too many tries — wait a bit'; ok.disabled = false; }
          else { err.textContent = 'could not restore'; ok.disabled = false; }
        }).catch(function () { err.textContent = 'network error'; ok.disabled = false; });
      }
      ok.addEventListener('click', submit);
      cancel.addEventListener('click', function () { close(null); });
      input.addEventListener('keydown', function (e) {
        e.stopPropagation();
        if (e.key === 'Enter') submit();
        if (e.key === 'Escape') close(null);
      });
      setTimeout(function () { try { input.focus(); } catch (e) {} }, 30);
    });
  }

  // Rotate + show the calling player's recovery code.
  function showMyCode() {
    if (!ENABLED) return;
    ensureSession().then(function () { return rotateCode(); }).then(function (res) {
      if (res && res.ok && res.recovery_code) {
        showRecoveryCode(lsGet(LS_HANDLE) || '', res.recovery_code, { rotated: true });
      } else if (res && res.reason === 'needs_handle') {
        promptHandle('claim a name first, then you get a code');
      }
    }).catch(function () {});
  }

  // ---- UI: the board -------------------------------------------------
  var boardEl = null;
  var boardKeys = null;
  function dismissBoard() {
    if (boardEl) { boardEl.remove(); boardEl = null; }
    if (boardKeys) {
      window.removeEventListener('keydown', boardKeys, true);
      window.removeEventListener('pointerdown', boardKeys, true);
      boardKeys = null;
    }
  }
  function refreshBoard(game) {
    return sb.from('leaderboard_top').select('handle,score,rank')
      .eq('game', game).order('rank').limit(10)
      .then(function (r) {
        cacheBoardTop(game, { board: r.data || [] });
        showBoard(game, { board: r.data || [] });
      });
  }
  function showBoard(game, data) {
    dismissBoard();
    var mine = (lsGet(LS_HANDLE) || '').toLowerCase();
    boardEl = el('div', 'blip-hs-modal', document.body);
    var panel = el('div', 'blip-hs-panel wide', boardEl);
    var g = (window.blipGameFromPath && blipGameFromPath(location.pathname));
    el('div', 'blip-hs-title', panel).textContent = (g ? g.name : 'HIGH') + ' SCORES';

    var list = el('ol', 'blip-hs-list', panel);
    var board = (data && data.board) || [];
    if (!board.length) el('div', 'blip-hs-sub', panel).textContent = 'be the first';
    board.forEach(function (r) {
      var li = el('li', 'blip-hs-item', list);
      if (String(r.handle).toLowerCase() === mine) li.classList.add('me');
      el('span', 'hs-rank', li).textContent = r.rank;
      el('span', 'hs-name', li).textContent = r.handle;
      el('span', 'hs-score', li).textContent = Number(r.score).toLocaleString();
    });

    if (data && data.rank) {
      el('div', 'blip-hs-sub', panel).textContent =
        data.rank <= 10 ? "you're #" + data.rank : 'your rank: #' + data.rank;
    }

    var row = el('div', 'blip-hs-row', panel);
    var cont = el('button', 'blip-hs-btn', row);
    cont.type = 'button'; cont.textContent = 'CONTINUE';
    cont.addEventListener('click', dismissBoard);

    if (ENABLED) {
      var row2 = el('div', 'blip-hs-row', panel);
      if (lsGet(LS_HANDLE)) {
        var codeBtn = el('button', 'blip-hs-btn ghost', row2);
        codeBtn.type = 'button'; codeBtn.textContent = 'MY CODE';
        codeBtn.title = 'show the recovery code for your name';
        codeBtn.addEventListener('click', function () { dismissBoard(); showMyCode(); });
      }
      var restoreBtn = el('button', 'blip-hs-btn ghost', row2);
      restoreBtn.type = 'button'; restoreBtn.textContent = 'RESTORE';
      restoreBtn.title = 'get a name back with its recovery code';
      restoreBtn.addEventListener('click', function () {
        dismissBoard();
        promptRestore().then(function (h) { if (h) refreshBoard(game); });
      });
    }

    // first tap / key anywhere dismisses the board (and is swallowed, so
    // it doesn't also poke the WASM game-over screen behind it)
    boardKeys = function (e) {
      if (boardEl && boardEl.contains(e.target)) return;
      e.preventDefault(); e.stopPropagation();
      dismissBoard();
    };
    setTimeout(function () {
      if (!boardEl) return;
      window.addEventListener('keydown', boardKeys, true);
      window.addEventListener('pointerdown', boardKeys, true);
    }, 400);
  }

  // ---- entry point from shell.js ------------------------------------
  var lastGame = null, lastScore = -1;
  function onGameOver(game, score) {
    score = Math.max(0, Math.floor(Number(score) || 0));
    setLocalBest(game, score);
    if (!ENABLED || !game) return Promise.resolve();
    // de-dupe: WASM may re-enter Over on a redraw
    if (game === lastGame && score === lastScore) return Promise.resolve();
    lastGame = game; lastScore = score;

    return ensureSession()
      .then(function () { return submitScore(game, score); })
      .then(function (res) {
        if (res && res.ok) {
          if (res.best != null) setLocalBest(game, res.best);
          cacheBoardTop(game, res);
          showBoard(game, res);
          return;
        }
        if (res && res.reason === 'needs_handle') {
          return promptHandle('new high score — pick a name for the board')
            .then(function (h) {
              if (!h) return;                       // skipped
              return submitScore(game, score).then(function (r2) {
                if (r2 && r2.ok) {
                  if (r2.best != null) setLocalBest(game, r2.best);
                  cacheBoardTop(game, r2);
                  showBoard(game, r2);
                }
              });
            });
        }
        // rate_limited / bad_* — just show the current board, best effort
        return refreshBoard(game);
      })
      .catch(function (e) {
        if (window.console) console.warn('[blip-scores]', e && e.message || e);
      });
  }

  window.blipScores = {
    enabled: ENABLED,
    onGameOver: onGameOver,
    promptHandle: promptHandle,
    promptRestore: promptRestore,
    showMyCode: showMyCode,
    dismissBoard: dismissBoard
  };

  // On a game page, refresh the cached #1 once at load so the title screen
  // shows the current record straight away (one small indexed GET).
  (function primeTop() {
    if (!ENABLED) return;
    var g = (window.blipGameFromPath && blipGameFromPath(location.pathname));
    if (!g) return;
    sb.from('leaderboard_top').select('handle,score')
      .eq('game', g.slug).order('rank').limit(1)
      .then(function (r) { if (r.data && r.data[0]) cacheTop(g.slug, r.data[0]); })
      .catch(function () {});
  }());
}());

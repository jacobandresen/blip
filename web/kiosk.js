var MAX_COINS = 5;

function getCoins() {
  try {
    var n = parseInt(sessionStorage.getItem('blip-coins') || '0', 10);
    return isNaN(n) ? 0 : Math.min(Math.max(n, 0), MAX_COINS);
  } catch (e) { return 0; }
}

function saveCoins(n) {
  try { sessionStorage.setItem('blip-coins', n); } catch (e) {}
}

function updateCoinsHud() {
  var n = getCoins(), icons = '';
  for (var i = 0; i < MAX_COINS; i++) icons += i < n ? '●' : '○';
  var text = 'COINS ' + icons;
  document.querySelectorAll('[data-coins-hud]').forEach(function (el) {
    el.textContent = text;
  });
}

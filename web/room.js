/* Things in the arcade room, standing in the gaps between the machines in a machine's own CSS 3D space, so the browser
 * draws them from wherever the player stands. What is round (pots, a bush) or flat (a sword, a shield) is one upright
 * picture facing the player, painted in detail (a plane facing the player keeps its shape in perspective, and the
 * machines never turn); what has corners (the sword's stand) is boxes with wood on their faces. A machine's frame: x
 * from its left edge (it is 300 wide; the gap to the next is x 300 to 480), y up from the floor, z toward the player
 * from its front (it is 200 deep; the wall is at -215). No DOM until build(), so test/room.test.mjs checks the parts. */
var BlipRoom = (function () {
  var GAP = [300, 480], WALL = -215;

  // ---- Textures: SVG, lit from the upper left like the room.
  function svg(w, h, body) {
    return 'url("data:image/svg+xml,' + encodeURIComponent("<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 " + w + ' ' + h +
      "' width='" + w + "' height='" + h + "'>" + body + '</svg>') + '") center/100% 100% no-repeat';
  }
  // A grain of noise over a shape: clay, enamel, wood. freq: its fineness across and down; tint: its colour; alpha.
  function grain(id, freq, tint, alpha, seed) {
    return "<filter id='" + id + "' x='0' y='0' width='100%' height='100%'><feTurbulence type='fractalNoise' baseFrequency='" + freq +
      "' numOctaves='3' seed='" + (seed || 2) + "'/><feColorMatrix values='0 0 0 0 " + tint[0] + ' 0 0 0 0 ' + tint[1] + ' 0 0 0 0 ' + tint[2] +
      ' 0 0 0 ' + alpha + " 0'/><feComposite in2='SourceGraphic' operator='in'/></filter>";
  }
  // A seeded random, so the leaves fall the same way every time.
  function seeded(n) { return function () { n = (n * 16807) % 2147483647; return n / 2147483647; }; }

  // A clay pot: a full body on a foot, a short neck, a flared lip round a dark mouth, a band scored round it. tall: a
  // jar, slimmer and higher shouldered.
  function potArt(tall) {
    var body = tall
      ? 'M42 128C28 126 20 112 18 90C16 66 24 50 38 44L42 34L35 30C35 25 45 22 60 22C75 22 85 25 85 30L78 34L82 44C96 50 104 66 102 90C100 112 92 126 78 128Z'
      : 'M40 128C22 126 8 110 8 86C8 64 22 50 36 44L40 34L34 30C34 25 44 22 60 22C76 22 86 25 86 30L80 34L84 44C98 50 112 64 112 86C112 110 98 126 80 128Z';
    return svg(120, 130,
      "<defs><radialGradient id='b' cx='.36' cy='.42' r='.7'><stop offset='0' stop-color='#f0a070'/><stop offset='.4' stop-color='#cf7040'/>" +
      "<stop offset='.8' stop-color='#94441f'/><stop offset='1' stop-color='#5c2612'/></radialGradient>" +
      "<radialGradient id='m' cx='.5' cy='.35' r='.6'><stop offset='0' stop-color='#1e0c06'/><stop offset='1' stop-color='#4a2010'/></radialGradient>" +
      "<linearGradient id='l' x2='0' y2='1'><stop offset='0' stop-color='#f2a676'/><stop offset='1' stop-color='#a4502a'/></linearGradient>" +
      grain('g', '.9', [.25, .12, .05], .5, 4) + "<filter id='s'><feGaussianBlur stdDeviation='3'/></filter></defs>" +
      "<path d='" + body + "' fill='url(#b)'/><path d='" + body + "' fill='#000' filter='url(#g)'/>" +
      // the scored band, and its light lower edge
      "<path d='M14 74Q60 86 106 74' fill='none' stroke='#6a2c14' stroke-width='2'/><path d='M14 76.5Q60 88.5 106 76.5' fill='none' stroke='#f0a070' stroke-opacity='.5' stroke-width='1'/>" +
      "<path d='M12 94Q60 106 108 94' fill='none' stroke='#6a2c14' stroke-width='1.5'/>" +
      // the lip: its top face, the mouth inside it, and its rounded front
      "<ellipse cx='60' cy='27' rx='27' ry='7' fill='url(#l)'/><ellipse cx='60' cy='27.5' rx='20' ry='4.6' fill='url(#m)'/>" +
      "<path d='M33 29C34 33 44 36 60 36C76 36 86 33 87 29' fill='none' stroke='#7a3418' stroke-width='1.5'/>" +
      // a glaze highlight and the shade where the body meets the floor
      "<ellipse cx='38' cy='70' rx='9' ry='16' fill='#fff' opacity='.35' filter='url(#s)' transform='rotate(18 38 70)'/>" +
      "<ellipse cx='34' cy='66' rx='2.5' ry='6' fill='#fff' opacity='.5' transform='rotate(18 34 66)'/>" +
      // the honey floor's light thrown back up on its lower side
      "<path d='M22 112C34 124 86 124 98 112C92 122 76 127 60 127C44 127 28 122 22 112Z' fill='#f0b860' opacity='.28'/>" +
      "<path d='M30 124C46 129 74 129 90 124' fill='none' stroke='#3a1408' stroke-width='3' opacity='.6'/>");
  }

  // A cream china pot with a blue band, and a bush of leaves, each shaded by where it sits in the bush.
  function plantArt() {
    var rnd = seeded(7), leaves = '';
    var list = [];
    for (var i = 0; i < 170; i++) {
      var a = rnd() * Math.PI * 2, r = Math.sqrt(rnd()) * 46, x = 70 + Math.cos(a) * r * 1.15, y = 60 + Math.sin(a) * r * .95 - (1 - r / 46) * 8;
      list.push([x, y, rnd() * 360, 15 + rnd() * 9, rnd()]);
    }
    // back to front: the lower leaves behind
    list.sort(function (p, q) { return p[1] - q[1]; });
    list.forEach(function (l) {
      var lit = Math.max(0, Math.min(1, .55 - (l[0] - 70) / 110 - (l[1] - 62) / 90 + (l[4] - .5) * .3));
      var c = 'hsl(' + (100 + l[4] * 22) + ' 52% ' + (24 + lit * 34) + '%)', vein = 'hsl(100 40% ' + (16 + lit * 22) + '%)';
      leaves += "<g transform='translate(" + l[0].toFixed(1) + ' ' + l[1].toFixed(1) + ') rotate(' + l[2].toFixed(0) + ")'>" +
        "<path d='M0 0C" + (l[3] * .35) + ' ' + (-l[3] * .32) + ' ' + (l[3] * .75) + ' ' + (-l[3] * .28) + ' ' + l[3] + ' 0C' + (l[3] * .75) + ' ' + (l[3] * .28) + ' ' + (l[3] * .35) + ' ' + (l[3] * .32) +
        " 0 0Z' fill='" + c + "'/><path d='M" + (l[3] * .45) + ' ' + (-l[3] * .16) + 'C' + (l[3] * .7) + ' ' + (-l[3] * .2) + ' ' + (l[3] * .9) + ' ' + (-l[3] * .08) + ' ' + l[3] + " 0' fill='none' stroke='#e8ffd0' stroke-width='1' opacity='" + (.15 + lit * .4).toFixed(2) + "'/>" +
        "<path d='M1 0L" + (l[3] * .85) + " 0' stroke='" + vein + "' stroke-width='.7'/></g>";
    });
    return svg(140, 170,
      "<defs><linearGradient id='p' x2='1'><stop offset='0' stop-color='#c8bca8'/><stop offset='.25' stop-color='#fbf5e8'/><stop offset='.6' stop-color='#e6dcc8'/>" +
      "<stop offset='1' stop-color='#9a8e7c'/></linearGradient><linearGradient id='u' x2='1'><stop offset='0' stop-color='#2a5a96'/><stop offset='.3' stop-color='#5a9ae0'/>" +
      "<stop offset='1' stop-color='#1e3e6e'/></linearGradient><radialGradient id='d' cx='.5' cy='.5' r='.5'><stop offset='0' stop-color='#3a2416'/><stop offset='1' stop-color='#5a3a24'/></radialGradient>" +
      "<filter id='s'><feGaussianBlur stdDeviation='2.5'/></filter></defs>" +
      // the pot: a tapered body, the band, the rolled rim, the soil
      "<path d='M42 118L48 166H92L98 118Z' fill='url(#p)'/><path d='M43.5 130H96.5L95.6 138H44.4Z' fill='url(#u)'/>" +
      "<rect x='38' y='110' width='64' height='10' rx='4' fill='url(#p)'/><ellipse cx='70' cy='111' rx='31' ry='4' fill='url(#d)'/>" +
      "<path d='M52 122L55 162' stroke='#fff' stroke-width='3' opacity='.55' filter='url(#s)'/>" +
      "<ellipse cx='70' cy='108' rx='40' ry='8' fill='#1c3a14' opacity='.5' filter='url(#s)'/>" + leaves + flowers());
  }

  // White flowers among the leaves, five petals round a yellow eye.
  function flowers() {
    var out = '';
    [[50, 40, 0], [86, 34, 30], [70, 58, 60], [36, 66, 15], [100, 62, 45], [62, 22, 80]].forEach(function (f) {
      out += "<g transform='translate(" + f[0] + ' ' + f[1] + ') rotate(' + f[2] + ")'>";
      for (var k = 0; k < 5; k++) out += "<ellipse cx='0' cy='-5' rx='3.4' ry='5.6' fill='#fffaf0' stroke='#d8cfc0' stroke-width='.5' transform='rotate(" + k * 72 + ")'/>";
      out += "<circle r='2.4' fill='#f8c840'/><circle r='1' cx='-.6' cy='-.6' fill='#fff2a0'/></g>";
    });
    return out;
  }
  // A heater shield of our own: blue enamel in a bevelled gold rim with rivets, a raised red heart, a few scratches.
  function shieldArt() {
    var outer = 'M3 3H107V58C107 98 82 126 55 147C28 126 3 98 3 58Z', inner = 'M13 13H97V58C97 92 76 115 55 133C34 115 13 92 13 58Z';
    var rivets = '';
    [[8, 8], [55, 8], [102, 8], [8, 40], [102, 40], [10, 76], [100, 76], [22, 108], [88, 108], [55, 140]].forEach(function (p) {
      rivets += "<circle cx='" + p[0] + "' cy='" + p[1] + "' r='2.4' fill='#7a5418'/><circle cx='" + (p[0] - .6) + "' cy='" + (p[1] - .6) + "' r='1.6' fill='#ffe9a8'/>";
    });
    var heart = 'M55 96C38 83 30 74 30 63C30 54 37 48 44 48C49 48 53 51 55 55C57 51 61 48 66 48C73 48 80 54 80 63C80 74 72 83 55 96Z';
    return svg(110, 150,
      "<defs><linearGradient id='r' x1='0' y1='0' x2='1' y2='1'><stop offset='0' stop-color='#fff0b0'/><stop offset='.3' stop-color='#e2a83a'/>" +
      "<stop offset='.65' stop-color='#9a6a20'/><stop offset='1' stop-color='#e6b450'/></linearGradient>" +
      "<radialGradient id='e' cx='.35' cy='.25' r='.9'><stop offset='0' stop-color='#5a8ee0'/><stop offset='.55' stop-color='#2c5cb0'/><stop offset='1' stop-color='#163672'/></radialGradient>" +
      "<radialGradient id='h' cx='.38' cy='.3' r='.75'><stop offset='0' stop-color='#ff8890'/><stop offset='.5' stop-color='#d8303e'/><stop offset='1' stop-color='#801424'/></radialGradient>" +
      grain('n', '1.4', [.05, .08, .2], .35, 9) + "</defs>" +
      "<path d='" + outer + "' fill='url(#r)' stroke='#5a3a10' stroke-width='1.5'/>" +
      "<path d='" + inner + "' fill='#5a3a10' transform='translate(1 1)'/><path d='" + inner + "' fill='url(#e)'/><path d='" + inner + "' fill='#000' filter='url(#n)'/>" +
      // a sheen across the enamel, the heart raised on it, and wear
      "<path d='M18 18H50C40 50 30 80 22 92C16 80 14 70 14 58Z' fill='#fff' opacity='.12'/>" +
      "<path d='" + heart + "' fill='#4a0c14' transform='translate(1.5 2)'/><path d='" + heart + "' fill='url(#h)' stroke='#e2a83a' stroke-width='2.2'/>" +
      "<path d='M41 55C44 52 48 52 50 55' fill='none' stroke='#fff' stroke-width='2' opacity='.7' stroke-linecap='round'/>" +
      "<path d='M70 24L82 36M30 104L37 99M76 92L80 86' stroke='#c8dcff' stroke-width='.7' opacity='.5'/>" + rivets);
  }

  // A sword of our own: a steel blade with bevelled edges and a fuller down its middle, a gold guard with round ends,
  // a grip wound in blue leather, a gold pommel with a red stone.
  function swordArt() {
    var wrap = '';
    for (var y = 52; y < 104; y += 5) wrap += "<path d='M33 " + y + 'L47 ' + (y - 3) + "' stroke='#16204a' stroke-width='1.6'/><path d='M33 " + (y + 1.6) + 'L47 ' + (y - 1.4) + "' stroke='#6a80d0' stroke-width='.6' opacity='.7'/>";
    return svg(80, 330,
      // the steel mirrors the room: its dark bands are the wood walls, its light ones the lamp
      "<defs><linearGradient id='s' x2='1'><stop offset='0' stop-color='#6e5a4a'/><stop offset='.2' stop-color='#eef0f2'/><stop offset='.47' stop-color='#a89888'/>" +
      "<stop offset='.53' stop-color='#f8f8f6'/><stop offset='.8' stop-color='#b8b0a6'/><stop offset='1' stop-color='#5e4a3c'/></linearGradient>" +
      "<linearGradient id='v' x2='0' y2='1'><stop offset='0' stop-color='#fff' stop-opacity='.35'/><stop offset='.35' stop-color='#fff' stop-opacity='0'/>" +
      "<stop offset='.7' stop-color='#000' stop-opacity='.12'/><stop offset='1' stop-color='#fff' stop-opacity='.2'/></linearGradient>" +
      "<linearGradient id='g' x2='0' y2='1'><stop offset='0' stop-color='#fff0b0'/><stop offset='.45' stop-color='#e2a83a'/><stop offset='1' stop-color='#8a5a18'/></linearGradient>" +
      "<radialGradient id='o' cx='.35' cy='.35' r='.7'><stop offset='0' stop-color='#fff2c0'/><stop offset='.45' stop-color='#e2a83a'/><stop offset='1' stop-color='#7a4c14'/></radialGradient>" +
      "<radialGradient id='j' cx='.35' cy='.3' r='.7'><stop offset='0' stop-color='#ffc0c8'/><stop offset='.4' stop-color='#e0283c'/><stop offset='1' stop-color='#6a0a18'/></radialGradient>" +
      "<linearGradient id='w' x2='1'><stop offset='0' stop-color='#1e2a60'/><stop offset='.35' stop-color='#4a5cb0'/><stop offset='1' stop-color='#141c44'/></linearGradient></defs>" +
      // the blade: its two bevels, the flat between, the fuller, and the light running down it
      "<path d='M33 116H47V312L40 330L33 312Z' fill='url(#s)'/><path d='M33 116H47V312L40 330L33 312Z' fill='url(#v)'/>" +
      // the room in it: the lamp's light high up, the honey floor low down
      "<path d='M33 230H47V312L40 330L33 312Z' fill='#d89a50' opacity='.16'/><path d='M34 128H36V200H34Z' fill='#fff' opacity='.55'/>" +
      "<path d='M36 122H44V290L40 296L36 290Z' fill='#5e687a'/><path d='M43 122H43.6V290H43Z' fill='#fff' opacity='.6'/>" +
      "<path d='M33 116V312L40 330' fill='none' stroke='#4a5262' stroke-width='.6'/>" +
      // its maker's name inlaid in gold down the fuller, read from the hilt: large and outlined, to read across the room
      "<text transform='translate(36.2 128) rotate(90)' font-family='Georgia,serif' font-weight='bold' font-style='italic' font-size='10' letter-spacing='1.4' fill='#ffd860' stroke='#3a2408' stroke-width='.9' paint-order='stroke'>Jacob A.</text>" +
      // the guard, its round ends and its stone
      "<rect x='4' y='106' width='72' height='9' rx='3' fill='url(#g)' stroke='#6a4410' stroke-width='.8'/>" +
      "<path d='M12 108.5H30M50 108.5H68M12 112.5H30M50 112.5H68' stroke='#8a5a18' stroke-width='.6'/>" +
      "<circle cx='5' cy='110.5' r='6' fill='url(#o)'/><circle cx='75' cy='110.5' r='6' fill='url(#o)'/>" +
      "<circle cx='3.5' cy='108.5' r='1.6' fill='#fff' opacity='.8'/><circle cx='73.5' cy='108.5' r='1.6' fill='#fff' opacity='.8'/>" +
      "<path d='M36 106L40 100L44 106Z' fill='url(#g)'/><circle cx='40' cy='111' r='3' fill='url(#j)'/>" +
      // the grip, wound, and the pommel
      "<rect x='33' y='44' width='14' height='62' rx='3' fill='url(#w)'/>" + wrap +
      "<rect x='31' y='42' width='18' height='5' rx='2' fill='url(#g)'/><rect x='31' y='102' width='18' height='5' rx='2' fill='url(#g)'/>" +
      "<circle cx='40' cy='30' r='14' fill='url(#o)' stroke='#6a4410' stroke-width='.8'/><circle cx='40' cy='30' r='5.5' fill='url(#j)'/>" +
      "<circle cx='38' cy='28' r='1.6' fill='#fff' opacity='.9'/><path d='M30 22C33 18 38 17 42 18' fill='none' stroke='#fff' stroke-width='1.6' opacity='.6' stroke-linecap='round'/>");
  }

  // A small brass plaque on a box's front, engraved with a line of text.
  function plaque(w, h) {
    var pw = 34, ph = h - 8, px = (w - pw) / 2, py = 4;
    return "<defs><linearGradient id='q' x1='0' y1='0' x2='1' y2='1'><stop offset='0' stop-color='#fff0b8'/><stop offset='.45' stop-color='#d8a040'/>" +
      "<stop offset='1' stop-color='#8a5a18'/></linearGradient></defs><rect x='" + px + "' y='" + py + "' width='" + pw + "' height='" + ph + "' rx='1.5' fill='url(#q)' stroke='#5a3a10' stroke-width='.6'/>" +
      "<path d='M" + (px + 5) + ' ' + (py + ph / 2) + 'H' + (px + pw - 5) + "' stroke='#5a3a10' stroke-width='1.4' stroke-dasharray='2 1 4 1 3 1'/>" +
      "<circle cx='" + (px + 2.5) + "' cy='" + (py + ph / 2) + "' r='.9' fill='#5a3a10'/><circle cx='" + (px + pw - 2.5) + "' cy='" + (py + ph / 2) + "' r='.9' fill='#5a3a10'/>";
  }
  // Oak: a grain of long fibres along the board, a little darker toward its ends.
  function wood(w, h, along, extra) {
    var freq = along === 'up' ? '.5 .015' : '.015 .5';
    return svg(w, h, "<defs>" + grain('o', freq, [.16, .07, .02], .75, 5) +
      "<linearGradient id='t' x2='" + (along === 'up' ? 0 : 1) + "' y2='" + (along === 'up' ? 1 : 0) + "'><stop offset='0' stop-color='#000' stop-opacity='.18'/>" +
      "<stop offset='.5' stop-color='#000' stop-opacity='0'/><stop offset='1' stop-color='#000' stop-opacity='.22'/></linearGradient></defs>" +
      "<rect width='" + w + "' height='" + h + "' fill='#a8703e'/><rect width='" + w + "' height='" + h + "' fill='#000' filter='url(#o)'/>" +
      "<rect width='" + w + "' height='" + h + "' fill='url(#t)'/>" + (extra || ''));
  }

  var THINGS = {
    // Two clay pots against the wall.
    pots: function () {
      return [{ kind: 'picture', x: 360, y: 0, z: -175, w: 70, h: 92, art: potArt(true) },
        { kind: 'picture', x: 428, y: 0, z: -150, w: 60, h: 65, art: potArt() }];
    },
    // A leafy bush in a china pot.
    plant: function () { return [{ kind: 'picture', x: 390, y: 0, z: -165, w: 112, h: 136, art: plantArt() }]; },
    // A shield of our own on its point, its top resting on the wall: leant back LEAN, its point stands H sin LEAN out.
    shield: function () {
      var h = 150, lean = 12;
      return [{ kind: 'picture', x: 390, y: 0, z: WALL + h * Math.sin(lean * Math.PI / 180) + 3, w: 110, h: h, lean: lean, edge: 3, art: shieldArt() }];
    },
    // A sword of our own upright in an oak stand: its point in a slot in the base, a post beside it, and an arm from the
    // post holding its grip in a fork.
    sword: function () {
      var x = 382, z = -106, post = x + 36;
      return [
        { kind: 'box', x: x + 14, y: 0, z: z - 4, w: 100, h: 20, d: 62, wood: true, plaque: true },
        { kind: 'box', x: x + 8, y: 20, z: z - 2, w: 70, h: 7, d: 44, wood: true, slot: { x: x, z: z, w: 16, d: 4 } },
        { kind: 'box', x: post, y: 27, z: z - 4, w: 14, h: 231, d: 14, wood: true },
        { kind: 'box', x: (post + x) / 2 + 2, y: 248, z: z - 4, w: post - x - 4, h: 10, d: 10, wood: true },
        { kind: 'box', x: x - 11, y: 248, z: z, w: 5, h: 22, d: 10, wood: true },
        { kind: 'box', x: x + 11, y: 248, z: z, w: 5, h: 22, d: 10, wood: true },
        // the picture's foot is the blade's point, 8 below the plinth's top, in the slot
        { kind: 'picture', x: x, y: 19, z: z, w: 80, h: 330, art: swordArt() }];
    }
  };

  // How far a part reaches across, up and toward the player: [x0, x1], [y0, y1], [z0, z1]. A picture's art is drawn
  // within its frame: the sword's guard, the widest part, is 72 of its 80 across.
  function extent(p) {
    if (p.kind === 'box') return [[p.x - p.w / 2, p.x + p.w / 2], [p.y, p.y + p.h], [p.z - p.d / 2, p.z + p.d / 2]];
    var lean = (p.lean || 0) * Math.PI / 180;
    return [[p.x - p.w / 2, p.x + p.w / 2], [p.y, p.y + p.h * Math.cos(lean)], [p.z - p.h * Math.sin(lean) - (p.edge || 0), p.z]];
  }

  // ---- Drawing.
  function face(doc, parent, w, h, transform, background) {
    var el = doc.createElement('span');
    el.className = 'room-face';
    el.style.cssText = 'width:' + w + 'px;height:' + h + 'px;margin:' + (-h / 2) + 'px 0 0 ' + (-w / 2) + 'px;transform:' + transform + ';background:' + background;
    parent.appendChild(el);
    return el;
  }
  function at(x, y, z) { return 'translate3d(' + x + 'px,' + (-y) + 'px,' + z + 'px)'; }
  // Light on a face from the upper left and in front: a veil of shade over its texture.
  function veil(k) { return k < 1 ? 'linear-gradient(rgb(20 10 4 / ' + (1 - k).toFixed(2) + '),rgb(20 10 4 / ' + (1 - k).toFixed(2) + ')),' : 'linear-gradient(rgb(255 240 210 / ' + (k - 1).toFixed(2) + '),rgb(255 240 210 / ' + (k - 1).toFixed(2) + ')),'; }
  // The shadow a part casts on the wall: the lamp hangs to the left, in front and above, so a part d in front of the
  // wall throws it 0.55 d to the right and 0.45 d down; what falls below the floor is cut off there.
  function cast(doc, g, p, x, y, w, h, background) {
    var d = Math.max(0, p.z - WALL), sx = x + .55 * d, sy = y - .45 * d, below = Math.max(0, -sy);
    if (below >= h) return;
    var el = face(doc, g, w, h, at(sx, sy + h / 2, WALL + 1), background);
    el.style.filter = 'brightness(0) blur(' + (1.5 + d / 40).toFixed(1) + 'px)';
    el.style.opacity = Math.max(.2, .6 - d / 400).toFixed(2);
    if (below) el.style.clipPath = 'inset(0 0 ' + (below / h * 100).toFixed(1) + '% 0)';
  }
  var DRAW = {
    box: function (doc, g, p) {
      var c = at(p.x, p.y + p.h / 2, p.z);
      if (p.h > 40) cast(doc, g, p, p.x, p.y, p.w, p.h, '#000');
      face(doc, g, p.w, p.h, c + ' translateZ(' + p.d / 2 + 'px)', veil(.92) + wood(p.w, p.h, p.h > p.w ? 'up' : 'across', p.plaque ? plaque(p.w, p.h) : ''));
      var side = wood(p.d, p.h, p.h > p.d ? 'up' : 'across');
      face(doc, g, p.d, p.h, c + ' rotateY(-90deg) translateZ(' + p.w / 2 + 'px)', veil(1.06) + side);
      face(doc, g, p.d, p.h, c + ' rotateY(90deg) translateZ(' + p.w / 2 + 'px)', veil(.6) + side);
      // The top, and the slot cut in it for whatever stands there (its place from the top face's back left corner).
      var slot = p.slot ? "<rect x='" + (p.slot.x - p.slot.w / 2 - (p.x - p.w / 2)) + "' y='" + (p.slot.z - p.slot.d / 2 - (p.z - p.d / 2)) + "' width='" + p.slot.w +
        "' height='" + p.slot.d + "' rx='1' fill='#1a0c04'/>" : '';
      var bevel = "<path d='M0 " + p.d + 'H' + p.w + "' stroke='#ffe2b0' stroke-width='2' opacity='.5'/><path d='M1 0V" + p.d + "' stroke='#ffe2b0' stroke-width='1.5' opacity='.35'/>";
      face(doc, g, p.w, p.d, c + ' rotateX(90deg) translateZ(' + p.h / 2 + 'px)', veil(1.12) + wood(p.w, p.d, 'across', slot + bevel));
    },
    picture: function (doc, g, p) {
      var c = at(p.x, p.y, p.z) + ' rotateX(' + (p.lean || 0) + 'deg) translate3d(0,' + (-p.h / 2) + 'px,0)';
      if (!p.lean) cast(doc, g, p, p.x, p.y, p.w, p.h, p.art);
      if (p.edge) face(doc, g, p.w, p.h, c + ' translateZ(' + (-p.edge) + 'px)', p.art).style.filter = 'brightness(.35)';
      // What leans on the wall darkens it behind, toward the lower right as the light falls.
      if (p.lean) face(doc, g, p.w * 1.3, p.h * 1.1, at(p.x + 10, p.h * .5, WALL + 1), 'radial-gradient(closest-side,rgb(20 10 6 / .45),transparent)');
      face(doc, g, p.w, p.h, c, p.art);
    }
  };
  // The thing's parts, and its shadow pooled on the floor under them, a little toward the right as the light falls.
  function build(doc, card, name) {
    var g = doc.createElement('span'), parts = THINGS[name](), x0 = Infinity, x1 = -Infinity, z0 = Infinity, z1 = -Infinity;
    g.className = 'room-thing room-' + name;
    g.setAttribute('aria-hidden', 'true');
    parts.forEach(function (p) {
      var e = extent(p);
      x0 = Math.min(x0, e[0][0]); x1 = Math.max(x1, e[0][1]); z0 = Math.min(z0, e[2][0]); z1 = Math.max(z1, e[2][1]);
    });
    var shadow = face(doc, g, x1 - x0 + 24, Math.max(z1 - z0, 40) + 24, at((x0 + x1) / 2 + 8, .5, (z0 + z1) / 2 - 4) + ' rotateX(90deg)',
      'radial-gradient(closest-side,rgb(20 10 6 / .7),rgb(20 10 6 / .35) 45%,transparent)');
    shadow.style.borderRadius = '50%';
    parts.forEach(function (p) { DRAW[p.kind](doc, g, p); });
    card.appendChild(g);
    return g;
  }
  return { THINGS: THINGS, GAP: GAP, WALL: WALL, extent: extent, build: build };
}());
if (typeof module !== 'undefined') module.exports = BlipRoom;

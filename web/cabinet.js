// A fresh cabinet visit powers down; travelling between its pages keeps credit.
(function(){
  if(!document.getElementById('game-grid')||new URLSearchParams(location.search).has('manual'))return;
  var navigation=performance.getEntriesByType('navigation')[0],internal=false;
  try{var previous=new URL(document.referrer),scope=new URL('.',location.href);internal=previous.origin===scope.origin&&previous.pathname.startsWith(scope.pathname);}catch(e){}
  if((navigation&&navigation.type==='reload')||(!internal&&(!navigation||navigation.type!=='back_forward')))saveCoins(0);
}());
// Free play keeps the cabinet lit with no coin in it.
function blipPowered() { return getCoins()>0||document.documentElement.hasAttribute('data-freeplay'); }
document.documentElement.toggleAttribute('data-credit',blipPowered());
function buildPowerIndicators() {
  if (!document.body || document.querySelector('.blip-power-indicator')) return;
  var lamp=document.createElement('div');
  lamp.className='blip-power-indicator';
  lamp.setAttribute('role','status');
  lamp.setAttribute('aria-live','polite');
  lamp.innerHTML='<span class="power-label">POWER</span><i class="power-bulb" aria-hidden="true"><svg viewBox="0 0 10 16"><path d="M6.2 0 1 9h3.3L3.8 16 9 6.7H5.7z"/></svg></i>';
  document.body.appendChild(lamp);
  function place() {
    var topBar=document.querySelector('.top-marquee-bar, #marquee-bar')||document.querySelector('.blip-logo');
    if(!topBar)return;
    var barBox=topBar.getBoundingClientRect();
    lamp.style.left='';
    lamp.style.right='max('+Math.max(8,innerWidth-barBox.right+8)+'px,calc(env(safe-area-inset-right,0px) + 8px))';
    lamp.style.top=barBox.top+(barBox.height-lamp.offsetHeight)/2+'px';
  }
  function sync() {
    var on=document.documentElement.hasAttribute('data-credit');
    lamp.setAttribute('aria-label','Power '+(on?'on':'off'));
    lamp.title='Power '+(on?'on':'off');
    place();
  }
  sync();
  window.addEventListener('resize',place);
  if(typeof ResizeObserver==='function'){
    var resizeObserver=new ResizeObserver(place);
    var bar=document.querySelector('.top-marquee-bar, #marquee-bar')||document.querySelector('.blip-logo');
    if(bar)resizeObserver.observe(bar);
  }
  new MutationObserver(sync).observe(document.documentElement,{attributes:true,attributeFilter:['data-credit']});
}
if(document.readyState==='loading')document.addEventListener('DOMContentLoaded',buildPowerIndicators,{once:true});
else buildPowerIndicators();
function updateCoinsHud() {
  document.documentElement.toggleAttribute('data-credit',blipPowered());
  blipUpdateCabinetHum();
}

/* ---- Audio ---- */
var _kioskAudioCtx = null;

function getKioskAudio() {
  if (Howler.ctx) {
    if (Howler.ctx.state === 'suspended') Howler.ctx.resume();
    return Howler.ctx;
  }
  if (!_kioskAudioCtx) _kioskAudioCtx = new (window.AudioContext || window.webkitAudioContext)();
  if (_kioskAudioCtx.state === 'suspended') _kioskAudioCtx.resume();
  return _kioskAudioCtx;
}

/* A deck press (blip-deck's click option): a soft plastic clack, its weight by the part. */
function blipDeckClick(kind) {
  var ctx = getKioskAudio(), t = ctx.currentTime;
  var spec = ({
    face:     { hp: 1900, ng: 0.16, nd: 0.020, thk: 128, tg: 0.10, td: 0.045 },
    dpad:     { hp: 1400, ng: 0.11, nd: 0.028, thk: 92,  tg: 0.09, td: 0.055 },
    shoulder: { hp: 1200, ng: 0.13, nd: 0.024, thk: 78,  tg: 0.11, td: 0.060 },
    menu:     { hp: 2600, ng: 0.10, nd: 0.014, thk: 180, tg: 0.05, td: 0.030 }
  })[kind];
  // the contact transient: a very short filtered-noise tick
  var src = blipNoise(ctx, spec.nd, function (t, p) { return 1 - p; });
  var hp = ctx.createBiquadFilter(); hp.type = 'highpass'; hp.frequency.value = spec.hp;
  var ng = ctx.createGain();
  ng.gain.setValueAtTime(spec.ng, t);
  ng.gain.exponentialRampToValueAtTime(0.0006, t + spec.nd);
  src.connect(hp); hp.connect(ng); ng.connect(blipOut(ctx));
  src.start(t); src.stop(t + spec.nd + 0.01);
  // the muted thock of the cap bottoming out
  var osc = ctx.createOscillator(), og = ctx.createGain();
  osc.type = 'sine'; osc.frequency.setValueAtTime(spec.thk, t);
  osc.frequency.exponentialRampToValueAtTime(spec.thk * 0.6, t + spec.td);
  og.gain.setValueAtTime(spec.tg, t);
  og.gain.exponentialRampToValueAtTime(0.0006, t + spec.td);
  osc.connect(og); og.connect(blipOut(ctx));
  osc.start(t); osc.stop(t + spec.td + 0.02);
}

var cabinetHum=null;
function blipUpdateCabinetHum(unlock) {
  var powered=blipPowered()&&!document.hidden;
  if(!powered) {
    if(cabinetHum&&!cabinetHum.offTimer) {
      var fading=cabinetHum;
      fading.output.gain.setTargetAtTime(0,fading.ctx.currentTime,.22);
      fading.offTimer=setTimeout(function(){
        fading.sources.forEach(function(source){source.stop();source.disconnect();});
        fading.nodes.forEach(function(node){node.disconnect();});
        if(cabinetHum===fading)cabinetHum=null;
      },1400);
    }
    return;
  }
  try {
    var existing=_kioskAudioCtx||Howler.ctx;
    if(!existing&&!unlock)return;
    var ctx=getKioskAudio();
    if(ctx.state!=='running') {ctx.resume().then(function(){blipUpdateCabinetHum();}).catch(function(){});return;}
    if(!cabinetHum) {
      var output=ctx.createGain(),filter=ctx.createBiquadFilter(),sources=[],nodes=[output,filter];
      filter.type='lowpass';filter.frequency.value=430;filter.Q.value=.65;
      output.gain.value=0;filter.connect(output);output.connect(blipOut(ctx));
      [[60,'sawtooth',.6],[120,'sine',.28],[93,'triangle',.12]].forEach(function(voice){
        var oscillator=ctx.createOscillator(),level=ctx.createGain();
        oscillator.type=voice[1];oscillator.frequency.value=voice[0];level.gain.value=voice[2];
        oscillator.connect(level);level.connect(filter);oscillator.start();sources.push(oscillator);nodes.push(level);
      });
      var buffer=ctx.createBuffer(1,ctx.sampleRate*2,ctx.sampleRate),data=buffer.getChannelData(0),smoothed=0;
      for(var i=0;i<data.length;i++){smoothed=.97*smoothed+.03*(Math.random()*2-1);data[i]=smoothed;}
      var fan=ctx.createBufferSource(),air=ctx.createGain();fan.buffer=buffer;fan.loop=true;air.gain.value=.55;
      fan.connect(air);air.connect(filter);fan.start();sources.push(fan);nodes.push(air);
      cabinetHum={ctx:ctx,output:output,sources:sources,nodes:nodes,offTimer:null};
    }
    clearTimeout(cabinetHum.offTimer);cabinetHum.offTimer=null;
    cabinetHum.output.gain.setTargetAtTime(.09,cabinetHum.ctx.currentTime,.35);
  }catch(e){}
}
['pointerdown','keydown','touchstart'].forEach(function(event){
  document.addEventListener(event,function(){blipUpdateCabinetHum(true);},{capture:true,passive:true});
});
document.addEventListener('visibilitychange',function(){blipUpdateCabinetHum();});
window.addEventListener('pagehide',function(){
  if(!cabinetHum)return;
  clearTimeout(cabinetHum.offTimer);
  cabinetHum.sources.forEach(function(source){source.stop();source.disconnect();});
  cabinetHum.nodes.forEach(function(node){node.disconnect();});cabinetHum=null;
});

/* ---- The arcade's room ---- Each machine within two and a half of you plays a short attract jingle in its own
 * instrument, louder and more central the nearer it is; under them a murmur and now and then a coin. Like the hum it
 * starts on the first key or tap and stops with the page hidden; blipOut() carries the mute. */
// wave, root (Hz), beat (s), notes (semitones from the root, null a rest), decay (s), level.
var BLIP_ATTRACT = {
  rally:    {wave:'square',   root:440, beat:.18, decay:.06, level:.35, notes:[0,null,null,0,null,null,7]},
  bouncer:  {wave:'triangle', root:523, beat:.12, decay:.18, level:.8,  notes:[0,4,7,12,7,4,0,null,0,4,7,12,16]},
  defender: {wave:'sawtooth', root:220, beat:.1,  decay:.09, level:.3,  notes:[0,0,12,0,10,0,7,0,0,0,12,15,12,10,7]},
  bubbler:  {wave:'sine',     root:784, beat:.22, decay:.5,  level:.7,  notes:[0,4,7,4,9,7,4,null,2,5,9,7]},
  raider:   {wave:'square',   root:196, beat:.13, decay:.1,  level:.3,  notes:[0,null,0,3,null,5,null,7,5,3,0]},
  meteors:  {wave:'triangle', root:62,  beat:.45, decay:.22, level:1.4, notes:[0,1,0,1,0,1,0,1]},
  serpent:  {wave:'triangle', root:392, beat:.14, decay:.14, level:.8,  notes:[0,2,4,7,9,7,4,2,0,null,-5,0]},
  brawler:  {wave:'sawtooth', root:110, beat:.15, decay:.12, level:.35, notes:[0,null,0,7,null,5,3,null,0,null,12,10]},
  adder:    {wave:'sine',     root:659, beat:.2,  decay:.4,  level:.7,  notes:[0,7,4,12,null,7,9,7]},
  warlock:  {wave:'triangle', root:196, beat:.18, decay:.3,  level:.9,  notes:[0,null,7,5,null,3,7,12]}
};
var blipArcadeSound=(function(){
  var ROOM=.22,HEARD=2.5;
  var slugs=[],pos=0,placed=NaN,enabled=false,ducked=false,ctx=null,room=null,machines={},murmur=null,timer=null,coinAt=0;
  function wanted() { return enabled&&!document.hidden; }
  function wake(unlock) {
    if(!wanted()){sleep();return;}
    if(room)return;
    try {
      var existing=_kioskAudioCtx||Howler.ctx;
      if(!existing&&!unlock)return;
      ctx=getKioskAudio();
      if(ctx.state!=='running'){ctx.resume().then(function(){wake();}).catch(function(){});return;}
      room=ctx.createGain();room.gain.value=ROOM*(ducked?.5:1);room.connect(blipOut(ctx));
      murmur=makeMurmur();
      coinAt=ctx.currentTime+8+Math.random()*12;
      placed=NaN;place(pos);
      timer=setInterval(schedule,200);schedule();
    } catch(e) {}
  }
  function sleep() {
    if(!room)return;
    clearInterval(timer);timer=null;
    Object.keys(machines).forEach(drop);
    murmur.forEach(function(node){if(node.stop)node.stop();node.disconnect();});murmur=null;
    var fading=room;fading.gain.setTargetAtTime(0,ctx.currentTime,.1);setTimeout(function(){fading.disconnect();},600);
    room=null;
  }
  // Talk across the room: noise in the voice band, swelling and falling slowly.
  function makeMurmur() {
    var noise=blipNoise(ctx,3),band=ctx.createBiquadFilter(),level=ctx.createGain(),swell=ctx.createOscillator(),depth=ctx.createGain();
    noise.loop=true;band.type='bandpass';band.frequency.value=520;band.Q.value=.8;
    level.gain.value=.05;swell.frequency.value=.13;depth.gain.value=.025;
    swell.connect(depth);depth.connect(level.gain);noise.connect(band);band.connect(level);level.connect(room);
    noise.start();swell.start();
    return [noise,band,level,swell,depth];
  }
  function machine(slug) {
    var tune=BLIP_ATTRACT[slug];
    if(!tune)return null;
    var gain=ctx.createGain(),pan=ctx.createStereoPanner?ctx.createStereoPanner():null,warm=ctx.createBiquadFilter();
    warm.type='lowpass';warm.frequency.value=2600;gain.gain.value=0;
    warm.connect(gain);if(pan){gain.connect(pan);pan.connect(room);}else gain.connect(room);
    // Machines start out of step, so the room never plays in unison.
    return {slug:slug,tune:tune,gain:gain,pan:pan,warm:warm,level:0,panAt:0,offset:0,next:ctx.currentTime+.3+Math.random()*3};
  }
  function drop(slug) {
    var m=machines[slug];delete machines[slug];
    m.gain.gain.setTargetAtTime(0,ctx.currentTime,.05);
    setTimeout(function(){m.warm.disconnect();m.gain.disconnect();if(m.pan)m.pan.disconnect();},400);
  }
  // Where you stand (in machines, as BlipAisle): who is heard, how loud and from which side.
  function place(at) {
    pos=at;
    // Small moves wait for the next; arriving on a machine always counts.
    if(!room||at===placed||(Math.abs(at-placed)<.01&&at!==Math.round(at)))return;
    placed=at;
    slugs.forEach(function(slug,i){
      var offset=BlipAisle.offset(i,at,slugs.length),d=Math.abs(offset),m=machines[slug];
      if(d>HEARD){if(m)drop(slug);return;}
      if(!m){m=machine(slug);if(!m)return;machines[slug]=m;}
      m.offset=offset;m.level=BlipAisle.machineLevel(d);m.panAt=BlipAisle.machinePan(offset);
      m.gain.gain.setTargetAtTime(m.level,ctx.currentTime,.05);
      if(m.pan)m.pan.pan.setTargetAtTime(m.panAt,ctx.currentTime,.05);
    });
  }
  function note(m,freq,at) {
    var tune=m.tune,osc=ctx.createOscillator(),env=ctx.createGain(),peak=.22*tune.level;
    osc.type=tune.wave;osc.frequency.value=freq;
    env.gain.setValueAtTime(.0001,at);env.gain.exponentialRampToValueAtTime(peak,at+.006);
    env.gain.exponentialRampToValueAtTime(.0001,at+tune.decay);
    osc.connect(env);env.connect(m.warm);osc.start(at);osc.stop(at+tune.decay+.02);
    osc.onended=function(){osc.disconnect();env.disconnect();};
  }
  // A look-ahead of 0.4 s: each machine's jingle, then six to ten seconds of quiet.
  function schedule() {
    if(!room)return;
    var horizon=ctx.currentTime+.4;
    Object.keys(machines).forEach(function(slug){
      var m=machines[slug],tune=m.tune;
      while(m.next<horizon){
        tune.notes.forEach(function(step,i){if(step!==null)note(m,tune.root*Math.pow(2,step/12),m.next+i*tune.beat);});
        m.next+=tune.notes.length*tune.beat+6+Math.random()*4;
      }
    });
    if(coinAt<horizon){coinDrop(coinAt);coinAt+=12+Math.random()*18;}
  }
  // Someone down the aisle drops a coin in.
  function coinDrop(at) {
    var side=ctx.createStereoPanner?ctx.createStereoPanner():null;
    if(side){side.pan.value=(Math.random()<.5?-1:1)*(.5+Math.random()*.4);side.connect(room);}
    [2400,3550,4900].forEach(function(freq,i){
      var osc=ctx.createOscillator(),ring=ctx.createGain(),start=at+i*.006;
      osc.type='triangle';osc.frequency.value=freq;
      ring.gain.setValueAtTime(.05/(i+1),start);ring.gain.exponentialRampToValueAtTime(.0004,start+.08);
      osc.connect(ring);ring.connect(side||room);osc.start(start);osc.stop(start+.09);
      osc.onended=function(){osc.disconnect();ring.disconnect();if(side&&i===2)side.disconnect();};
    });
  }
  function duck(on) {
    ducked=!!on;
    if(room)room.gain.setTargetAtTime(ROOM*(ducked?.5:1),ctx.currentTime,.15);
  }
  // For the tests: who is playing, and the level and pan each is heading for.
  function state() {
    return {running:!!room,ducked:ducked,machines:Object.keys(machines).map(function(slug){var m=machines[slug];return {slug:slug,offset:m.offset,level:m.level,pan:m.panAt};})};
  }
  function enable(names,at) {
    slugs=names.slice();pos=at||0;enabled=true;
    ['pointerdown','keydown','touchstart'].forEach(function(event){
      document.addEventListener(event,function(){wake(true);},{capture:true,passive:true});
    });
    document.addEventListener('visibilitychange',function(){wake();});
    window.addEventListener('pagehide',sleep);
    wake();
  }
  return {enable:enable,place:place,duck:duck,state:state};
}());

// Chute clink, collection-box rattle, then the credit chime.
function playCoinInsert() {
  var ctx = getKioskAudio();
  var t   = ctx.currentTime;

  var noise = blipNoise(ctx, 0.045);
  var noiseFilter = ctx.createBiquadFilter();
  noiseFilter.type = 'bandpass';
  noiseFilter.frequency.value = 4200;
  noiseFilter.Q.value = 1.1;
  var noiseGain = ctx.createGain();
  noiseGain.gain.setValueAtTime(0.4, t);
  noiseGain.gain.exponentialRampToValueAtTime(0.001, t + 0.045);
  noise.connect(noiseFilter); noiseFilter.connect(noiseGain); noiseGain.connect(blipOut(ctx));
  noise.start(t); noise.stop(t + 0.045);

  [2400, 3550, 4900].forEach(function (freq, i) {
    var osc = ctx.createOscillator(), gain = ctx.createGain();
    osc.type = 'triangle'; osc.frequency.value = freq;
    osc.connect(gain); gain.connect(blipOut(ctx));
    var start = t + i * 0.006;
    gain.gain.setValueAtTime(0.11 / (i + 1), start);
    gain.gain.exponentialRampToValueAtTime(0.0008, start + 0.085);
    osc.start(start); osc.stop(start + 0.09);
  });

  function boxHit(at, strength) {
    var noise=blipNoise(ctx,.035,function(t,p){return 1-p;}),filter=ctx.createBiquadFilter(),gain=ctx.createGain();
    filter.type='bandpass';filter.frequency.value=2600;filter.Q.value=1.4;
    gain.gain.setValueAtTime(strength,at);gain.gain.exponentialRampToValueAtTime(.001,at+.035);
    noise.connect(filter);filter.connect(gain);gain.connect(blipOut(ctx));noise.start(at);noise.stop(at+.036);
    [[920,.12,.19],[1470,.075,.15],[2240,.045,.105]].forEach(function(partial){
      var osc=ctx.createOscillator(),ring=ctx.createGain();osc.type='triangle';osc.frequency.value=partial[0];
      ring.gain.setValueAtTime(partial[1]*strength,at);ring.gain.exponentialRampToValueAtTime(.0005,at+partial[2]);
      osc.connect(ring);ring.connect(blipOut(ctx));osc.start(at);osc.stop(at+partial[2]);
    });
  }
  boxHit(t+.66,.9);
  boxHit(t+.73,.42);

  [{ freq: 1047, start: 0.86 }, { freq: 1319, start: 0.915 }].forEach(function(note) {
    [[1, 0.13, 0.17], [2.73, 0.028, 0.11]].forEach(function(partial) {
      var osc = ctx.createOscillator(), gain = ctx.createGain();
      var start = t + note.start, end = start + partial[2];
      osc.type = 'sine';
      osc.frequency.setValueAtTime(note.freq * partial[0], start);
      osc.frequency.exponentialRampToValueAtTime(note.freq * partial[0] * 0.985, end);
      gain.gain.setValueAtTime(partial[1], start);
      gain.gain.exponentialRampToValueAtTime(0.0008, end);
      osc.connect(gain); gain.connect(blipOut(ctx));
      osc.start(start); osc.stop(end + 0.005);
      osc.onended = function() { osc.disconnect(); gain.disconnect(); };
    });
  });
}

// Two footsteps, or one (count 1) when the aisle is walked a half machine at a time; feet alternate.
var machineStepFoot=0;
function playMachineStep(direction,count) {
  try {
    var ctx=getKioskAudio(), start=ctx.currentTime, output=blipOut(ctx);
    var buffer=ctx.createBuffer(1,Math.ceil(ctx.sampleRate*.095),ctx.sampleRate), data=buffer.getChannelData(0);
    for(var i=0;i<data.length;i++)data[i]=(Math.random()*2-1)*Math.exp(-i/(ctx.sampleRate*.022));
    for(var n=0;n<(count||2);n++) {
      let step=machineStepFoot=1-machineStepFoot;
      let at=start+n*.19, foot=ctx.createOscillator(), body=ctx.createGain();
      let grit=ctx.createBufferSource(), low=ctx.createBiquadFilter(), gritGain=ctx.createGain();
      let pan=ctx.createStereoPanner ? ctx.createStereoPanner() : null;
      if(pan){pan.pan.value=(direction<0?-1:1)*(step?-.16:.16);pan.connect(output);}
      var destination=pan||output;
      foot.type='sine';foot.frequency.setValueAtTime(106-step*7,at);foot.frequency.exponentialRampToValueAtTime(54,at+.1);
      body.gain.setValueAtTime(.0001,at);body.gain.exponentialRampToValueAtTime(.055,at+.012);body.gain.exponentialRampToValueAtTime(.0001,at+.13);
      foot.connect(body);body.connect(destination);foot.start(at);foot.stop(at+.14);
      grit.buffer=buffer;low.type='lowpass';low.frequency.value=460;
      gritGain.gain.setValueAtTime(.0001,at);gritGain.gain.exponentialRampToValueAtTime(.012,at+.009);gritGain.gain.exponentialRampToValueAtTime(.0001,at+.085);
      grit.connect(low);low.connect(gritGain);gritGain.connect(destination);grit.start(at);grit.stop(at+.095);
      grit.onended=function(){grit.disconnect();low.disconnect();gritGain.disconnect();};
      foot.onended=function(){foot.disconnect();body.disconnect();if(pan)pan.disconnect();};
    }
  } catch(e) {}
}

/* ---- The controls card ---- Printed card on a little arm: the "?" under the top bar brings it down.
 * A game page lists its keys. */
var BLIP_ARROWS = { '←': 'left', '→': 'right', '↑': 'up', '↓': 'down' };
function legendHtml(controls) {
  function caps(list) {
    return list.map(function (k) {
      return BLIP_ARROWS[k] ? '<kbd class="arrow ' + BLIP_ARROWS[k] + '" aria-label="' + BLIP_ARROWS[k] + ' arrow"></kbd>'
        : '<kbd' + (k.length > 1 ? ' class="wide' + (k === 'SPACE' ? ' space' : '') + (/^(RED|BLUE) /.test(k) ? ' ' + k.split(' ')[0].toLowerCase() : '') + '"' : '') + '>' + k + '</kbd>';
    }).join('');
  }
  return controls.map(function (c) {
    return '<span class="pc-group"><span class="pc-keys">' + caps(c[1]) +
      (c[2] ? '<i>or</i>' + caps(c[2]) : '') + '</span><span class="pc-label">' + c[0] + '</span></span>';
  }).join('');
}
function blipControlsCard(opts) {
  var card = document.createElement('div');
  card.id = 'controls-card';
  card.setAttribute('role', 'dialog');
  card.setAttribute('aria-label', opts.label + ' ' + opts.name);
  card.innerHTML = '<span class="cc-rod" aria-hidden="true"></span><span class="cc-clamp" aria-hidden="true"></span><span class="cc-punch" aria-hidden="true"></span>' +
    '<div class="cc-paper"><p class="cc-title">' + opts.label + '</p><p class="cc-game">' + opts.name + '</p>' + opts.html +
    '<p class="cc-foot">BLIP ARCADE &middot; OPERATOR&rsquo;S CARD</p></div>';
  var sleeve = document.createElement('span');
  sleeve.id = 'card-sleeve';
  sleeve.setAttribute('aria-hidden', 'true');
  var button = document.createElement('button');
  button.id = 'card-button';
  button.type = 'button';
  button.setAttribute('aria-controls', 'controls-card');
  button.setAttribute('aria-expanded', 'false');
  button.setAttribute('aria-label', 'Controls card');
  button.title = 'Controls card';
  button.innerHTML = '<span aria-hidden="true">?</span>';
  document.body.appendChild(card);
  document.body.appendChild(sleeve);
  document.body.appendChild(button);
  var root = document.documentElement;
  function isOpen() { return root.hasAttribute('data-card'); }
  function set(open) {
    root.toggleAttribute('data-card', open);
    button.setAttribute('aria-expanded', open ? 'true' : 'false');
    playControlsCard(open);
    button.blur();
    if (opts.after) opts.after(open);
  }
  button.addEventListener('click', function () { set(!isOpen()); });
  card.addEventListener('click', function () { set(false); });
  // A press anywhere else puts the card away and still reaches what was pressed.
  document.addEventListener('pointerdown', function (e) {
    if (isOpen() && !card.contains(e.target) && !button.contains(e.target)) set(false);
  }, true);
  window.addEventListener('keydown', function (e) {
    if (e.key === 'Escape' && isOpen()) { set(false); e.stopImmediatePropagation(); e.preventDefault(); }
  }, true);
  return { set: set, isOpen: isOpen };
}

// The controls card's little arm: a servo whirr that climbs as the card swings out and falls as it
// is put away, a breath of air, and a soft latch at the end.
function playControlsCard(out) {
  try {
    var ctx=getKioskAudio(),t=ctx.currentTime,dur=out?.6:.42,level=out?1:.7,sink=blipOut(ctx);
    var motor=ctx.createOscillator(),tone=ctx.createBiquadFilter(),volume=ctx.createGain();
    motor.type='sawtooth';motor.frequency.setValueAtTime(out?120:300,t);motor.frequency.linearRampToValueAtTime(out?310:140,t+dur);
    tone.type='lowpass';tone.frequency.setValueAtTime(out?500:1100,t);tone.frequency.linearRampToValueAtTime(out?1200:450,t+dur);tone.Q.value=1.1;
    volume.gain.setValueAtTime(0,t);volume.gain.linearRampToValueAtTime(.05*level,t+.06);
    volume.gain.setValueAtTime(.05*level,t+dur-.07);volume.gain.linearRampToValueAtTime(0,t+dur);
    var cog=ctx.createOscillator(),teeth=ctx.createGain();
    cog.type='triangle';cog.frequency.setValueAtTime(out?20:46,t);cog.frequency.linearRampToValueAtTime(out?48:20,t+dur);teeth.gain.value=.016*level;
    cog.connect(teeth);teeth.connect(volume.gain);
    motor.connect(tone);tone.connect(volume);volume.connect(sink);
    motor.start(t);cog.start(t);motor.stop(t+dur+.05);cog.stop(t+dur+.05);
    motor.onended=function(){motor.disconnect();tone.disconnect();volume.disconnect();cog.disconnect();teeth.disconnect();};
    // The latch: a short tick and a low knock as the card seats.
    var tick=blipNoise(ctx,.05,function(t,p){return Math.pow(1-p,3);}),hp=ctx.createBiquadFilter(),tickGain=ctx.createGain();
    hp.type='highpass';hp.frequency.value=1500;tickGain.gain.value=.18*level;
    tick.connect(hp);hp.connect(tickGain);tickGain.connect(sink);tick.start(t+dur);
    tick.onended=function(){tick.disconnect();hp.disconnect();tickGain.disconnect();};
    var knock=ctx.createOscillator(),knockGain=ctx.createGain();
    knock.type='sine';knock.frequency.setValueAtTime(150,t+dur);knock.frequency.exponentialRampToValueAtTime(70,t+dur+.08);
    knockGain.gain.setValueAtTime(.16*level,t+dur);knockGain.gain.exponentialRampToValueAtTime(.001,t+dur+.1);
    knock.connect(knockGain);knockGain.connect(sink);knock.start(t+dur);knock.stop(t+dur+.12);
    knock.onended=function(){knock.disconnect();knockGain.disconnect();};
  }catch(e){}
}

function playManualPaper(lift) {
  // Page turns are a whisper: soft air through paper, quieter and rounder than the lift of the whole book.
  try {
    var ctx=getKioskAudio(),duration=lift?.4:.34;
    var paper=blipNoise(ctx,duration,function(t,p){return Math.pow(Math.sin(Math.PI*Math.pow(p,lift?1:.8)),1.8)*(.7+.3*Math.sin(p*(lift?31:22)));}),filter=ctx.createBiquadFilter(),soften=ctx.createBiquadFilter(),gain=ctx.createGain();
    filter.type='bandpass';filter.frequency.value=lift?1000:1250;filter.Q.value=.4;
    soften.type='lowpass';soften.frequency.value=lift?3200:2300;gain.gain.value=lift?.085:.04;
    paper.connect(filter);filter.connect(soften);soften.connect(gain);gain.connect(blipOut(ctx));paper.start();
    paper.onended=function(){paper.disconnect();filter.disconnect();soften.disconnect();gain.disconnect();};
  }catch(e){}
}

// The coin that drops into the slot (blip.css #coin-drop-anim): a real
// element so a class toggle can animate it, made on first use in the page's slot.
function dropCoinAnimation() {
  var btn = document.getElementById('kiosk-insert-btn') || document.getElementById('insert-coin-btn');
  if (!btn) return;
  var coinDropAnim = btn.querySelector('#coin-drop-anim');
  if (!coinDropAnim) {
    coinDropAnim = document.createElement('span');
    coinDropAnim.id = 'coin-drop-anim';
    coinDropAnim.setAttribute('aria-hidden', 'true');
    btn.appendChild(coinDropAnim);
  }
  // The slot is the centre of the coin plate, padding-right plus half the
  // plate; both change across breakpoints.
  var padRight = parseFloat(getComputedStyle(btn).paddingRight) || 8;
  var plateW = parseFloat(getComputedStyle(btn, '::after').width) || 18;
  coinDropAnim.style.setProperty('--slot-x', (btn.classList.contains('deck-coin-slot')?btn.offsetWidth/2:padRight+plateW/2)+'px');
  coinDropAnim.classList.remove('dropping');
  void coinDropAnim.offsetWidth;
  coinDropAnim.classList.add('dropping');
  coinDropAnim.addEventListener('animationend', function () {
    coinDropAnim.classList.remove('dropping');
  }, { once: true });
}

function playNoRoom() {
  var ctx  = getKioskAudio();
  var t    = ctx.currentTime;
  var osc  = ctx.createOscillator();
  var gain = ctx.createGain();
  osc.connect(gain);
  gain.connect(blipOut(ctx));
  osc.type = 'sawtooth';
  osc.frequency.setValueAtTime(200, t);
  osc.frequency.exponentialRampToValueAtTime(65, t + 0.38);
  gain.gain.setValueAtTime(0.32, t);
  gain.gain.exponentialRampToValueAtTime(0.001, t + 0.38);
  osc.start(t);
  osc.stop(t + 0.39);
}

function insertCoin() {
  var n = getCoins();
  if (n < MAX_COINS) {
    saveCoins(n + 1);
    playCoinInsert();
    dropCoinAnimation();
    updateCoinsHud();
    updateCoinPrompt();
    if (typeof window.onCoinInserted === 'function') window.onCoinInserted();
  } else {
    playNoRoom();
  }
}

function wireCoinSlot() {
  var slot=document.getElementById('kiosk-insert-btn');
  if(slot)slot.addEventListener('click',insertCoin);
  updateCoinsHud();
}
if(document.readyState==='loading')document.addEventListener('DOMContentLoaded',wireCoinSlot);
else wireCoinSlot();

// Mark the coin control while the landing page has no credits.
function updateCoinPrompt() {
  if (!document.querySelector('.game-grid')) return;
  var btn = document.getElementById('kiosk-insert-btn');
  if (btn) btn.classList.toggle('needs-coin', getCoins() <= 0);
}
window.addEventListener('load', updateCoinPrompt);

/* Poll the first gamepad and report logical button edges to games or the cabinet picker. */
function pollGamepad(onDown, onUp) {
  if (!navigator.getGamepads) return;

  var DEADZONE = 0.25;
  var held = {};

  var BTN_MAP = [
    { idx: 0,  code: 'Space'      },  // A / Cross   — Button 1
    { idx: 1,  code: 'KeyZ'       },  // B / Circle  — Button 2
    { idx: 2,  code: 'KeyZ'       },  // X / Square  — Button 2
    { idx: 3,  code: 'Space'      },  // Y / Triangle— Button 1
    { idx: 4,  code: 'KeyZ'       },  // L shoulder  — Button 2
    { idx: 5,  code: 'KeyZ'       },  // R shoulder  — Button 2
    { idx: 8,  code: 'Select'     },  // Select / Back — arcade menu
    { idx: 9,  code: 'Start'      },  // Start / Options — start the game
    { idx: 12, code: 'ArrowUp'    },
    { idx: 13, code: 'ArrowDown'  },
    { idx: 14, code: 'ArrowLeft'  },
    { idx: 15, code: 'ArrowRight' },
  ];

  function findPad() {
    var pads = navigator.getGamepads();
    for (var i = 0; i < pads.length; i++) {
      if (pads[i] && pads[i].connected) return pads[i];
    }
    return null;
  }

  // Poll continuously because some Linux gamepads omit `gamepadconnected`; lamp dims after `IDLE_MS`.
  var IDLE_MS = 10000;
  var root = document.documentElement;
  var lastInput = -Infinity, lamp = '';
  function setLamp(state) {
    if (state === lamp) return;
    lamp = state;
    if (state) root.setAttribute('data-pad', state);
    else root.removeAttribute('data-pad');
  }

  function tick() {
    var pad = findPad();
    if (!pad) setLamp('');
    if (pad) {
      var want = {};
      for (var j = 0; j < BTN_MAP.length; j++) {
        var m = BTN_MAP[j];
        var b = pad.buttons[m.idx];
        if (b && (b.pressed || b.value > 0.5)) want[m.code] = true;
      }
      var ax = pad.axes[0] || 0, ay = pad.axes[1] || 0;
      if (ax < -DEADZONE) want['ArrowLeft']  = true;
      if (ax >  DEADZONE) want['ArrowRight'] = true;
      if (ay < -DEADZONE) want['ArrowUp']    = true;
      if (ay >  DEADZONE) want['ArrowDown']  = true;

      var now = performance.now();
      for (var any in want) { lastInput = now; break; }
      setLamp(now - lastInput < IDLE_MS ? 'active' : 'idle');
      var code;
      for (code in want) {
        if (!held[code]) { held[code] = true; onDown(code); }
      }
      for (code in held) {
        if (held[code] && !want[code]) { held[code] = false; onUp(code); }
      }
    }
    requestAnimationFrame(tick);
  }

  requestAnimationFrame(tick);
}

// iOS suspends AudioContext on load and re-suspends after backgrounding.
// Howler's autoUnlock is disabled (it calls unload() on non-44100 Hz devices, destroying
// all WASM sounds), so we resume Howler.ctx manually on any gesture and on tab refocus.
(function () {
  function unlockAudio() {
    if (typeof Howler === 'undefined') return;
    if (!Howler.ctx) {
      // Force Howler to create its AudioContext now, while inside a user gesture.
      // On iOS, a context created outside a gesture starts suspended; inside one it starts running.
      // Howler.volume() triggers _setupAudioContext() internally via `ctx || _()`.
      Howler.volume();
    }
    if (Howler.ctx && Howler.ctx.state !== 'running') {
      Howler.ctx.resume();
    }
  }
  // iOS counts a touch's end, not its start, as the gesture that may start sound,
  // and the touch strip suppresses the click that would follow.
  ['touchstart', 'touchend', 'pointerup'].forEach(function (type) {
    document.addEventListener(type, unlockAudio, { passive: true, capture: true });
  });
  document.addEventListener('click',      unlockAudio, { capture: true });
  document.addEventListener('visibilitychange', function () {
    if (!document.hidden) unlockAudio();
  });
}());

/* The side badges and flags sit 16px above the page's deck, whose height
   changes with the controller, layout and player count, or 16px off the
   screen's edge when there is none (a touch screen's front page). */
(function () {
  var GAP = 16;
  function place() {
    var top = Infinity;
    document.querySelectorAll('.kiosk-bar').forEach(function (bar) {
      var r = bar.getBoundingClientRect();
      if (r.height > 0 && r.top < top) top = r.top;
    });
    var v = top === Infinity
      ? 'calc(' + GAP + 'px + env(safe-area-inset-bottom, 0px))'
      : Math.round(window.innerHeight - top + GAP) + 'px';
    document.documentElement.style.setProperty('--side-badges-bottom', v);
  }
  window.addEventListener('resize', place);
  document.addEventListener('DOMContentLoaded', function () {
    place();
    if (typeof ResizeObserver === 'function') {
      document.querySelectorAll('.kiosk-bar').forEach(function (bar) { new ResizeObserver(place).observe(bar); });
    }
  });
}());

function blipCardDrive(duration) {
  try {
    var ctx=getKioskAudio(), out=ctx.createGain(), filter=ctx.createBiquadFilter();
    filter.type='lowpass'; filter.frequency.value=850;
    filter.connect(out); out.connect(blipOut(ctx)); out.gain.value=0;
    var motor=ctx.createOscillator(), harmonic=ctx.createOscillator();
    motor.type='sine'; harmonic.type='triangle';
    var overtone=ctx.createGain(); overtone.gain.value=.22;
    motor.connect(filter); harmonic.connect(overtone); overtone.connect(filter);
    var friction=blipNoise(ctx,.4), rub=ctx.createGain();
    friction.loop=true; rub.gain.value=.12;
    friction.connect(rub); rub.connect(filter);
    motor.start(); harmonic.start(); friction.start();
    var stopped=false;
    function stop() {
      if(stopped) return; stopped=true;
      out.gain.setTargetAtTime(0,ctx.currentTime,.015);
      [motor,harmonic,friction].forEach(function(node){node.stop(ctx.currentTime+.08);});
      setTimeout(function(){out.disconnect();filter.disconnect();},150);
    }
    window.addEventListener('pagehide',stop,{once:true});
    setTimeout(stop,duration);
    return {update:function(progress){
      var speed=Math.sin(progress*Math.PI), now=ctx.currentTime;
      motor.frequency.setTargetAtTime(62+speed*74,now,.025);
      harmonic.frequency.setTargetAtTime(124+speed*148,now,.025);
      out.gain.setTargetAtTime(.04+.105*speed,now,.025);
    },stop:stop};
  } catch(e) {return null;}
}

function playArcadeDock() {
  try {
    var ctx=getKioskAudio(),start=ctx.currentTime,output=ctx.createGain(),nodes=[output];
    output.gain.value=.85;output.connect(blipOut(ctx));
    // Both catches engage together; a short second strike is the latch's rebound.
    [0,.032].forEach(function(offset,index){
      var at=start+offset,snap=blipNoise(ctx,.09,function(t){return Math.exp(-t/.009);}),filter=ctx.createBiquadFilter(),gain=ctx.createGain();
      filter.type='highpass';filter.frequency.value=950;gain.gain.value=index?.28:.9;
      snap.connect(filter);filter.connect(gain);gain.connect(output);snap.start(at);nodes.push(snap,filter,gain);
      [163,431,1187].forEach(function(frequency,partial){
        var tone=ctx.createOscillator(),body=ctx.createGain();tone.type='triangle';tone.frequency.value=frequency;
        body.gain.setValueAtTime((index?.07:.18)/(partial+1),at);body.gain.exponentialRampToValueAtTime(.0001,at+.12);
        tone.connect(body);body.connect(output);tone.start(at);tone.stop(at+.14);nodes.push(tone,body);
      });
    });
    setTimeout(function(){nodes.forEach(function(node){node.disconnect();});},300);
  } catch(e) {}
}

function blipBuildCabinetStars() {
  if (!document.body.classList.contains('cabinet-page')) return;
  var stars = document.createElement('div');
  stars.className = 'cabinet-stars';
  stars.setAttribute('aria-hidden', 'true');
  // Fixed seed keeps the cabinet's speckles in place across page navigation.
  var seed = 1977;
  function random() {
    seed = (Math.imul(seed, 1664525) + 1013904223) >>> 0;
    return seed / 4294967296;
  }
  for (var i = 0; i < 120; i++) {
    var star = document.createElement('i');
    star.style.left = random() * 100 + '%';
    star.style.top = random() * 100 + '%';
    star.style.opacity = (0.12 + random() * 0.23).toFixed(2);
    stars.appendChild(star);
  }
  document.body.prepend(stars);
}
if (document.readyState === 'loading') {
  document.addEventListener('DOMContentLoaded', blipBuildCabinetStars, { once:true });
} else blipBuildCabinetStars();

/* ---- The front-end switch ---- Home Screen, on the top bar of the cabinet and of every game page:
 * beside the logo, else just left of the manual; where the bar has no room for it, centred on the
 * logo's height just after it, with the game's name made small enough to fit what is left of its sign. */
(function () {
  function build() {
    var root = document.documentElement;
    if (!document.body.classList.contains('cabinet-page') || !document.querySelector('.blip-logo') ||
        root.hasAttribute('data-appstyle')) return;
    var up = blipGameFromPath(location.pathname) ? '../' : '';
    function switchButton(id, cls, label, title, html) {
      var b = document.getElementById(id);
      if (b) return b;
      b = document.createElement('button');
      b.type = 'button'; b.id = id; b.className = cls; b.title = title; b.innerHTML = html;
      b.setAttribute('aria-label', label);
      document.body.appendChild(b);
      return b;
    }
    var button = switchButton('home-switch-btn', 'home-switch', 'Switch to the home screen', 'Home screen',
      '<svg viewBox="0 0 12 12" aria-hidden="true"><rect x="1" y="1" width="4" height="4" rx="1.2"/><rect x="7" y="1" width="4" height="4" rx="1.2"/><rect x="1" y="7" width="4" height="4" rx="1.2"/><rect x="7" y="7" width="4" height="4" rx="1.2"/></svg>');
    button.addEventListener('click', function () { blipSetFrontend('home'); location.replace(up + 'home.html'); });
    function shown(el) {
      if (!el) return null;
      var style = getComputedStyle(el), r = el.getBoundingClientRect();
      return r.width > 0 && style.visibility !== 'hidden' && style.display !== 'none' && Number(style.opacity) > 0.05 ? r : null;
    }
    // Squeezed, the switch takes the sign's left end: the sign runs from the pair to the next part on the bar.
    function fitName(from) {
      var sign = document.querySelector('#marquee-name.machine-marquee'), name = sign && sign.querySelector('.machine-marquee-name');
      if (!name) return;
      ['left', 'width', 'transform', 'paddingLeft', 'visibility'].forEach(function (k) { sign.style[k] = ''; });
      name.style.fontSize = '';
      if (from) {
        var next = ['#card-button', '.manual-pocket', '.blip-power-indicator'].map(function (q) { return shown(document.querySelector(q)); })
          .filter(function (r) { return r && r.left > from; }).map(function (r) { return r.left; });
        var to = Math.min.apply(null, next.concat(innerWidth - 8)) - 6;
        sign.style.left = from + 'px'; sign.style.width = Math.max(40, to - from) + 'px';
        sign.style.transform = 'none'; sign.style.paddingLeft = '4px';
      }
      // Whole pixels only: the pixel font blurs between.
      for (var size = parseFloat(getComputedStyle(name).fontSize); size > 7 && name.scrollWidth > sign.clientWidth - 8; size--)
        name.style.fontSize = (size - 1) + 'px';
      // Smaller than 7px cannot be read: a name that still does not fit gives way; the logo leads back.
      if (sign.scrollWidth > sign.clientWidth + 1) sign.style.setProperty('visibility', 'hidden', 'important');
    }
    function place() {
      // Measure the sign where it stands unsqueezed and shown: squeezed by the last run, it would free its own space for the
      // switch, and the two would trade places on every run; hidden, the same.
      fitName(0);
      var sign = document.querySelector('#marquee-name');
      if (sign) sign.style.removeProperty('visibility');
      var logo = document.querySelector('.blip-logo').getBoundingClientRect();
      var manual = shown(document.querySelector('.manual-pocket'));
      var blockers = ['#card-button', '.manual-pocket', '#marquee-name', '.blip-power-indicator']
        .map(function (q) { return shown(document.querySelector(q)); }).filter(Boolean);
      var size = button.offsetWidth;
      var free = function (x) { return x > logo.right + 4 && x + size < innerWidth - 8 && !blockers.some(function (b) { return x < b.right + 8 && x + size > b.left - 8; }); };
      var left = [logo.right + 10, manual ? manual.left - 10 - size : -1].filter(free)[0];
      var squeezed = left === undefined;
      if (squeezed) left = logo.right + 30;   // well clear of the logo
      button.style.left = Math.round(left) + 'px';
      button.style.top = squeezed ? Math.round((logo.top + logo.bottom - size) / 2) + 'px' : '';
      fitName(squeezed ? Math.round(left + size + 6) : 0);
    }
    place();
    window.addEventListener('resize', place);
    window.addEventListener('load', place);
    if (document.fonts && document.fonts.ready) document.fonts.ready.then(place);
    new ResizeObserver(place).observe(document.querySelector('.blip-logo'));
  }
  if (document.readyState === 'loading') document.addEventListener('DOMContentLoaded', build, { once: true });
  else build();
}());

/* ---- The manual, in its pocket on the top bar ---- Opened in place on the arcade; from a game page, in a new tab.
 * The page's own script calls this first (arcade.js, game.js). */
function blipManualPocket() {
  if (document.querySelector('.manual-pocket')) return;
  var game = blipGameFromPath(location.pathname), up = game ? '../' : '';
  var book = document.createElement(document.getElementById('manual-overlay') ? 'button' : 'a');
  book.className = 'cabinet-manual';
  book.id = 'manual-book';
  book.innerHTML = '<span class="manual-book-spine" aria-hidden="true"></span><span class="manual-book-copy"><img class="manual-book-plate" src="' + up +
    'blip-logo.svg" alt=""><strong>MANUAL</strong><small>BLIP · OPERATOR’S COPY</small></span><span class="manual-page-edges" aria-hidden="true"></span>';
  book.setAttribute('data-page-card', 'true');
  book.setAttribute('aria-label', 'Lift the BLIP Arcade Manual');
  book.title = 'Open the BLIP manual';
  if (book.tagName === 'BUTTON') book.type = 'button';
  else {
    book.href = up + 'index.html?manual=controls' + (game ? '&game=' + game.slug : '');
    if (game) { book.target = '_blank'; book.rel = 'noopener'; book.title = 'Manual (opens in a new tab)'; book.setAttribute('aria-label', 'Open manual in a new tab'); }
  }
  var pocket = document.createElement('div');
  pocket.className = 'manual-pocket';
  pocket.appendChild(book);
  document.body.appendChild(pocket);
}

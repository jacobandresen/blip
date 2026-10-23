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
var cabinetPowerTimer=null;
function updateCoinsHud() {
  var root=document.documentElement,powered=blipPowered(),wasPowered=root.hasAttribute('data-credit');
  root.toggleAttribute('data-credit',powered);
  if(powered&&!wasPowered){
    clearTimeout(cabinetPowerTimer);root.setAttribute('data-power-up','');
    cabinetPowerTimer=setTimeout(function(){root.removeAttribute('data-power-up');},1200);
  }else if(!powered){clearTimeout(cabinetPowerTimer);root.removeAttribute('data-power-up');}
  blipUpdateCabinetHum();
}

/* ---- Audio ---- */
var _kioskAudioCtx = null;

function getKioskAudio() {
  if (typeof Howler !== 'undefined' && Howler.ctx) {
    if (Howler.ctx.state === 'suspended') Howler.ctx.resume();
    return Howler.ctx;
  }
  if (!_kioskAudioCtx) _kioskAudioCtx = new (window.AudioContext || window.webkitAudioContext)();
  if (_kioskAudioCtx.state === 'suspended') _kioskAudioCtx.resume();
  return _kioskAudioCtx;
}

var cabinetHum=null;
function blipUpdateCabinetHum(unlock) {
  var powered=blipPowered()&&!document.hidden&&!document.documentElement.hasAttribute('data-cabinet-screen');
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
    var existing=_kioskAudioCtx||(typeof Howler!=='undefined'&&Howler.ctx);
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
  adder:    {wave:'sine',     root:659, beat:.2,  decay:.4,  level:.7,  notes:[0,7,4,12,null,7,9,7]}
};
var blipArcadeSound=(function(){
  var ROOM=.22,HEARD=2.5;
  var slugs=[],pos=0,placed=NaN,enabled=false,ducked=false,ctx=null,room=null,machines={},murmur=null,timer=null,coinAt=0;
  function wanted() { return enabled&&!document.hidden; }
  function wake(unlock) {
    if(!wanted()){sleep();return;}
    if(room)return;
    try {
      var existing=_kioskAudioCtx||(typeof Howler!=='undefined'&&Howler.ctx);
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
    var buffer=ctx.createBuffer(1,ctx.sampleRate*3,ctx.sampleRate),data=buffer.getChannelData(0);
    for(var i=0;i<data.length;i++)data[i]=Math.random()*2-1;
    var noise=ctx.createBufferSource(),band=ctx.createBiquadFilter(),level=ctx.createGain(),swell=ctx.createOscillator(),depth=ctx.createGain();
    noise.buffer=buffer;noise.loop=true;band.type='bandpass';band.frequency.value=520;band.Q.value=.8;
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

  var noiseBuf = ctx.createBuffer(1, Math.ceil(ctx.sampleRate * 0.045), ctx.sampleRate);
  var noiseData = noiseBuf.getChannelData(0);
  for (var i = 0; i < noiseData.length; i++) noiseData[i] = Math.random() * 2 - 1;
  var noise = ctx.createBufferSource();
  noise.buffer = noiseBuf;
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
    var size=Math.ceil(ctx.sampleRate*.035),buffer=ctx.createBuffer(1,size,ctx.sampleRate),data=buffer.getChannelData(0);
    for(var i=0;i<size;i++)data[i]=(Math.random()*2-1)*(1-i/size);
    var noise=ctx.createBufferSource(),filter=ctx.createBiquadFilter(),gain=ctx.createGain();
    noise.buffer=buffer;filter.type='bandpass';filter.frequency.value=2600;filter.Q.value=1.4;
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

// A switch latch and the muted clack of a card seating in its guide.
function playCardMechanism() {
  try {
    var ctx = getKioskAudio(), t = ctx.currentTime;
    var size = Math.ceil(ctx.sampleRate * 0.026), buffer = ctx.createBuffer(1, size, ctx.sampleRate);
    var data = buffer.getChannelData(0);
    for (var i = 0; i < size; i++) data[i] = (Math.random() * 2 - 1) * (1 - i / size);
    var noise = ctx.createBufferSource(), filter = ctx.createBiquadFilter(), snap = ctx.createGain();
    noise.buffer = buffer; filter.type = 'bandpass'; filter.frequency.value = 1750; filter.Q.value = 1.3;
    snap.gain.setValueAtTime(0.12, t); snap.gain.exponentialRampToValueAtTime(0.0001, t + 0.027);
    noise.connect(filter); filter.connect(snap); snap.connect(blipOut(ctx));
    noise.start(t); noise.stop(t + 0.028);
    var osc = ctx.createOscillator(), body = ctx.createGain();
    osc.type = 'triangle'; osc.frequency.setValueAtTime(230, t + 0.004);
    osc.frequency.exponentialRampToValueAtTime(105, t + 0.065);
    body.gain.setValueAtTime(0.0001, t); body.gain.exponentialRampToValueAtTime(0.12, t + 0.006);
    body.gain.exponentialRampToValueAtTime(0.0001, t + 0.085);
    osc.connect(body); body.connect(blipOut(ctx)); osc.start(t); osc.stop(t + 0.09);
  } catch (e) {}
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
    var size=Math.ceil(ctx.sampleRate*.05),buffer=ctx.createBuffer(1,size,ctx.sampleRate),data=buffer.getChannelData(0);
    for(var i=0;i<size;i++)data[i]=(Math.random()*2-1)*Math.pow(1-i/size,3);
    var tick=ctx.createBufferSource(),hp=ctx.createBiquadFilter(),tickGain=ctx.createGain();
    tick.buffer=buffer;hp.type='highpass';hp.frequency.value=1500;tickGain.gain.value=.18*level;
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
    var ctx=getKioskAudio(),duration=lift?.4:.34,size=Math.ceil(ctx.sampleRate*duration);
    var buffer=ctx.createBuffer(1,size,ctx.sampleRate),data=buffer.getChannelData(0);
    for(var i=0;i<size;i++){var p=i/size;data[i]=(Math.random()*2-1)*Math.pow(Math.sin(Math.PI*Math.pow(p,lift?1:.8)),1.8)*(.7+.3*Math.sin(p*(lift?31:22)));}
    var paper=ctx.createBufferSource(),filter=ctx.createBiquadFilter(),soften=ctx.createBiquadFilter(),gain=ctx.createGain();
    paper.buffer=buffer;filter.type='bandpass';filter.frequency.value=lift?1000:1250;filter.Q.value=.4;
    soften.type='lowpass';soften.frequency.value=lift?3200:2300;gain.gain.value=lift?.085:.04;
    paper.connect(filter);filter.connect(soften);soften.connect(gain);gain.connect(blipOut(ctx));paper.start();
    paper.onended=function(){paper.disconnect();filter.disconnect();soften.disconnect();gain.disconnect();};
  }catch(e){}
}

// The coin that drops into the slot (kiosk.css #coin-drop-anim): a real
// element so a class toggle can animate it, injected once for every non-game
// page.
var coinDropAnim = null;
(function () {
  var btn = document.getElementById('kiosk-insert-btn');
  if (!btn) return;
  coinDropAnim = btn.querySelector('#coin-drop-anim');
  if(!coinDropAnim) {
    coinDropAnim = document.createElement('span');
    coinDropAnim.id = 'coin-drop-anim';
    coinDropAnim.setAttribute('aria-hidden', 'true');
    btn.appendChild(coinDropAnim);
  }
}());
function dropCoinAnimation() {
  if (!coinDropAnim) return;
  // The slot is the centre of the coin plate, padding-right plus half the
  // plate; both change across breakpoints.
  var btn = coinDropAnim.parentElement;
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

function wireCabinetLogo() {
  document.querySelectorAll('.led-logo').forEach(function(logo){
    if(logo.querySelector('.logo-power-wire'))return;
    var wire=document.createElementNS('http://www.w3.org/2000/svg','svg');
    wire.setAttribute('class','logo-power-wire');wire.setAttribute('viewBox','0 0 32 40');wire.setAttribute('aria-hidden','true');
    wire.innerHTML='<circle cx="27" cy="11" r="4.5" fill="#211c16" stroke="#9e8764" stroke-width="1.5"/>'+
      '<path class="wire-shadow" d="M0 22 H7 C14 22 10 34 20 34 C30 34 27 23 27 11"/>'+
      '<path class="wire-jacket" d="M0 20 H7 C14 20 10 32 20 32 C30 32 27 21 27 11"/>'+
      '<path class="wire-highlight" d="M0 19 H7 C14 19 10 31 20 31 C30 31 27 20 27 11"/>'+
      '<path class="wire-current" d="M0 19 H7 C14 19 10 31 20 31 C30 31 27 20 27 11"/>'+
      '<rect x="-2" y="16" width="7" height="8" rx="1.5" fill="#40372b" stroke="#aa9270" stroke-width=".8"/>';
    logo.appendChild(wire);
    logo.querySelectorAll('.led-on').forEach(function(lamp){lamp.style.setProperty('--led-bank',3-Math.floor(Number(lamp.getAttribute('cx'))/24));});
    var display=logo.querySelector('.blip-led-display'),brand=document.createElementNS('http://www.w3.org/2000/svg','g');
    brand.setAttribute('class','led-blip');
    while(display.firstChild)brand.appendChild(display.firstChild);display.appendChild(brand);
  });
}
function wireCoinSlot() {
  wireCabinetLogo();
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

/* ---- BLIP logo overcharge glitch ---- Now and then one letter arcs as if
 * surged, the rest flicker, with a zap on the speaker. Every page loads
 * kiosk.js; deferred to 'load' because game pages load it in <head>. */
(function () {
  var reduceMotion = !!(window.matchMedia && window.matchMedia('(prefers-reduced-motion: reduce)').matches);
  if (reduceMotion) return; // no schedule at all — this is pure motion, nothing informational

  window.addEventListener('load', function () {
    var logo = document.querySelector('.blip-logo');
    if (!logo || !logo.firstChild || logo.firstChild.nodeType !== Node.TEXT_NODE) return;

    // One span per letter of "BLIP" (the part of the wordmark always shown)
    // so one can be struck.
    var letters = logo.firstChild.textContent.split('');
    var frag = document.createDocumentFragment();
    letters.forEach(function (ch) {
      var span = document.createElement('span');
      span.className = 'lg-ch';
      span.textContent = ch;
      frag.appendChild(span);
    });
    logo.replaceChild(frag, logo.firstChild);
    var chars = logo.querySelectorAll('.lg-ch');
    if (!chars.length) return;

    // An electrical crackle (filtered noise burst) under a fast descending
    // zap (a sawtooth sweeping from a shriek down to a thud) — the same
    // procedural-audio recipe as playCoinInsert/playNoRoom above.
    function playZap() {
      var ctx = getKioskAudio();
      var t = ctx.currentTime;

      // A crackle burst (filtered noise), sharper and louder than the
      // coin sfx's — this has to carry across a whole room, not just
      // confirm a tap.
      function crackle(start, dur, gain) {
        var buf = ctx.createBuffer(1, Math.ceil(ctx.sampleRate * dur), ctx.sampleRate);
        var data = buf.getChannelData(0);
        for (var i = 0; i < data.length; i++) data[i] = (Math.random() * 2 - 1) * (1 - i / data.length);
        var noise = ctx.createBufferSource();
        noise.buffer = buf;
        var filt = ctx.createBiquadFilter();
        filt.type = 'highpass';
        filt.frequency.value = 2200;
        var ng = ctx.createGain();
        ng.gain.setValueAtTime(gain, start);
        ng.gain.exponentialRampToValueAtTime(0.001, start + dur);
        noise.connect(filt); filt.connect(ng); ng.connect(blipOut(ctx));
        noise.start(start); noise.stop(start + dur);
      }
      crackle(t, 0.16, 0.65);
      crackle(t + 0.22, 0.09, 0.4);   // a second, smaller sputter as it flickers back
      crackle(t + 0.36, 0.06, 0.25);

      // The main arc: a fast descending sawtooth sweep, shriek to thud.
      var osc = ctx.createOscillator();
      var og = ctx.createGain();
      osc.type = 'sawtooth';
      osc.frequency.setValueAtTime(3200, t);
      osc.frequency.exponentialRampToValueAtTime(70, t + 0.2);
      og.gain.setValueAtTime(0.3, t);
      og.gain.exponentialRampToValueAtTime(0.001, t + 0.22);
      osc.connect(og); og.connect(blipOut(ctx));
      osc.start(t); osc.stop(t + 0.23);

      // A low undertone thump, like the sign's transformer took the hit.
      var thump = ctx.createOscillator();
      var tg = ctx.createGain();
      thump.type = 'triangle';
      thump.frequency.setValueAtTime(110, t);
      thump.frequency.exponentialRampToValueAtTime(45, t + 0.3);
      tg.gain.setValueAtTime(0.28, t + 0.02);
      tg.gain.exponentialRampToValueAtTime(0.001, t + 0.32);
      thump.connect(tg); tg.connect(blipOut(ctx));
      thump.start(t + 0.02); thump.stop(t + 0.33);
    }

    function zap() {
      var el = chars[Math.floor(Math.random() * chars.length)];
      logo.classList.add('lg-surge');
      el.classList.add('lg-zap');
      playZap();
      var done = function () {
        el.classList.remove('lg-zap');
        logo.classList.remove('lg-surge');
      };
      el.addEventListener('animationend', done, { once: true });
      setTimeout(done, 900); // in case the tab was hidden mid-animation and it never fired
    }

    // The next strike is 45s-3min out, re-armed after each one (and while
    // hidden) rather than on an interval, so a backgrounded tab does not fire
    // a burst of overdue zaps.
    function scheduleNext() {
      var delay = 45000 + Math.random() * 135000;
      setTimeout(function () {
        if (document.hidden) { scheduleNext(); return; }
        zap();
        scheduleNext();
      }, delay);
    }
    scheduleNext();
  });
}());

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

/* The deck itself is built by deck.js, the same on every page. */

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

// The telescoping boom keeps its level clamp clear of the receiver.
window.blipTransferCard = function (options) {
  var stage = document.querySelector('.jukebox-transfer.idle') || document.createElement('div');
  stage.className = 'jukebox-transfer active' + (options.idle ? ' idle' : '');
  stage.setAttribute('aria-hidden', 'true');
  var cabinet=document.querySelector('.screen-bezel');
  var cabinetBox=cabinet&&cabinet.getBoundingClientRect();
  if(cabinetBox) {
    var fascia=document.querySelector('.top-marquee-bar, #marquee-bar'),fasciaBottom=fascia?fascia.getBoundingClientRect().bottom:40;
    stage.style.clipPath='inset('+Math.max(0,fasciaBottom)+'px '+Math.max(0,innerWidth-cabinetBox.right)+'px '+
      Math.max(0,innerHeight-cabinetBox.bottom)+'px '+Math.max(0,cabinetBox.left)+'px)';
  }
  stage.innerHTML = '<div class="jukebox-mechanics-bar"></div><div class="jukebox-transfer-arm articulated"><i class="jukebox-arm-anchor"></i><i class="jukebox-arm-mount"></i><i class="jukebox-arm-shaft"></i><i class="jukebox-arm-forearm"></i><i class="jukebox-arm-pivot"></i><div class="jukebox-arm-wrist"><div class="jukebox-flight-card"><img alt=""><strong></strong><small></small></div><i class="jukebox-arm-grip"></i></div></div>';
  var returning=!!options.returning;
  stage.dataset.cycle=returning?'return':'insert';
  var totalTime=returning?12300:14700;
  var arm = stage.querySelector('.jukebox-transfer-arm'), card = arm.querySelector('.jukebox-flight-card');
  card.querySelector('img').src = options.art;
  card.querySelector('strong').textContent = options.name;
  card.querySelector('small').textContent = options.code;
  var width = options.width, height = options.height, scale = options.scale || 1;
  if (options.original) {
    var copy = options.original.cloneNode(true);
    var originals = [options.original].concat(Array.from(options.original.querySelectorAll('*')));
    var copies = [copy].concat(Array.from(copy.querySelectorAll('*')));
    originals.forEach(function (original,index) {
      var styles=getComputedStyle(original);
      Array.from(styles).forEach(function (key) { copies[index].style.setProperty(key,styles.getPropertyValue(key)); });
    });
    copies.slice(1).forEach(function(part){part.style.visibility='visible';});
    copy.className='jukebox-flight-card'; copy.removeAttribute('href');
    copy.style.minWidth='0'; copy.style.minHeight='0'; copy.style.maxWidth='none'; copy.style.maxHeight='none';
    copy.style.position='absolute'; copy.style.margin='0'; copy.style.transition='none';
    copy.style.left='0'; copy.style.top='0'; copy.style.zIndex='5';
    copy.style.transformOrigin='50% 100%';
    copy.style.transform='translate(-50%,-100%) scale('+scale+')';
    card.replaceWith(copy); card=copy;
  }
  var cardOpacity=parseFloat(card.style.opacity || '1');
  var physicalHeight=height*scale,physicalWidth=width*scale;
  var exposedEdge=Math.min(18,physicalHeight*.2),feedTravel=physicalHeight-exposedEdge;
  card.style.visibility='hidden';
  card.style.width = width + 'px'; card.style.height = height + 'px';
  var pivot = options.pivot, source = options.source, slot = options.slot;
  var initialAngle=options.angle || 0, radians=initialAngle*Math.PI/180;
  var start = options.idle ? {x:pivot.x-58,y:pivot.y} :
    {x:source.x-Math.sin(radians)*physicalHeight/2, y:source.y+Math.cos(radians)*physicalHeight/2};
  var approach = {x:slot.x, y:slot.y + physicalHeight};
  var park = {x:pivot.x-58,y:pivot.y};
  var feedStart=returning?2100:7100;
  var gripDrop=returning?-exposedEdge-6:14-physicalHeight;
  var sourceGrip={x:start.x-Math.sin(radians)*gripDrop,y:start.y+Math.cos(radians)*gripDrop};
  var pickupGrip=returning?{x:slot.x,y:slot.y+exposedEdge+gripDrop}:sourceGrip;
  var toolSide=physicalWidth/2+38,bridge=toolSide-20;
  var restingRom=null;
  if(options.original&&!options.idle) {
    restingRom=card.cloneNode(true);restingRom.className='jukebox-resting-rom';
    restingRom.style.position='fixed';restingRom.style.zIndex='21';
    restingRom.style.left=start.x+'px';restingRom.style.top=start.y+'px';
    restingRom.style.transform='translate(-50%,-100%) rotate('+initialAngle+'deg) scale('+scale+')';
    restingRom.style.visibility='visible';restingRom.style.pointerEvents='none';
    arm.appendChild(restingRom);
  }
  var shaft = arm.querySelector('.jukebox-arm-shaft'), forearm = arm.querySelector('.jukebox-arm-forearm');
  var wrist = arm.querySelector('.jukebox-arm-wrist');
  wrist.dataset.depth='12';
  var grip=arm.querySelector('.jukebox-arm-grip');arm.appendChild(grip);
  grip.innerHTML='<i class="jukebox-claw-rail"></i><i class="jukebox-claw-jaw left"><b></b></i><i class="jukebox-claw-jaw right"><b></b></i><i class="jukebox-claw-lock"></i>';
  grip.style.setProperty('--rom-half',(physicalWidth/2)+'px');
  var tool=document.createElement('i');tool.className='jukebox-arm-tool';arm.appendChild(tool);
  var bearing=document.createElement('i');bearing.className='jukebox-arm-bearing';arm.appendChild(bearing);
  var shoulderLift=document.createElement('i');shoulderLift.className='jukebox-joint-lift shoulder';arm.appendChild(shoulderLift);
  shoulderLift.style.left=pivot.x+'px';shoulderLift.style.top=pivot.y+'px';
  var mechanicsBar=stage.querySelector('.jukebox-mechanics-bar');
  var rotaryPivot=arm.querySelector('.jukebox-arm-pivot'),rotor={x:pivot.x,y:pivot.y};
  rotaryPivot.dataset.joint='shoulder';arm.dataset.drive='telescopic';
  wrist.dataset.layer='rom';
  arm.querySelector('.jukebox-arm-mount').dataset.layer='chassis';
  tool.dataset.layer='gripper';
  [arm.querySelector('.jukebox-arm-mount'), arm.querySelector('.jukebox-arm-pivot')].forEach(function (part) {
    part.style.left = pivot.x + 'px'; part.style.top = pivot.y + 'px';
  });
  var anchor=arm.querySelector('.jukebox-arm-anchor'),guide=document.querySelector('.arcade-guide.right');
  anchor.style.left=pivot.x+'px';anchor.style.top=pivot.y+'px';
  anchor.style.width=(guide?Math.max(30,guide.getBoundingClientRect().left+5-pivot.x):40)+'px';
  function ease(t) { return t*t*t*(10+t*(-15+6*t)); }
  function mix(a,b,t) { return a+(b-a)*t; }
  function link(el,a,b,length) {
    el.style.left = a.x + 'px'; el.style.top = a.y + 'px';
    el.style.width = length + 'px';
    el.style.transform = 'rotate(' + Math.atan2(b.y-a.y,b.x-a.x) + 'rad)';
  }
  var picked = false, seated = false, lifted = false, extracted = false, drive = null, guideClicks = 0;
  var jointSound=null,loadSound=null,lastPose=null,soundEvents={};
  function stopSound() {
    if(jointSound){jointSound.stop();jointSound=null;}
    if(loadSound){loadSound.stop();loadSound=null;}
    if(drive){drive.stop();drive=null;}
  }
  function drawer(stow) {
    arm.style.transform='translateY('+(-60*stow)+'px)';
    mechanicsBar.style.transform='translateY('+(-60*stow)+'px)';
  }
  function render(ms) {
    var x=start.x, y=start.y, roll=options.idle?0:initialAngle, feed=0;
    if (ms>=2100 && ms<3100) {
      y=start.y-12*ease((ms-2100)/1000);
    } else if (ms >= 3100 && ms < 7100) {
      var t = ease((ms-3100)/4000);
      x=mix(start.x,approach.x,t); y=mix(start.y-12,approach.y,t)-Math.sin(t*Math.PI)*8;
      roll=mix(initialAngle,0,t)+Math.sin(t*Math.PI)*-8;
    } else if (ms >= 7100) {
      feed=ease(Math.min(1,(ms-7100)/2500));
      x=slot.x; y=approach.y-feedTravel*feed; roll=0;
    }
    if(!paused && !options.idle) {
      if(ms>=feedStart && ms<feedStart+2500) {
        if(!drive) drive=blipCardDrive();
        if(drive) drive.update((ms-feedStart)/2500);
        var clicks=Math.floor((ms-feedStart)/700);
        if(clicks>guideClicks) {guideClicks=clicks;playCardLatch(true);}
      } else if(drive) {drive.stop();drive=null;}
    }
    if(returning) {
      roll=0; feed=1;
      if(ms<2100) {x=slot.x;y=slot.y+exposedEdge;}
      else if(ms<4600) {
        feed=1-ease((ms-2100)/2500);x=slot.x;y=slot.y+exposedEdge+feedTravel*(1-feed);
      } else if(ms<8600) {
        var home=ease((ms-4600)/4000); feed=0;
        x=mix(approach.x,start.x,home);y=mix(approach.y,start.y-12,home)-Math.sin(home*Math.PI)*8;
        roll=mix(0,initialAngle,home)+Math.sin(home*Math.PI)*-8;
      } else if(ms<9800) {
        feed=0;x=start.x;y=start.y-12*(1-ease(Math.min(1,(ms-8600)/1000)));roll=initialAngle;
      } else {
        feed=0;roll=0;
      }
    }
    var toolAngle=roll*Math.PI/180;
    var carrierY=y+(returning?0:feedTravel*feed);
    var withdraw=returning?0:ease(Math.max(0,Math.min(1,(ms-7400)/800)))*
      (1-ease(Math.max(0,Math.min(1,(ms-10200)/2500))));
    var carrierX=x+12*withdraw;carrierY+=32*withdraw;
    var gripPoint={x:carrierX-Math.sin(toolAngle)*gripDrop,y:carrierY+Math.cos(toolAngle)*gripDrop};
    if(ms<1500) {
      if(ms<900) {
        var reach=ease(ms/900);
        gripPoint={x:mix(park.x,pickupGrip.x,reach),y:mix(park.y,pickupGrip.y-36,reach)};
      } else gripPoint={x:pickupGrip.x,y:mix(pickupGrip.y-36,pickupGrip.y,ease((ms-900)/600))};
    }
    if(returning&&ms>=9800) {
      if(ms<10700) {
        var rise=ease((ms-9800)/900);
        gripPoint={x:sourceGrip.x,y:sourceGrip.y-36*rise};
        roll=mix(initialAngle,0,rise);toolAngle=roll*Math.PI/180;
      } else {
        var retreat=ease(Math.min(1,(ms-10700)/1600));
        gripPoint={x:mix(sourceGrip.x,park.x,retreat),y:mix(sourceGrip.y-36,park.y,retreat)};
      }
    } else if(!returning&&ms>=10200) {
      var retreat=ease(Math.min(1,(ms-10200)/2500));
      gripPoint={x:mix(slot.x+12,park.x,retreat),y:mix(slot.y+46,park.y,retreat)};
    }
    if(options.idle) gripPoint={x:park.x,y:park.y};
    var toolPoint={x:gripPoint.x+Math.cos(toolAngle)*toolSide,y:gripPoint.y+Math.sin(toolAngle)*toolSide};
    var toolStart={x:gripPoint.x+Math.cos(toolAngle)*bridge,y:gripPoint.y+Math.sin(toolAngle)*bridge};
    var elevation=ms<1500?0:ms<2100?ease((ms-1500)/600):
      1-ease(Math.max(0,Math.min(1,(ms-(returning?9800:10200))/2500)));
    if(options.idle)elevation=0;
    // Elevation changes depth; the shoulder stays centred in its chassis bay.
    arm.dataset.elevation=elevation.toFixed(3);
    arm.dataset.depth=(16+12*elevation).toFixed(2);
    var distance=Math.hypot(toolPoint.x-rotor.x,toolPoint.y-rotor.y),shoulderAngle=Math.atan2(toolPoint.y-rotor.y,toolPoint.x-rotor.x);
    var housing=Math.min(52,distance),overlap=Math.min(14,housing),slide=Math.max(0,housing-overlap);
    var carriage={x:rotor.x+Math.cos(shoulderAngle)*slide,y:rotor.y+Math.sin(shoulderAngle)*slide};
    link(shaft,rotor,toolPoint,housing);link(forearm,carriage,toolPoint,distance-slide);
    shoulderLift.style.setProperty('--joint-rise',(12*elevation)+'px');
    // A 20px standoff keeps the wrist outside the open jaw's sweep.
    link(tool,toolStart,toolPoint,20);
    bearing.style.left=toolPoint.x+'px';bearing.style.top=toolPoint.y+'px';
    bearing.style.setProperty('--bearing-angle',roll+'deg');
    rotaryPivot.style.setProperty('--bearing-angle',shoulderAngle+'rad');
    var jawClose=ease(Math.max(0,Math.min(1,(ms-1500)/600)))*
      (1-ease(Math.max(0,Math.min(1,(ms-(returning?9300:7100))/300))));
    grip.style.setProperty('--jaw-travel',(12*jawClose)+'px');
    grip.style.setProperty('--jaw-close',jawClose);
    grip.dataset.holding=String(jawClose>.999&&(returning?ms>=2100&&ms<9300:ms>=2100&&ms<=7100));
    var gripDepth=16+12*elevation-16*jawClose;
    grip.dataset.depth=gripDepth.toFixed(2);
    grip.style.filter='drop-shadow(0 '+Math.max(0,(gripDepth-12)*.4)+'px 2px #000a)';
    if(!paused&&!options.idle) {
      if(!jointSound)jointSound=blipArmJointSound();
      if(lastPose&&ms>lastPose.ms&&jointSound) {
        var dt=(ms-lastPose.ms)/1000;
        function angularSpeed(a,b){return Math.abs(Math.atan2(Math.sin(a-b),Math.cos(a-b)))/dt;}
        jointSound.update(angularSpeed(shoulderAngle,lastPose.shoulder),Math.abs(distance-lastPose.reach)/(70*dt),Math.abs(elevation-lastPose.elevation)/dt);
      }
      lastPose={ms:ms,shoulder:shoulderAngle,reach:distance,elevation:elevation};
      [[1500,'valve'],[2100,'attach'],[returning?4600:9600,returning?'unseat':'seat'],[returning?9600:7400,'release'],[returning?12300:12700,'vent']].forEach(function(event){
        if(ms>=event[0]&&!soundEvents[event[1]]){soundEvents[event[1]]=true;playRomCoupling(event[1]);}
      });
    }
    wrist.style.left=x+'px'; wrist.style.top=y+'px'; wrist.style.transform='rotate('+roll+'deg)';
    var clearance=returning?1-ease(Math.max(0,Math.min(1,(ms-8600)/1000))):ease(Math.max(0,Math.min(1,(ms-2100)/1000)));
    wrist.style.filter='drop-shadow(0 '+(9*clearance)+'px '+(4*clearance)+'px #000b)';
    grip.style.left=(gripPoint.x-16)+'px';grip.style.top=(gripPoint.y-11)+'px';grip.style.transform='rotate('+roll+'deg)';
    card.style.clipPath='inset('+(feed*feedTravel/physicalHeight*100)+'% 0 0 0)';
    if(returning&&ms>=2100)options.receiver.classList.remove('loaded');
    card.style.visibility=ms>=2100 && ms<9600?'visible':'hidden';
    card.style.opacity=ms>=2100 && ms<9600?cardOpacity:0;
    card.classList.toggle('seated',ms>=9600);
    if(restingRom)restingRom.style.display=(returning?ms>=9600:ms<2100)?'flex':'none';
    arm.classList.toggle('clamped',ms>=1500 && ms<(returning?9600:9800));
    if(ms>=2100 && !picked) {
      picked=true; card.style.visibility='visible';
      if(!returning && options.onPickup) options.onPickup();
    }
    var stow=returning?0:ease(Math.max(0,Math.min(1,(ms-12700)/2000)));
    drawer(stow);
    if(ms>=14700) options.receiver.classList.add('stowed');
    if(returning && ms>=4600 && !extracted) {extracted=true;options.receiver.classList.remove('loaded');if(options.onExtract) options.onExtract();}
    if(!returning && ms>=3100 && !lifted) {lifted=true;if(options.onLift) options.onLift();}
    stage.dataset.phase = ms<2100?'grip':ms<3100?'lift':ms<7100?'swing':ms<9600?'feed':ms<10200?'release':ms<12700?'return':'stow';
    if (ms >= 9600 && !seated) {
      seated=true; card.classList.add('seated');
      if(returning)options.receiver.classList.remove('loaded');
      else {
        blipSeatRom(options.receiver,options);
        if(!paused&&!options.idle)loadSound=playRomLoading(Math.min(4.8,(totalTime-ms)/1000-.1));
      }
      if(returning && options.onReturned) options.onReturned();
      else if (options.onSeat) options.onSeat();
    }
  }
  document.body.appendChild(stage);
  var paused=false;
  render(0);
  var began=performance.now();
  // A deterministic pose is useful when inspecting the joints in screenshots.
  stage.blipSeek=function(ms) { paused=true; stopSound(); render(ms); };
  function frame(now) {
    if (paused) return;
    var ms=now-began; render(ms);
    if(ms<totalTime) requestAnimationFrame(frame);
    else { stopSound();stage.remove(); if(options.onDone) options.onDone(); }
  }
  if (!options.idle && typeof playCardMechanism==='function') playCardMechanism();
  if(options.idle) {
    paused=true; options.receiver.classList.remove('stowed');
    if(options.deploy) {
      stage.dataset.deploying='true'; drawer(1);
      var deployStart=performance.now();
      function deploy(now) {
        var t=Math.min(1,(now-deployStart)/2000); drawer(1-ease(t));
        if(t<1) requestAnimationFrame(deploy);
        else { delete stage.dataset.deploying; stage.dispatchEvent(new Event('blip-deployed')); }
      }
      requestAnimationFrame(deploy);
    }
  } else requestAnimationFrame(frame);
  return stage;
};

function blipCardReceiver(bar) {
  var receiver=bar.querySelector('.jukebox-receiver');
  if (!receiver) {
    var game=blipGameFromPath(location.pathname),interactive=game&&!document.documentElement.hasAttribute('data-cabinet-screen');
    receiver=document.createElement(interactive?'a':'div'); receiver.className='jukebox-receiver';
    if(interactive){receiver.href='../index.html';receiver.setAttribute('aria-label','Return to the cabinet');receiver.title='Return to the cabinet';}
    else receiver.setAttribute('aria-hidden','true');
    receiver.innerHTML='<div class="jukebox-receiver-lip"><i></i><i></i></div><div class="jukebox-seated-card"><i></i><i></i></div>';
    bar.appendChild(receiver);
    try {
      var loaded=JSON.parse(sessionStorage.getItem('blip-loaded-card')||'null');
      if(loaded)blipSeatRom(receiver,loaded);
    }catch(e){}
  }
  return receiver;
}

function blipSeatRom(receiver,rom) {
  receiver.dataset.rom=rom.slug||rom.name||'';
  receiver.classList.add('loaded');
}

function blipCardArmPivot(receiver) {
  var box=receiver.getBoundingClientRect(),bar=receiver.closest('.top-marquee-bar, #marquee-bar');
  var top=bar?bar.getBoundingClientRect().bottom:box.top;
  var guide=document.querySelector('.arcade-guide.right'),center=box.left+box.width/2;
  var cabinetWidth=Math.min(960,document.documentElement.clientWidth-32);
  var x=guide?guide.getBoundingClientRect().left-30:center+Math.min(205,(cabinetWidth-26)/2)-40;
  var layerHeight=parseFloat(getComputedStyle(document.documentElement).getPropertyValue('--mechanics-layer-height'))||54;
  return {x:x,y:top+layerHeight/2};
}

function blipParkMechanism(track, deploy) {
  var bar=document.querySelector('.top-marquee-bar');
  if (!bar) return null;
  var receiver=blipCardReceiver(bar), housing=receiver.getBoundingClientRect(), barBox=bar.getBoundingClientRect();
  var slot=receiver.querySelector('.jukebox-receiver-lip').getBoundingClientRect();
  var area=track ? track.getBoundingClientRect() : {left:0,width:innerWidth,top:110,bottom:440};
  return blipTransferCard({
    idle:true,deploy:!!deploy,name:'',code:'',art:'',width:134,height:258,
    reach:Math.hypot(housing.right+34-(area.left+area.width/2),area.bottom-housing.top)+60,
    source:{x:area.left+area.width/2,y:area.top+150},
    pivot:blipCardArmPivot(receiver),
    slot:{x:slot.left+slot.width/2,y:slot.top+slot.height/2},receiver:receiver
  });
}

function blipBuildGuideMechanism() {
  if (!/\/(about|controls|history|api)\.html$/.test(location.pathname)) return;
  document.documentElement.setAttribute('data-cabinet-mechanism','');
  var deploy=!!document.referrer && new URL(document.referrer).origin===location.origin;
  function park() {
    if(document.querySelector('.jukebox-transfer[data-deploying]')) return;
    blipParkMechanism(null,deploy); deploy=false;
  }
  requestAnimationFrame(park); window.addEventListener('resize',park);
}
if(document.readyState==='loading') document.addEventListener('DOMContentLoaded',blipBuildGuideMechanism);
else blipBuildGuideMechanism();

function blipArmJointSound() {
  try {
    var ctx=getKioskAudio(),output=ctx.createGain(),nodes=[],sources=[],motors=[];
    output.gain.value=1.5;output.connect(blipOut(ctx));nodes.push(output);
    [79,113].forEach(function(base){
      var motor=ctx.createOscillator(),tone=ctx.createBiquadFilter(),volume=ctx.createGain();
      motor.type='sawtooth';motor.frequency.value=base;
      tone.type='lowpass';tone.frequency.value=640;tone.Q.value=1.2;
      volume.gain.value=0;motor.connect(tone);tone.connect(volume);volume.connect(output);
      var cog=ctx.createOscillator(),teeth=ctx.createGain();
      cog.type='triangle';cog.frequency.value=19;teeth.gain.value=0;
      cog.connect(teeth);teeth.connect(volume.gain);
      motor.start();cog.start();sources.push(motor,cog);nodes.push(tone,volume,teeth);
      motors.push({motor: motor,tone:tone,volume:volume,cog:cog,teeth:teeth,base:base});
    });
    var buffer=ctx.createBuffer(1,ctx.sampleRate,ctx.sampleRate),data=buffer.getChannelData(0);
    for(var i=0;i<data.length;i++)data[i]=Math.random()*2-1;
    var air=ctx.createBufferSource(),valve=ctx.createBiquadFilter(),pressure=ctx.createGain();
    air.buffer=buffer;air.loop=true;valve.type='bandpass';valve.frequency.value=2400;valve.Q.value=.65;
    pressure.gain.value=0;air.connect(valve);valve.connect(pressure);pressure.connect(output);
    air.start();sources.push(air);nodes.push(valve,pressure);
    var stopped=false,timer=setTimeout(stop,18000);
    function stop(){
      if(stopped)return;stopped=true;clearTimeout(timer);window.removeEventListener('pagehide',stop);
      output.gain.setTargetAtTime(0,ctx.currentTime,.015);
      sources.forEach(function(source){source.stop(ctx.currentTime+.1);});
      setTimeout(function(){sources.concat(nodes).forEach(function(node){node.disconnect();});},150);
    }
    window.addEventListener('pagehide',stop,{once:true});
    return {stop:stop,update:function(shoulder,elbow,lift){
      if(stopped)return;
      var t=ctx.currentTime;
      [shoulder,elbow].forEach(function(speed,index){
        var voice=motors[index],motion=Math.min(1,speed/.9);
        voice.motor.frequency.setTargetAtTime(voice.base+motion*125,t,.04);
        voice.tone.frequency.setTargetAtTime(400+motion*1100,t,.04);
        voice.volume.gain.setTargetAtTime(motion*.036,t,.025);
        voice.teeth.gain.setTargetAtTime(motion*.012,t,.025);
        voice.cog.frequency.setTargetAtTime(16+motion*67,t,.04);
      });
      var flow=Math.min(1,lift/2.2);
      pressure.gain.setTargetAtTime(flow*.11,t,.025);
      valve.frequency.setTargetAtTime(1600+flow*2100,t,.04);
    }};
  }catch(e){return null;}
}

function playRomLoading(duration) {
  if(duration<=1.2)return null;
  try {
    var ctx=getKioskAudio(),start=ctx.currentTime+.1,end=start+duration,nodes=[],sources=[];
    var output=ctx.createGain(),tone=ctx.createBiquadFilter();
    output.gain.setValueAtTime(.0001,start);output.gain.exponentialRampToValueAtTime(.19,start+.18);
    output.gain.setValueAtTime(.19,end-.95);output.gain.exponentialRampToValueAtTime(.0001,end);
    tone.type='lowpass';tone.frequency.value=1900;tone.Q.value=.55;
    tone.connect(output);output.connect(blipOut(ctx));nodes.push(tone,output);
    var wave=ctx.createPeriodicWave(new Float32Array(6),new Float32Array([0,1,.36,.2,.12,.07]));
    var motor=ctx.createOscillator();motor.setPeriodicWave(wave);
    motor.frequency.setValueAtTime(90,start);motor.frequency.exponentialRampToValueAtTime(260,start+.7);
    motor.frequency.linearRampToValueAtTime(235,end-.95);motor.frequency.exponentialRampToValueAtTime(65,end);
    motor.connect(tone);sources.push(motor);
    var rotor=ctx.createOscillator(),ripple=ctx.createGain();rotor.type='sine';
    rotor.frequency.setValueAtTime(12,start);rotor.frequency.linearRampToValueAtTime(34,start+.7);
    rotor.frequency.linearRampToValueAtTime(9,end);ripple.gain.value=7;
    rotor.connect(ripple);ripple.connect(motor.frequency);sources.push(rotor);nodes.push(ripple);
    var buffer=ctx.createBuffer(1,ctx.sampleRate,ctx.sampleRate),data=buffer.getChannelData(0);
    for(var i=0;i<data.length;i++)data[i]=Math.random()*2-1;
    var air=ctx.createBufferSource(),filter=ctx.createBiquadFilter(),breath=ctx.createGain();
    air.buffer=buffer;air.loop=true;filter.type='bandpass';filter.frequency.value=2100;filter.Q.value=.6;breath.gain.value=.18;
    air.connect(filter);filter.connect(breath);breath.connect(output);sources.push(air);nodes.push(filter,breath);
    sources.forEach(function(source){source.start(start);source.stop(end+.02);});
    var stopped=false,timer=setTimeout(stop,(duration+.2)*1000);
    function stop(){
      if(stopped)return;stopped=true;clearTimeout(timer);window.removeEventListener('pagehide',stop);
      output.gain.cancelScheduledValues(ctx.currentTime);output.gain.setTargetAtTime(.0001,ctx.currentTime,.025);
      sources.forEach(function(source){try{source.stop(ctx.currentTime+.1);}catch(e){}});
      setTimeout(function(){sources.concat(nodes).forEach(function(node){node.disconnect();});},150);
    }
    window.addEventListener('pagehide',stop,{once:true});return {stop:stop};
  }catch(e){return null;}
}

function playRomCoupling(kind) {
  try {
    var ctx=getKioskAudio(),now=ctx.currentTime,isValve=kind==='valve'||kind==='vent';
    var offsets=isValve?[0]:kind==='attach'?[0,.045,.095]:kind==='release'?[0,.055]:[0,.025,.08];
    offsets.forEach(function(offset,index){
      var t=now+offset,duration=isValve?.22:.12;
      var buffer=ctx.createBuffer(1,Math.ceil(ctx.sampleRate*duration),ctx.sampleRate),data=buffer.getChannelData(0);
      for(var i=0;i<data.length;i++)data[i]=(Math.random()*2-1)*Math.exp(-i/(ctx.sampleRate*(isValve?.05:.008)));
      var noise=ctx.createBufferSource(),filter=ctx.createBiquadFilter(),snap=ctx.createGain();
      noise.buffer=buffer;filter.type='bandpass';filter.frequency.value=isValve?3300:kind==='release'?2100:1250+index*480;filter.Q.value=.8;
      snap.gain.value=isValve?.22:(kind==='attach'?.42:.23)/(1+index*.3);
      noise.connect(filter);filter.connect(snap);snap.connect(blipOut(ctx));noise.start(t);
      noise.onended=function(){noise.disconnect();filter.disconnect();snap.disconnect();};
      if(isValve)return;
      [kind==='attach'?96:kind==='seat'?94:146,327+index*91,713].forEach(function(frequency,partial){
        var metal=ctx.createOscillator(),ring=ctx.createGain();metal.type='sine';
        metal.frequency.setValueAtTime(frequency,t);metal.frequency.exponentialRampToValueAtTime(frequency*.82,t+.09);
        ring.gain.setValueAtTime((kind==='attach'?.28:.16)/(1+partial+index),t);ring.gain.exponentialRampToValueAtTime(.0001,t+.14);
        metal.connect(ring);ring.connect(blipOut(ctx));metal.start(t);metal.stop(t+.15);
        metal.onended=function(){metal.disconnect();ring.disconnect();};
      });
    });
  }catch(e){}
}

function blipCardDrive(duration) {
  try {
    var ctx=getKioskAudio(), out=ctx.createGain(), filter=ctx.createBiquadFilter();
    filter.type='lowpass'; filter.frequency.value=850;
    filter.connect(out); out.connect(blipOut(ctx)); out.gain.value=0;
    var motor=ctx.createOscillator(), harmonic=ctx.createOscillator();
    motor.type='sine'; harmonic.type='triangle';
    var overtone=ctx.createGain(); overtone.gain.value=.22;
    motor.connect(filter); harmonic.connect(overtone); overtone.connect(filter);
    var buffer=ctx.createBuffer(1,ctx.sampleRate*.4,ctx.sampleRate), data=buffer.getChannelData(0);
    for(var i=0;i<data.length;i++) data[i]=Math.random()*2-1;
    var friction=ctx.createBufferSource(), rub=ctx.createGain();
    friction.buffer=buffer; friction.loop=true; rub.gain.value=.12;
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
    setTimeout(stop,duration || 3500);
    return {update:function(progress){
      var speed=Math.sin(progress*Math.PI), now=ctx.currentTime;
      motor.frequency.setTargetAtTime(62+speed*74,now,.025);
      harmonic.frequency.setTargetAtTime(124+speed*148,now,.025);
      out.gain.setTargetAtTime(.04+.105*speed,now,.025);
    },stop:stop};
  } catch(e) {return null;}
}

function playArcadeDock(attached) {
  try {
    var ctx=getKioskAudio(),start=ctx.currentTime,output=ctx.createGain(),nodes=[output];
    output.gain.value=.85;output.connect(blipOut(ctx));
    // Both catches engage together; a short second strike is the latch's rebound.
    [0,attached ? .065 : .032].forEach(function(offset,index){
      var at=start+offset,buffer=ctx.createBuffer(1,Math.ceil(ctx.sampleRate*.09),ctx.sampleRate),data=buffer.getChannelData(0);
      for(var i=0;i<data.length;i++)data[i]=(Math.random()*2-1)*Math.exp(-i/(ctx.sampleRate*.009));
      var snap=ctx.createBufferSource(),filter=ctx.createBiquadFilter(),gain=ctx.createGain();
      snap.buffer=buffer;filter.type='highpass';filter.frequency.value=950;gain.gain.value=index?.28:.9;
      snap.connect(filter);filter.connect(gain);gain.connect(output);snap.start(at);nodes.push(snap,filter,gain);
      (attached ? [92,287,741] : [163,431,1187]).forEach(function(frequency,partial){
        var tone=ctx.createOscillator(),body=ctx.createGain();tone.type='triangle';tone.frequency.value=frequency;
        body.gain.setValueAtTime((index?.07:.18)/(partial+1),at);body.gain.exponentialRampToValueAtTime(.0001,at+.12);
        tone.connect(body);body.connect(output);tone.start(at);tone.stop(at+.14);nodes.push(tone,body);
      });
    });
    setTimeout(function(){nodes.forEach(function(node){node.disconnect();});},300);
  } catch(e) {}
}

function playCardLatch(guide) {
  try {
    var ctx=getKioskAudio(), t=ctx.currentTime;
    var buffer=ctx.createBuffer(1,Math.ceil(ctx.sampleRate*.06),ctx.sampleRate), data=buffer.getChannelData(0);
    for(var i=0;i<data.length;i++) data[i]=(Math.random()*2-1)*Math.exp(-i/(ctx.sampleRate*.012));
    var noise=ctx.createBufferSource(), filter=ctx.createBiquadFilter(), gain=ctx.createGain();
    noise.buffer=buffer; filter.type='bandpass'; filter.frequency.value=guide?1500:950; filter.Q.value=.7;
    gain.gain.value=guide ? .045 : .16;
    noise.connect(filter); filter.connect(gain); gain.connect(blipOut(ctx)); noise.start(t);
    (guide?[270]:[118,283,517]).forEach(function(frequency,index){
      var osc=ctx.createOscillator(), body=ctx.createGain(); osc.type='sine'; osc.frequency.value=frequency;
      var at=t+(index===0?0:.014);
      body.gain.setValueAtTime(guide ? .025 : .1/(index+1),at);
      body.gain.exponentialRampToValueAtTime(.0001,at+(guide ? .04 : .16));
      osc.connect(body);body.connect(blipOut(ctx));osc.start(at);osc.stop(at+.18);
    });
  } catch(e) {}
}

function blipSetMarquee(name, animate) {
  var sign=document.getElementById('marquee-name');
  if(!sign) return;
  var tiles=sign.querySelectorAll('.marquee-letter');
  sign.setAttribute('aria-label',name || 'No game selected');
  sign.classList.remove('marquee-engage');
  tiles.forEach(function(tile,index){
    tile.textContent=animate?'\u00a0':name[index] || '\u00a0';
    tile.style.setProperty('--flap-index',index);
  });
  if(animate) {
    void sign.offsetWidth; sign.classList.add('marquee-engage');
    Array.from(name).forEach(function(char,index){
      setTimeout(function(){tiles[index].textContent=char;playCardLatch(true);},index*190);
    });
  }
}

function blipBuildCabinetStars() {
  if (!document.body.classList.contains('cabinet-page') ||
      document.documentElement.hasAttribute('data-cabinet-screen')) return;
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

/* ---- The front-end switches ---- Home Screen and BLIP, on the top bar of the cabinet and of every game page:
 * beside the logo, else just left of the manual; where the bar has no room for the pair, stacked after the logo's
 * wire, with the game's name made small enough to fit what is left of its sign. */
(function () {
  function build() {
    var root = document.documentElement;
    if (!document.body.classList.contains('cabinet-page') || !document.querySelector('.blip-logo') ||
        root.hasAttribute('data-appstyle') || root.hasAttribute('data-cabinet-screen') || root.hasAttribute('data-amstrad')) return;
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
    var basic = switchButton('basic-switch-btn', 'home-switch basic-switch', 'Switch to BLIP', 'BLIP', 'B');
    button.addEventListener('click', function () { blipSetFrontend('home'); location.replace(up + 'home.html'); });
    basic.addEventListener('click', function () { blipSetFrontend('basic'); location.replace(up + 'basic.html'); });
    function shown(el) {
      if (!el) return null;
      var style = getComputedStyle(el), r = el.getBoundingClientRect();
      return r.width > 0 && style.visibility !== 'hidden' && style.display !== 'none' && Number(style.opacity) > 0.05 ? r : null;
    }
    // Stacked, the pair takes the sign's left end: the sign runs from the pair to the next part on the bar.
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
      // The sign is placed for as if shown: hidden, it would free its own space for the switches.
      var sign = document.querySelector('#marquee-name');
      if (sign) sign.style.removeProperty('visibility');
      var logo = document.querySelector('.blip-logo').getBoundingClientRect();
      var manual = shown(document.querySelector('.manual-pocket'));
      var blockers = ['#card-button', '.manual-pocket', '#marquee-name', '.blip-power-indicator']
        .map(function (q) { return shown(document.querySelector(q)); }).filter(Boolean);
      var size = button.offsetWidth, pair = size + 9 + basic.offsetWidth;
      var free = function (x) { return x > logo.right + 4 && x + pair < innerWidth - 8 && !blockers.some(function (b) { return x < b.right + 8 && x + pair > b.left - 8; }); };
      var left = [logo.right + 10, manual ? manual.left - 10 - pair : -1].filter(free)[0];
      var stacked = left === undefined, middle = (logo.top + logo.bottom) / 2;
      if (stacked) left = logo.right + 30;   // clear of the logo's power wire
      button.style.left = Math.round(left) + 'px';
      basic.style.left = Math.round(stacked ? left : left + size + 9) + 'px';
      button.style.top = stacked ? Math.round(middle - size - 3) + 'px' : '';
      basic.style.top = stacked ? Math.round(middle + 3) + 'px' : '';
      fitName(stacked ? Math.round(left + size + 6) : 0);
    }
    place();
    window.addEventListener('resize', place);
    window.addEventListener('load', place);
    if (document.fonts && document.fonts.ready) document.fonts.ready.then(place);
    if (typeof ResizeObserver === 'function') new ResizeObserver(place).observe(document.querySelector('.blip-logo'));
  }
  if (document.readyState === 'loading') document.addEventListener('DOMContentLoaded', build, { once: true });
  else build();
}());

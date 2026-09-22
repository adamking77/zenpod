// @ts-nocheck
// The Zen Futurism audio reader's Field, Thread, Orbit and tether, as pass 7 ported them (resting Orbit
// holds the episode's loudness; light flavors carry 1.6x ink). Copied from mockups/listener-pass7.html.

/* --accent is var(--blue) etc.; canvas needs the computed colour. */
function resolved(root, name){ const p = document.createElement('i'); p.style.color = `var(${name})`; p.style.display = 'none';
  root.appendChild(p); const c = getComputedStyle(p).color; p.remove(); return c; }

const smooth = (el, tc) => 1 - Math.exp(-el/tc);
export const SIG = { energy:0, presence:0, pulse:0, bands:new Float32Array(48), targets:new Float32Array(48), t:0 };
/* the player window's analyser, 30 times a second: { te, tp, bands } */
const FED = { te:0, tp:0, bands:new Float32Array(48), at:-1e9 };
export function feed(d){ FED.te = d.te; FED.tp = d.tp; FED.bands.set(d.bands); FED.at = performance.now(); }
function sampleField(p){ const n = 48, b = Math.max(0, Math.min(1, p))*(n-1), lo = Math.floor(b), hi = Math.min(n-1, lo+1), k = b-lo; return SIG.bands[lo]*(1-k) + SIG.bands[hi]*k; }
export function updateSignal(el, active, env){
  SIG.t += el/1000;
  let te = 0, tp = 0;
  const T = SIG.targets;
  if (active && performance.now() - FED.at < 250){
    te = FED.te; tp = FED.tp; T.set(FED.bands);
  } else if (active){
    const t = SIG.t, syl = .45 + .55*Math.abs(Math.sin(t*Math.PI*4.3 + Math.sin(t*1.7)*1.4));
    const breath = Math.sin(t*.63) < -.88 ? .15 : 1;
    te = Math.min(1, (.22 + env*.6)*syl*breath);
    tp = te*(.62 + .25*Math.sin(t*3.1));
    const f1 = .16 + .05*Math.sin(t*5.1), f2 = .42 + .09*Math.sin(t*3.7 + 1), f3 = .7 + .06*Math.sin(t*2.3);
    for (let i=0;i<48;i++){ const p = i/47, g = (c,w) => Math.exp(-((p-c)*(p-c))/(2*w*w));
      T[i] = Math.min(1, te*1.25*(g(f1,.07) + g(f2,.07)*.8 + g(f3,.09)*.5)*(.8 + .2*Math.sin(i*1.7 + t*9))); }
  } else T.fill(0);
  SIG.pulse = te;
  SIG.energy += (te - SIG.energy)*smooth(el, te > SIG.energy ? 42 : 138);
  SIG.presence += (tp - SIG.presence)*smooth(el, 96);
  for (let i=0;i<48;i++){ const sp = T[Math.max(0,i-1)]*.22 + T[i]*.56 + T[Math.min(47,i+1)]*.22; SIG.bands[i] += (sp - SIG.bands[i])*smooth(el, sp > SIG.bands[i] ? 56 : 175); }
}

export function makeViz(canvas, kind){ return { canvas, kind, ctx: canvas.getContext('2d'), thread:new Float32Array(3), phases:new Float32Array([.35,2.15,4.45]), tPulse:0, rays:new Float32Array(96), oPulse:0, oPhase:.35, last:0 }; }
let INK = '#3f3f48', SIGNAL = '#355aa0', INKK = 1;
export function resolveColors(){ const root = document.documentElement, cs = getComputedStyle(root); INK = cs.getPropertyValue('--text').trim() || INK; SIGNAL = resolved(root, '--accent') || SIGNAL;
  /* the component's alphas were tuned on dark grounds; light flavors carry 1.6x the ink */
  const b = (cs.getPropertyValue('--base').trim() || '#fff').replace('#',''), n = parseInt(b.length === 3 ? b.split('').map(x=>x+x).join('') : b, 16);
  const lum = (.299*(n>>16&255) + .587*(n>>8&255) + .114*(n&255))/255; INKK = lum > .5 ? 1.6 : 1; }

function samplePeaks(peaks, count){
  return Array.from({length:count}, (_, i) => { const a = Math.floor(i/count*peaks.length), b = Math.max(a+1, Math.floor((i+1)/count*peaks.length)); let m = 0; for (let k=a;k<b;k++) m = Math.max(m, peaks[k]||0); return m; });
}
export function drawField(c, w, h, center, progress, peaks){
  const count = Math.max(72, Math.floor(w/(w < 520 ? 4 : 4.8))), data = samplePeaks(peaks, count), gap = w/count;
  const datumX = Math.min(w-5, Math.max(5, progress*w)), win = w*(w < 520 ? .24 : .17), amps = [];
  c.strokeStyle = INK; c.lineWidth = 1;
  data.forEach((v, i) => {
    const x = (i+.5)*gap, wake = x <= datumX ? win*1.2 : win*.58, d = Math.abs(x-datumX)/wake, loc = Math.max(0, 1-d), le = loc*loc*(3-2*loc);
    const sp = sampleField((x - (datumX - win*1.2))/(win*1.78));
    const quiet = .024 + Math.pow(Math.max(.02, v), .82)*.07, act = le*(SIG.energy*.04 + sp*.14 + SIG.presence*.035);
    const a = Math.min(h*.43, Math.max(1.4, h*(quiet + act))); amps.push(a);
    c.globalAlpha = Math.min(1, ((x <= datumX ? .22 : .11) + le*.05)*INKK); c.beginPath(); c.moveTo(x, center-a); c.lineTo(x, center+a); c.stroke();
  });
  c.globalAlpha = .08*INKK; c.beginPath();
  amps.forEach((a,i) => { const x = (i+.5)*gap; i ? c.lineTo(x, center-a) : c.moveTo(x, center-a); });
  for (let i=amps.length-1;i>=0;i--) c.lineTo((i+.5)*gap, center+amps[i]);
  c.closePath(); c.stroke();
  c.globalAlpha = .1*INKK; c.beginPath(); c.moveTo(0, center); c.lineTo(w, center); c.stroke();
  c.strokeStyle = SIGNAL; c.globalAlpha = .82; c.lineWidth = 1.25; c.beginPath(); c.moveTo(0, center); c.lineTo(datumX, center); c.stroke();
  c.fillStyle = SIGNAL; c.globalAlpha = .9; c.beginPath(); c.arc(datumX, center, 4.2, 0, Math.PI*2); c.fill();
}
export function drawThread(V, c, w, h, center, progress, peaks, active, el){
  const datumX = Math.min(w-5, Math.max(5, progress*w)), win = w*(w < 520 ? .4 : .28);
  const env = peaks[Math.min(peaks.length-1, Math.floor(progress*peaks.length))] || 0;
  const body = (sampleField(.12)+sampleField(.22)+sampleField(.32))/3, voice = (sampleField(.4)+sampleField(.52)+sampleField(.64))/3, detail = (sampleField(.7)+sampleField(.82)+sampleField(.94))/3;
  const pt = active ? Math.pow(Math.min(1, SIG.pulse*1.1 + SIG.presence*.24), .62) : 0;
  V.tPulse += (pt - V.tPulse)*smooth(el, pt > V.tPulse ? 10 : 62);
  const E = SIG.energy, P = SIG.pulse, R = SIG.presence;
  const tg = active ? [Math.min(.94, E*.48+P*.22+body*.22+env*.34*.08), Math.min(.76, E*.28+P*.18+voice*.22+R*.12), Math.min(.58, E*.16+P*.14+detail*.18+R*.1)] : [0,0,0];
  for (let i=0;i<3;i++){ V.thread[i] += (tg[i]-V.thread[i])*smooth(el, tg[i] > V.thread[i] ? 18+i*6 : 72+i*12); if (active) V.phases[i] += el*.0016*(1+i*.23); }
  c.lineCap = 'round'; c.lineJoin = 'round'; c.lineWidth = 1; c.strokeStyle = INK; c.globalAlpha = .18*INKK;
  c.beginPath(); c.moveTo(0, center); c.lineTo(w, center); c.stroke();
  c.strokeStyle = SIGNAL; c.globalAlpha = .82; c.lineWidth = 1.25; c.beginPath(); c.moveTo(0, center); c.lineTo(datumX, center); c.stroke();
  const start = Math.max(0, datumX-win), end = Math.min(w, datumX+win), n = Math.max(80, Math.min(180, Math.floor((end-start)/2.2)));
  const att = v => Math.pow(4/(4+v*v), 4);
  const fam = [
    { color:SIGNAL, alpha:.8, lw:1.3, hh:1, comp:[[-1.55,2.65,1,1,0],[1.4,1.85,.78,-1,1.18],[.1,2.25,.58,1,2.38]] },
    { color:INK, alpha:.28, lw:.9, hh:.73, comp:[[-2.45,2.15,.86,-1,.72],[.45,1.65,1,1,2.05],[2.15,2.55,.66,-1,3.2]] },
    { color:INK, alpha:.12, lw:.7, hh:.5, comp:[[-2.8,2.8,.7,1,1.32],[-.35,1.45,1,-1,2.84],[2.65,2.05,.72,1,4.08]] } ];
  const boundary = (f, fi, sign) => {
    const s = V.thread[fi]; if (s < .002) return;
    const maxH = h*.41*f.hh, safe = h*.39 - f.lw, cs = safe*.76;
    c.strokeStyle = f.color; c.globalAlpha = Math.min(1, f.alpha*Math.min(1, .22 + s*1.16)*(f.color === INK ? INKK : 1)); c.lineWidth = f.lw; c.beginPath();
    for (let i=0;i<=n;i++){
      const nm = i/n, gp = (nm*2-1)*25, oe = att(gp/25*2), wp = gp + (Math.sin(gp*.37+fi*1.13)*.34 + Math.sin(gp*.91-fi*.67)*.13);
      let b = 0; f.comp.forEach(([off,wd,wt,vs,ph]) => { const lp = wp/wd - off; b += Math.abs(wt*Math.sin(vs*lp - V.phases[fi] - ph)*att(lp)); });
      b /= f.comp.length;
      const irr = 1 + Math.sin(gp*.47+fi*1.13)*.075 + Math.sin(gp*1.21-fi*.83)*.04 + Math.sin(gp*2.17+fi*.49)*.018;
      const bias = 1 + sign*(Math.sin(gp*.31+fi*.91)*.045 + Math.sin(gp*.83-fi*.57)*.025);
      b *= Math.max(.78, irr*bias);
      const x = start + nm*(end-start), taper = Math.pow(Math.max(0, Math.sin(nm*Math.PI)), 1.18);
      const woven = Math.sin(nm*Math.PI*2*(1.9+fi*.22) - V.phases[fi]*1.35 + fi*.82)*(h*[.06,.052,.042][fi]*Math.pow(s,.7)*taper*(.42 + V.tPulse*.95));
      const raw = woven - sign*(maxH*Math.pow(s,.72)*b*oe*2.55*(.48 + V.tPulse*.9)), rm = Math.abs(raw);
      const cm = rm <= cs ? rm : cs + (safe-cs)*(1 - Math.exp(-(rm-cs)/Math.max(1, safe-cs)));
      const y = center + Math.sign(raw)*cm; i ? c.lineTo(x, y) : c.moveTo(x, y);
    }
    c.stroke();
  };
  for (let fi=2; fi>=0; fi--){ boundary(fam[fi], fi, 1); boundary(fam[fi], fi, -1); }
  c.fillStyle = SIGNAL; c.globalAlpha = .94; c.beginPath(); c.arc(datumX, center, 4, 0, Math.PI*2); c.fill();
}
const circDist = (p, c) => { const d = Math.abs(p-c); return Math.min(d, 1-d); };
const cluster = p => { const g = (c,w,wt) => { const d = circDist(p,c); return Math.exp(-(d*d)/(2*w*w))*wt; }; return Math.min(1, g(.04,.082,.82)+g(.27,.052,.68)+g(.56,.115,.76)+g(.82,.067,.94)); };
export function drawOrbit(V, c, w, h, progress, peaks, active, el, opts = {}){
  const diameter = Math.max(1, Math.min(w, h)), cx = w/2, cy = h/2, small = opts.small;
  const inner = diameter*(opts.inner || .165), rayStart = inner + Math.max(4, diameter*.022), maxRay = diameter*(opts.maxRay || .295), n = opts.rays || 96;
  const pt = active ? Math.min(1, SIG.energy*.88 + SIG.presence*.44) : 0;
  V.oPulse += (pt - V.oPulse)*smooth(el, pt > V.oPulse ? 22 : 78);
  if (active) V.oPhase = (V.oPhase + el*.00265*(.82 + SIG.energy*.72 + V.oPulse*.55)) % (Math.PI*2);
  const env = peaks[Math.min(peaks.length-1, Math.floor(progress*peaks.length))] || 0;
  c.lineCap = 'round'; c.strokeStyle = INK; c.lineWidth = small ? .7 : .82;
  for (let i=0;i<n;i++){
    const p = i/n, ang = p*Math.PI*2 - Math.PI/2, bp = (p*1.72 + .08 + Math.sin(i*1.93 + V.oPhase*.42)*.032 + 1) % 1;
    const fine = (Math.sin(i*1.31+.35)+1)/2, cl = (.14 + cluster(p)*.86)*(.76 + fine*.24);
    /* at rest the rays hold the episode's shape (its loudness), played part in the accent; playing, the voice drives them */
    const rest = .04 + Math.pow(peaks[Math.min(peaks.length-1, Math.floor(p*peaks.length))] || 0, 1.6)*.42;
    const live = Math.min(1, (sampleField(bp)*.62 + SIG.energy*.29 + SIG.presence*.15 + env*.06)*cl*(.7 + V.oPulse*.55)*(.8 + Math.sin(V.oPhase + i*.36)*.2));
    const tgt = active ? Math.max(rest*.55, live) : rest;
    const ri = Math.min(95, Math.round(p*95));
    V.rays[ri] += (tgt - V.rays[ri])*smooth(el, tgt > V.rays[ri] ? 26 : 92);
    const lv = V.rays[ri], len = (small ? 1.3 : 2) + maxRay*Math.pow(lv, .68)*(.86 + V.oPulse*.24);
    const co = Math.cos(ang), si = Math.sin(ang);
    const played = p <= progress;
    c.strokeStyle = played ? SIGNAL : INK;
    c.globalAlpha = played ? Math.min(.95, .62 + lv*.4) : Math.min(1, (.13 + Math.min(.32, lv*(.32 + V.oPulse*.08)))*INKK);
    c.beginPath(); c.moveTo(cx+co*rayStart, cy+si*rayStart); c.lineTo(cx+co*(rayStart+len), cy+si*(rayStart+len)); c.stroke();
  }
  c.strokeStyle = INK; c.globalAlpha = .2*INKK; c.lineWidth = 1; c.beginPath(); c.arc(cx, cy, inner, 0, Math.PI*2); c.stroke();
  c.strokeStyle = SIGNAL; c.globalAlpha = .9; c.lineWidth = small ? 1.3 : 1.6; c.beginPath(); c.arc(cx, cy, inner, -Math.PI/2, -Math.PI/2 + Math.PI*2*progress); c.stroke();
}
/* the tether thread from the component's sticky control, used in the pill panel */
export function drawTether(V, c, w, h, center, progress, active){
  const inset = Math.min(9, w*.08), start = inset, end = w - inset, n = Math.max(40, Math.floor(end-start));
  const act = active ? Math.min(1, SIG.energy*.72 + SIG.presence*.42 + V.tPulse*.45) : .1;
  if (active) for (let i=0;i<3;i++) V.phases[i] += 16*.0016*(1+i*.23);
  c.strokeStyle = INK; c.globalAlpha = .12*INKK; c.lineWidth = 1; c.beginPath(); c.moveTo(start, center); c.lineTo(end, center); c.stroke();
  [[active ? SIGNAL : INK,.68,1.05,1],[INK,.25,.8,.72],[INK,.13,.65,.5]].forEach(([col,al,lw,sc], fi) => {
    c.strokeStyle = col; c.globalAlpha = Math.min(1, al*(col === INK ? INKK : 1)); c.lineWidth = lw; c.lineCap = 'round'; c.beginPath();
    for (let i=0;i<=n;i++){ const nm = i/n, wp = nm + Math.sin(nm*Math.PI*4.6 + fi*.87)*.012 + Math.sin(nm*Math.PI*9.4 - fi*.54)*.005;
      const tp = Math.pow(Math.max(0, Math.sin(nm*Math.PI)), 1.28), pv = .94 + Math.sin(nm*Math.PI*6.2 + fi*1.17)*.055 + Math.sin(nm*Math.PI*13.6 - fi*.71)*.025;
      const a = (center-4)*(.22 + act*.78)*sc*tp*pv, y = center + Math.sin(wp*Math.PI*2*(1.85+fi*.2) - V.phases[fi]*1.4 + fi*.82)*a, x = start + nm*(end-start);
      i ? c.lineTo(x, y) : c.moveTo(x, y); }
    c.stroke(); });
  const dx = start + progress*(end-start); c.fillStyle = SIGNAL; c.globalAlpha = .9; c.beginPath(); c.arc(dx, center, 2.6, 0, Math.PI*2); c.fill();
}


// Run: bun src/lib/plate.check.ts
// Every place a note's plate can land, at the smallest, default and a large window: it must stay inside the listening
// side, clear of the title block, off the shape (above the line, or outside the Orbit's rays), near its note, and
// centred in the space above the line when it fits.
import { EDGE, GAP, LIFT, MINW, RING_CLEAR, RUN, maxHeight, placePlate } from './plate';

const WINDOWS = [[920, 660], [1080, 700], [1600, 1000]];
const fails: string[] = [];
let cases = 0;

for (const [W, H] of WINDOWS) {
  const lib = Math.round(Math.max(300, Math.min(560, W - 620, 400)));
  const room = W - lib - 96, narrow = room < 700;
  const stageH = H - 64 - 44 - 136 - (narrow ? 98 : 56);
  for (const view of ['field', 'thread', 'orbit'] as const) {
    const ch = view === 'orbit' ? 340 : 280, cw = room, oy = (stageH - ch) / 2, ox = 0;
    const orbit = view === 'orbit' ? { cx: cw / 2, cy: ch / 2, rout: Math.min(cw, ch) * 0.49, ring: Math.min(cw, ch) * 0.165 } : null;
    for (const quote of [false, true]) for (const kind of ['empty', 'short', 'long', 'peek'] as const) for (let n = 0; n <= 20; n++) {
      const p = 0.005 + (n / 20) * 0.99;
      let x: number, y: number;
      if (orbit) { const a = p * Math.PI * 2 - Math.PI / 2, R = Math.min(cw, ch) * 0.165; x = ox + cw / 2 + Math.cos(a) * R; y = oy + ch / 2 + Math.sin(a) * R; }
      else { const i = Math.round(cw * 0.1); x = ox + i + p * (cw - 2 * i); y = oy + ch / 2; }
      const max = maxHeight({ y, stageH, orbit });
      // The plate's height as the component lays it out: label, quote, field, the line under it, fitted to `max`.
      const height = (w: number) => {
        const label = orbit && w < 300 ? 32 : 16;
        if (kind === 'peek') return label + 4 + 22;
        const chars = kind === 'long' ? 160 : kind === 'short' ? 18 : 0, lines = Math.min(3, Math.max(1, Math.ceil((chars * 7.4) / w)));
        const fixed = label + 6 + 6 + 8 + 20;
        let q = quote ? 44 + 6 : 0;
        if (fixed + q + 22 * lines > max && quote) q = 22 + 6;
        return fixed + q + Math.max(22, Math.min(22 * lines, max - fixed - q));
      };
      const out = placePlate({ x, y, room, stageH, ox, oy, orbit, height });
      const B = { l: out.left, t: out.top, r: out.left + out.width, b: out.top + out.height }, bad: string[] = [];
      cases++;
      if (B.l < -0.5 || B.r > room + 0.5) bad.push(`leaves the listening side (${Math.round(B.l)}–${Math.round(B.r)} of ${room})`);
      if (B.t < EDGE - 0.5) bad.push(`within ${EDGE}px of the title block (top ${Math.round(B.t)})`);
      if (B.b > stageH + 0.5) bad.push(`runs below the stage (${Math.round(B.b)} of ${stageH})`);
      if (orbit) {
        const cx = ox + orbit.cx, cy = oy + orbit.cy, nx = Math.max(B.l, Math.min(cx, B.r)), ny = Math.max(B.t, Math.min(cy, B.b)), d = Math.hypot(nx - cx, ny - cy);
        const west = x < cx, roomBeside = west ? cx - orbit.rout - EDGE - GAP : room - (cx + orbit.rout + EDGE + GAP);
        // Beside the rays whenever there's room for a plate there; never over the ring the notes sit on.
        if (roomBeside >= MINW && d < orbit.rout) bad.push('over the rays although there was room beside them');
        if (d < orbit.ring + RING_CLEAR - 0.5) bad.push(`over the ring the notes sit on (${Math.round(d)}px from the centre)`);
        if (west ? B.l > x : B.r < x) bad.push('on the far side from its note');
      } else {
        if (B.b > y - LIFT + 0.5) bad.push(`over the shape (ends ${Math.round(B.b)}, line ${Math.round(y)})`);
        const band = y - LIFT - EDGE, mid = EDGE + band / 2;
        if (out.height < band && Math.abs((B.t + B.b) / 2 - mid) > 1) bad.push(`not centred above its line (${Math.round((B.t + B.b) / 2)} vs ${Math.round(mid)})`);
        const dist = x < B.l ? B.l - x : x > B.r ? x - B.r : 0;
        if (dist > RUN + GAP + 0.5) bad.push(`${Math.round(dist)}px from its note`);
      }
      if (bad.length) fails.push(`${W}×${H} ${view} ${quote ? 'quote' : 'no quote'} ${kind} ${n * 5}%: ${bad.join('; ')}`);
    }
  }
}

if (fails.length) { console.error(fails.slice(0, 15).join('\n')); throw new Error(`${fails.length} of ${cases} plate positions break a rule`); }
console.log(`plate placement ok: ${cases} positions`);

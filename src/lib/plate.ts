// Where a note's plate sits. The shape never moves for it.
//
// Field and Thread: above its note, centred in the open space between the title block (the stage's top edge) and the
// line, never closer than EDGE to the one or LIFT to the other. The leader rises from the ring to the label's centre
// line and turns RUN px; the plate starts GAP px later, to the right unless the left has more room, and narrows (to
// MINW) rather than leave the listening side.
//
// Orbit: beside the rays, on the note's side, EDGE outside them; the label's centre line level with the note where the
// stage allows. When the window leaves too little room beside the rays, the plate sits over the outer rays on the
// note's side instead, never over the ring the notes sit on, and the rays behind its words fade as the Field's bars do.
// The leader runs flat from the note to the plate, with one vertical step if the plate had to move.

export const EDGE = 12, RUN = 14, GAP = 8, LIFT = 22, MAXW = 380, MINW = 200, LABEL = 7;

export type PlateIn = {
  /** The note's ring, in stage pixels. */
  x: number; y: number;
  /** The stage's size, and where the canvas sits in it. */
  room: number; stageH: number; ox: number; oy: number;
  /** The Orbit's centre and outer reach, in canvas pixels, when the shape is an Orbit. */
  orbit: { cx: number; cy: number; rout: number; ring: number } | null;
  /** The plate's height at a given width. */
  height: (width: number) => number;
};

export type PlateOut = { left: number; top: number; width: number; height: number; side: boolean; leader: string; clear: { x: number; y: number; w: number; h: number } | null };

/** The tallest the plate may be at this note: the open space above the line, or the stage beside the Orbit. The plate
 * fits itself to it (a one-line quote, a note that scrolls inside its field) rather than cover the shape. */
export const maxHeight = (p: Pick<PlateIn, 'y' | 'stageH' | 'orbit'>) => (p.orbit ? p.stageH - 2 * EDGE : p.y - LIFT - EDGE);

const clampW = (room: number, min = MINW) => Math.round(Math.max(min, Math.min(MAXW, room)));
/** How far the plate keeps from the ring the notes sit on, past the ring's own radius. */
export const RING_CLEAR = 16;

export function placePlate(p: PlateIn): PlateOut {
  const { x, y, room } = p;
  if (p.orbit) {
    const cx = p.ox + p.orbit.cx, west = x < cx;
    const beside = west ? cx - p.orbit.rout - EDGE - GAP : room - (cx + p.orbit.rout + EDGE + GAP);
    const over = beside < MINW;
    // Beside the rays when they leave room; otherwise over them, stopping short of the ring.
    const edge = over ? (west ? cx - p.orbit.ring - RING_CLEAR : cx + p.orbit.ring + RING_CLEAR) : west ? cx - p.orbit.rout - EDGE : cx + p.orbit.rout + EDGE;
    const gap = over ? 0 : GAP;
    const width = clampW(west ? edge - gap : room - edge - gap, over ? 150 : MINW), height = p.height(width);
    const left = west ? edge - gap - width : edge + gap;
    const top = Math.max(EDGE, Math.min(p.stageH - height, y - LABEL)), ly = top + LABEL;
    const leader = `M${x + (west ? -8 : 8)} ${y} H${edge}${Math.abs(ly - y) > 0.5 ? ` V${ly}` : ''}`;
    const clear = over ? { x: left - p.ox - 10, y: top - p.oy - 8, w: width + 20, h: height + 12 } : null;
    return { left, top, width, height, side: true, leader, clear };
  }
  const roomR = room - (x + RUN + GAP), roomL = x - RUN - GAP, right = roomR >= MAXW || roomR >= roomL;
  const width = clampW(right ? roomR : roomL), height = p.height(width);
  const bendX = right ? x + RUN : x - RUN;
  const left = Math.max(0, Math.min(room - width, right ? bendX + GAP : bendX - GAP - width));
  const top = EDGE + Math.max(0, (y - LIFT - EDGE - height) / 2);
  const leader = `M${x} ${y - 8} V${top + LABEL} H${bendX}`;
  return { left, top, width, height, side: false, leader, clear: { x: left - p.ox - 10, y: top - p.oy - 8, w: width + 20, h: height + 12 } };
}

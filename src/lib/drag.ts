import { getCurrentWindow, LogicalPosition } from '@tauri-apps/api/window';

// Controls keep their clicks; anything marked data-drag (the Pill's title) can also carry the window.
const CONTROLS = 'button:not([data-drag]), input, a, label, select, textarea, .cue, [data-no-drag]';

/**
 * Move the window by dragging anywhere that isn't a control. The window follows the pointer once it has
 * travelled a few pixels, and the click that would follow is swallowed, so a drag never seeks, plays or opens.
 * (macOS's own window drag has to start on the press itself, which would eat every plain click too.)
 */
export function dragWindow(node: HTMLElement, enabled = true) {
  if (!enabled) return;
  const win = getCurrentWindow();
  // Where the window is, kept current, so a drag can start on the very first move.
  let pos = { x: 0, y: 0 }, k = 1;
  const track = async () => { k = await win.scaleFactor(); const p = await win.outerPosition(); pos = { x: p.x / k, y: p.y / k }; };
  track();
  const moved = win.onMoved((e) => { pos = { x: e.payload.x / k, y: e.payload.y / k }; });
  let from: { x: number; y: number; wx: number; wy: number } | null = null;
  let dragged = false;
  const down = (e: PointerEvent) => {
    if (e.button !== 0 || (e.target as Element).closest(CONTROLS)) return;
    dragged = false;
    from = { x: e.screenX, y: e.screenY, wx: pos.x, wy: pos.y };
  };
  const move = (e: PointerEvent) => {
    if (!from) return;
    const dx = e.screenX - from.x, dy = e.screenY - from.y;
    if (!dragged && Math.hypot(dx, dy) < 4) return;
    // Capture only once it's a drag: capturing on the press would retarget a plain click away from the canvas.
    if (!dragged) node.setPointerCapture(e.pointerId);
    dragged = true;
    win.setPosition(new LogicalPosition(from.wx + dx, from.wy + dy));
  };
  const up = () => { from = null; };
  const click = (e: MouseEvent) => {
    if (dragged) { e.stopPropagation(); e.preventDefault(); dragged = false; }
  };
  node.addEventListener('pointerdown', down);
  node.addEventListener('pointermove', move);
  node.addEventListener('pointerup', up);
  node.addEventListener('pointercancel', up);
  node.addEventListener('click', click, true);
  return {
    destroy() {
      moved.then((f) => f());
      node.removeEventListener('pointerdown', down);
      node.removeEventListener('pointermove', move);
      node.removeEventListener('pointerup', up);
      node.removeEventListener('pointercancel', up);
      node.removeEventListener('click', click, true);
    },
  };
}

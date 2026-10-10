<script lang="ts">
  import { onMount } from 'svelte';
  import Library from '$lib/Library.svelte';
  import Listen from '$lib/Listen.svelte';
  import { loadPrefs, prefs, setPref } from '$lib/prefs.svelte';
  import { dragWindow } from '$lib/drag';
  import { arrivals, follow, keys, now, player } from '$lib/now.svelte';
  import { followMarks } from '$lib/marks.svelte';
  import { followReads, openRead, take, takeFile } from '$lib/reads.svelte';
  import { ui } from '$lib/ui.svelte';
  import { getCurrentWebview } from '@tauri-apps/api/webview';

  // The library column: drag its edge to make it narrower or wider; the listening side keeps room for its controls.
  let drag = $state<number | null>(null), vw = $state(1080);
  const clampLib = (w: number) => Math.round(Math.max(300, Math.min(560, vw - 620, w)));
  const lib = $derived(clampLib(drag ?? (Number(prefs.libw) || 400)));
  function resize(e: PointerEvent) {
    const el = e.currentTarget as HTMLElement;
    el.setPointerCapture(e.pointerId);
    const move = (m: PointerEvent) => (drag = clampLib(window.innerWidth - m.clientX));
    const up = () => { el.removeEventListener('pointermove', move); if (drag != null) setPref('libw', String(drag)); drag = null; };
    el.addEventListener('pointermove', move);
    el.addEventListener('pointerup', up, { once: true });
  }

  onMount(() => {
    loadPrefs();
    const un = follow(), arr = arrivals('win'), mk = followMarks(), rd = followReads();
    // Files dragged in arrive through the window (with their paths); links and text through the page's own drop.
    const dd = getCurrentWebview().onDragDropEvent((e) => {
      const p = e.payload;
      if (p.type === 'enter') { if (p.paths.length) ui.dropping = true; }
      else if (p.type === 'leave') ui.dropping = false;
      else if (p.type === 'drop') { ui.dropping = false; if (p.paths[0]) takeFile(p.paths[0]); }
    });
    return () => { un.then((f) => f()); arr.then((f) => f()); mk.then((f) => f()); rd.then((f) => f()); dd.then((f) => f()); };
  });

  const typing = (t: EventTarget | null) => t instanceof HTMLInputElement || t instanceof HTMLTextAreaElement || t instanceof HTMLSelectElement;
  // The player's keys everywhere, and R for Read to me here in the window.
  function key(e: KeyboardEvent) {
    keys(e);
    if ((e.key === 'r' || e.key === 'R') && !typing(e.target) && !e.metaKey && !e.ctrlKey && !e.altKey && !e.repeat) { e.preventDefault(); openRead(); }
  }
  // ⌘V with nothing to type into: a link or text goes to Read to me.
  function paste(e: ClipboardEvent) {
    if (typing(e.target)) return;
    const t = e.clipboardData?.getData('text/plain')?.trim();
    if (t) { e.preventDefault(); take(t); }
  }
  let depth = 0;
  const linkDrag = (e: DragEvent) => !!e.dataTransfer && !e.dataTransfer.types.includes('Files') && (e.dataTransfer.types.includes('text/uri-list') || e.dataTransfer.types.includes('text/plain'));
  function dragenter(e: DragEvent) { if (!linkDrag(e)) return; e.preventDefault(); depth++; ui.dropping = true; }
  function dragover(e: DragEvent) { if (!linkDrag(e)) return; e.preventDefault(); e.dataTransfer!.dropEffect = 'copy'; }
  function dragleave(e: DragEvent) { if (!linkDrag(e)) return; if (--depth <= 0) { depth = 0; ui.dropping = false; } }
  function drop(e: DragEvent) {
    if (!linkDrag(e)) return;
    e.preventDefault(); depth = 0; ui.dropping = false;
    const d = e.dataTransfer!, uri = d.getData('text/uri-list').split('\n').map((l) => l.trim()).find((l) => l && !l.startsWith('#'));
    take(uri || d.getData('text/plain'));
  }

</script>

<svelte:window onkeydown={key} onpaste={paste} ondragenter={dragenter} ondragover={dragover} ondragleave={dragleave} ondrop={drop} bind:innerWidth={vw} />

<main class="win" use:dragWindow>
  <div class="panes" style:--lib="{lib}px">
    <div class="edge" role="separator" aria-orientation="vertical" aria-label="Library width" data-no-drag onpointerdown={resize}
      ondblclick={() => setPref('libw', '400')}></div>
    <Listen />
    <Library current={now.episode?.id ?? null} onplay={(e, list) => player.choose(e.id, list.map((x) => x.id))} />
  </div>
</main>

<style>
  .win { position: relative; height: 100vh; overflow: hidden; }
  /* the light: cast from the cover's centre in the show's colour */
  .win::before {
    content: ""; position: absolute; inset: 0; pointer-events: none; transition: opacity 0.4s var(--ease);
    background: radial-gradient(var(--glow) calc(var(--glow) * 0.8) at 116px 132px,
      color-mix(in srgb, var(--cast) var(--cast-pct), transparent), transparent 62%);
  }
  :global(:root[data-light="flat"]) .win::before { opacity: 0; }
  .panes { position: relative; display: grid; grid-template-columns: minmax(0, 1fr) var(--lib); height: 100%; }
  .edge { position: absolute; top: 0; bottom: 0; right: var(--lib); width: 9px; margin-right: -4px; cursor: col-resize; z-index: 3; }
  .edge:hover { background: linear-gradient(90deg, transparent 4px, var(--accent) 4px 5px, transparent 5px); opacity: 0.6; }
</style>

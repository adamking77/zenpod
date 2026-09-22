<script lang="ts">
  import { onMount } from 'svelte';
  import Library from '$lib/Library.svelte';
  import Listen from '$lib/Listen.svelte';
  import { loadPrefs, prefs, setPref } from '$lib/prefs.svelte';
  import { dragWindow } from '$lib/drag';
  import { arrivals, follow, keys, now, player } from '$lib/now.svelte';

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
    const un = follow(), arr = arrivals('win');
    return () => { un.then((f) => f()); arr.then((f) => f()); };
  });

</script>

<svelte:window onkeydown={keys} bind:innerWidth={vw} />

<main class="win" use:dragWindow>
  <div class="panes" style:--lib="{lib}px">
    <div class="edge" role="separator" aria-orientation="vertical" aria-label="Library width" data-no-drag onpointerdown={resize}
      ondblclick={() => setPref('libw', '400')}></div>
    <Listen />
    <Library current={now.episode?.id ?? null} onplay={(e) => player.choose(e.id)} />
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

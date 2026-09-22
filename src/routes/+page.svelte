<script lang="ts">
  import { onMount } from 'svelte';
  import Library from '$lib/Library.svelte';
  import Listen from '$lib/Listen.svelte';
  import { loadPrefs } from '$lib/prefs.svelte';
  import { follow, now, player } from '$lib/now.svelte';

  onMount(() => {
    loadPrefs();
    const un = follow();
    return () => { un.then((f) => f()); };
  });

  function keys(e: KeyboardEvent) {
    if (e.target instanceof HTMLInputElement || e.metaKey || e.ctrlKey) return;
    // Space on a focused button presses that button; anywhere else it plays and pauses.
    if (e.code === 'Space' && !(e.target instanceof HTMLButtonElement)) { e.preventDefault(); player.toggle(); }
    if (e.key === 'ArrowLeft') player.skip(-15);
    if (e.key === 'ArrowRight') player.skip(30);
  }
</script>

<svelte:window onkeydown={keys} />

<main class="win">
  <div class="drag" data-tauri-drag-region></div>
  <div class="panes">
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
  .drag { position: absolute; inset: 0 0 auto 0; height: 44px; z-index: 1; }
  .panes { position: relative; display: grid; grid-template-columns: minmax(0, 1fr) 400px; height: 100%; }
</style>

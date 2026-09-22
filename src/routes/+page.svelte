<script lang="ts">
  import { onMount } from 'svelte';
  import Library from '$lib/Library.svelte';
  import Listen from '$lib/Listen.svelte';
  import { loadPrefs } from '$lib/prefs.svelte';
  import { arrivals, follow, keys, now, player } from '$lib/now.svelte';

  onMount(() => {
    loadPrefs();
    const un = follow(), arr = arrivals('win');
    return () => { un.then((f) => f()); arr.then((f) => f()); };
  });

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

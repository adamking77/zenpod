<script lang="ts">
  import { onMount } from 'svelte';
  import Library from '$lib/Library.svelte';
  import { loadPrefs } from '$lib/prefs.svelte';
  import type { Episode } from '$lib/api';

  let current = $state<number | null>(null);

  onMount(() => { loadPrefs(); });

  function play(e: Episode) { current = e.id; }
</script>

<main class="win">
  <div class="drag" data-tauri-drag-region></div>
  <div class="panes">
    <section class="listen" aria-label="Listening"></section>
    <Library {current} onplay={play} />
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
  .panes { position: relative; display: grid; grid-template-columns: 1fr 400px; height: 100%; }
  .listen { padding: 64px 48px 44px; }
</style>

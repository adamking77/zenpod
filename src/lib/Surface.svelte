<script lang="ts">
  // Shared frame for the floating surfaces: follows state, settings and the artwork's light.
  import { onMount, type Snippet } from 'svelte';
  import { loadPrefs } from '$lib/prefs.svelte';
  import { arrivals, follow, keys, now } from '$lib/now.svelte';
  import { castFrom } from '$lib/cast';
  import { dragPanel } from '$lib/drag';

  let { me, radius, drag = true, children }: { me: string; radius: number; drag?: boolean; children: Snippet } = $props();

  onMount(() => {
    loadPrefs();
    const a = follow(), b = arrivals(me);
    return () => { a.then((f) => f()); b.then((f) => f()); };
  });
  $effect(() => { if (now.episode) castFrom(now.episode.show_id); });
</script>

<svelte:window onkeydown={keys} />
<div class="surface" style:border-radius="{radius}px" use:dragPanel={drag}>{@render children()}</div>

<style>
  :global(html), :global(body) { background: transparent !important; }
  .surface { position: fixed; inset: 0; overflow: hidden;
    background: var(--surface); background: color-mix(in srgb, var(--surface) 62%, transparent);
    box-shadow: inset 0 0 0 0.5px var(--hair); }
</style>

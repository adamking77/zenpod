<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import { listen } from '@tauri-apps/api/event';
  import { onMount } from 'svelte';
  import Surface from '$lib/Surface.svelte';
  import Signal from '$lib/Signal.svelte';
  import { fmt } from '$lib/api';
  import { now, player, position } from '$lib/now.svelte';
  import { I } from '$lib/icons';

  let t = $state(0), open = $state(false);
  onMount(() => {
    let r = 0; const f = () => { t = position(); r = requestAnimationFrame(f); }; f();
    const un = listen<boolean>('panel', (e) => (open = e.payload));
    return () => { cancelAnimationFrame(r); un.then((g) => g()); };
  });
  const left = $derived(now.duration ? `${Math.max(1, Math.round((now.duration - t) / 60))} min left` : '');
</script>

<Surface me="pill" radius={24}>
  <div class="pill">
    <span class="po">
      {#if !open}<Signal variant="pill" />{/if}
      {#if now.episode}<span class="disc"><img src="listener://localhost/art/{now.episode.show_id}" alt="" /></span>{/if}
    </span>
    <button class="who" data-drag aria-label="Open the pill panel" aria-expanded={open} onclick={() => invoke('pill_panel', { open: !open })}>
      <span class="t">{now.episode?.title ?? 'The room is quiet.'}</span>
      <span class="num">{now.episode ? `${fmt(t)} · ${left}` : ''}</span>
    </button>
    <button class="pp" aria-label={now.playing ? 'Pause' : 'Play'} onclick={player.toggle}>{@html now.playing ? I.pause : I.play}</button>
  </div>
</Surface>

<style>
  .pill { display: flex; align-items: center; gap: 10px; height: 48px; padding: 0 8px 0 2px; }
  .po { position: relative; width: 46px; height: 46px; flex: none; }
  .disc { position: absolute; left: 50%; top: 50%; width: 13px; height: 13px; transform: translate(-50%, -50%); border-radius: 50%; overflow: hidden; }
  .disc img { width: 100%; height: 100%; object-fit: cover; display: block; }
  .who { flex: 1; min-width: 0; display: grid; line-height: 1.25; text-align: left; }
  .who .t { font-size: 13px; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .pp { width: 32px; height: 32px; border-radius: 50%; display: grid; place-items: center; flex: none; color: var(--text); }
  .pp:hover { background: var(--hair); }
</style>

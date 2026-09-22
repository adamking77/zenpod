<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import Surface from '$lib/Surface.svelte';
  import Signal from '$lib/Signal.svelte';
  import { short } from '$lib/api';
  import { goTo, now, player } from '$lib/now.svelte';
  import { I } from '$lib/icons';

  const close = () => invoke('pill_panel', { open: false });
  const to = (m: 'win' | 'mini') => { close(); goTo(m); };
</script>

<svelte:window onkeydown={(e) => e.key === 'Escape' && close()} />

<Surface me="panel" radius={18}>
  <div class="panel">
    {#if now.episode}
      <div class="top">
        <span class="cover"><img src="listener://localhost/art/{now.episode.show_id}" alt="" /></span>
        <div class="who"><div class="t">{now.episode.title}</div><span class="sub">{short(now.episode.show_title)}</span></div>
      </div>
      <div class="tether"><Signal variant="tether" /></div>
    {/if}
    <div class="ctl">
      <button class="ico" aria-label="Open the full window" onclick={() => to('win')}>{@html I.win}</button>
      <button class="ico" aria-label="Back 15 seconds" onclick={() => player.skip(-15)}>{@html I.back}</button>
      <button class="play" aria-label={now.playing ? 'Pause' : 'Play'} onclick={player.toggle}>{@html now.playing ? I.pause : I.play}</button>
      <button class="ico" aria-label="Forward 30 seconds" onclick={() => player.skip(30)}>{@html I.fwd}</button>
      <button class="ico" aria-label="Open the mini player" onclick={() => to('mini')}>{@html I.mini}</button>
    </div>
  </div>
</Surface>

<style>
  .panel { padding: 18px 20px; display: grid; gap: 12px; }
  .top { display: grid; grid-template-columns: 44px minmax(0, 1fr); gap: 12px; align-items: center; }
  .cover { display: block; width: 44px; height: 44px; border-radius: 7px; overflow: hidden; }
  .cover img { width: 100%; height: 100%; object-fit: cover; display: block; }
  .who { min-width: 0; }
  .t { font-size: 14.5px; line-height: 1.3; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .tether { height: 34px; }
  .ctl { display: flex; align-items: center; justify-content: center; gap: 24px; }
  .ico { color: var(--text-dim); display: grid; place-items: center; width: 28px; height: 28px; transition: color 0.14s ease; }
  .ico:hover { color: var(--text); }
  .play { width: 40px; height: 40px; border-radius: 50%; display: grid; place-items: center; box-shadow: inset 0 0 0 1px var(--line); color: var(--text); }
  .play:hover { box-shadow: inset 0 0 0 1px var(--accent); }
</style>

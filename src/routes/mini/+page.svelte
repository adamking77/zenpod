<script lang="ts">
  import Surface from '$lib/Surface.svelte';
  import Signal from '$lib/Signal.svelte';
  import { fmt, short } from '$lib/api';
  import { goTo, now, player, position } from '$lib/now.svelte';
  import { I } from '$lib/icons';
  import { onMount } from 'svelte';

  let t = $state(0);
  onMount(() => { let r = 0; const f = () => { t = position(); r = requestAnimationFrame(f); }; f(); return () => cancelAnimationFrame(r); });
</script>

<Surface me="mini" radius={18}>
  <div class="mini">
    <div class="head">
      <button class="ico" aria-label="Open the full window" onclick={() => goTo('win')}>{@html I.win}</button>
      <button class="ico" aria-label="Fold into the pill" onclick={() => goTo('pill')}>{@html I.pill}</button>
    </div>
    {#if now.episode}
      <div class="orb">
        <Signal variant="mini" />
        <span class="disc"><img src="listener://localhost/art/{now.episode.show_id}" alt="" /></span>
      </div>
      <p class="t">{now.episode.title}</p>
      <p class="s sub">{short(now.episode.show_title)}</p>
      <div class="tp">
        <span class="num">{fmt(t)}</span>
        <span class="ctl">
          <button class="ico" aria-label="Back 15 seconds" onclick={() => player.skip(-15)}>{@html I.back}</button>
          <button class="play" aria-label={now.playing ? 'Pause' : 'Play'} onclick={player.toggle}>{@html now.playing ? I.pause : I.play}</button>
          <button class="ico" aria-label="Forward 30 seconds" onclick={() => player.skip(30)}>{@html I.fwd}</button>
        </span>
        <span class="num end">{now.duration ? `−${Math.max(1, Math.round((now.duration - t) / 60))} min` : ''}</span>
      </div>
    {:else}
      <p class="quiet">The room is quiet.</p>
    {/if}
  </div>
</Surface>

<style>
  .mini { position: relative; height: 100%; padding: 14px 20px 18px; box-sizing: border-box; }
  .mini::before { content: ""; position: absolute; inset: 0; pointer-events: none; transition: opacity 0.4s var(--ease);
    background: radial-gradient(240px 220px at 50% 128px, color-mix(in srgb, var(--cast) var(--cast-pct), transparent), transparent 70%); }
  :global(:root[data-light="flat"]) .mini::before { opacity: 0; }
  .head { position: absolute; top: 10px; right: 10px; display: flex; z-index: 2; opacity: 0; transition: opacity 0.2s var(--ease); }
  .mini:hover .head, .mini:focus-within .head { opacity: 1; }
  .orb { position: relative; width: 240px; height: 240px; margin: 0 auto; }
  .disc { position: absolute; left: 50%; top: 50%; width: 72px; height: 72px; transform: translate(-50%, -50%); border-radius: 50%; overflow: hidden; pointer-events: none; }
  .disc img { width: 100%; height: 100%; object-fit: cover; display: block; }
  .t { position: relative; text-align: center; font-weight: 300; font-size: 16.5px; line-height: 1.3; margin: 2px 0 2px; text-wrap: balance;
    display: -webkit-box; -webkit-line-clamp: 2; line-clamp: 2; -webkit-box-orient: vertical; overflow: hidden; }
  .s { position: relative; text-align: center; margin: 0; }
  .tp { position: relative; display: grid; grid-template-columns: 1fr auto 1fr; align-items: center; gap: 10px; margin-top: 14px; }
  .end { justify-self: end; }
  .ctl { display: flex; align-items: center; gap: 18px; }
  .ico { color: var(--text-dim); display: grid; place-items: center; width: 28px; height: 28px; transition: color 0.14s ease; }
  .ico:hover { color: var(--text); }
  .play { width: 42px; height: 42px; border-radius: 50%; display: grid; place-items: center; box-shadow: inset 0 0 0 1px var(--line); color: var(--text); transition: box-shadow 0.14s ease; }
  .play:hover { box-shadow: inset 0 0 0 1px var(--accent); }
  .quiet { color: var(--text-dim); text-align: center; margin-top: 160px; }
</style>

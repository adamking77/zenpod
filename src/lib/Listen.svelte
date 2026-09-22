<script lang="ts">
  import { onMount } from 'svelte';
  import { fmt } from '$lib/api';
  import { chapterAt, loadChapters, now, player, position } from '$lib/now.svelte';
  import { prefs, setPref } from '$lib/prefs.svelte';
  import { ui } from '$lib/ui.svelte';
  import Signal from '$lib/Signal.svelte';
  import { castFrom } from '$lib/cast';

  const MODES = ['field', 'thread', 'orbit'] as const;
  type Mode = (typeof MODES)[number];
  const mode = $derived<Mode>(MODES.includes(prefs.viz as Mode) ? (prefs.viz as Mode) : 'field');

  $effect(() => { if (now.episode) castFrom(now.episode.show_id); });

  const SPEEDS = [1, 1.2, 1.4, 1.6, 2];
  let t = $state(0);

  $effect(() => { const id = now.episode?.id; if (id) loadChapters(id); });
  const chapter = $derived(chapterAt(t));

  onMount(() => {
    let raf = 0;
    const tick = () => { t = position(); raf = requestAnimationFrame(tick); };
    tick();
    return () => cancelAnimationFrame(raf);
  });

  const left = $derived(now.duration ? `${Math.max(1, Math.round((now.duration - t) / 60))} min left` : '');
  const nextSpeed = () => player.speed(SPEEDS[(SPEEDS.indexOf(now.speed) + 1) % SPEEDS.length] ?? 1);
</script>

<section class="listen" aria-label="Listening">
  {#if now.episode}
    {@const e = now.episode}
    <div class="now">
      <span class="cover"><img src="listener://localhost/art/{e.show_id}" alt="" /></span>
      <div class="who">
        <button class="title" aria-pressed={ui.notes} onclick={() => (ui.notes = !ui.notes)} title="Show notes">{e.title}</button>
        <span class="sub">{e.show_title}{#if chapter?.title} · {chapter.title}{/if}</span>
      </div>
    </div>
    <div class="stage"><Signal {mode} /></div>
    <div class="tp">
      <span class="side">
        <span class="num">{fmt(t)}</span>
        <span class="vz" role="group" aria-label="Visualization">
          {#each MODES as m}<button aria-pressed={mode === m} onclick={() => setPref('viz', m)}>{m}</button>{/each}
        </span>
      </span>
      <span class="ctl">
        <button class="ico" aria-label="Back 15 seconds" onclick={() => player.skip(-15)}><svg width="20" height="20" viewBox="0 0 20 20" fill="none" stroke="currentColor" stroke-width="1.1"><path d="M4.6 8.4A6 6 0 1 1 4 12"/><path d="M4 4.6v3.9h3.9"/></svg></button>
        <button class="play" aria-label={now.playing ? 'Pause' : 'Play'} onclick={player.toggle}>
          {#if now.playing}
            <svg width="16" height="16" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.2"><line x1="5.5" y1="3.5" x2="5.5" y2="12.5"/><line x1="10.5" y1="3.5" x2="10.5" y2="12.5"/></svg>
          {:else}
            <svg width="16" height="16" viewBox="0 0 16 16"><path d="M5 3v10l8.2-5z" fill="none" stroke="currentColor" stroke-width="1.2" stroke-linejoin="round"/></svg>
          {/if}
        </button>
        <button class="ico" aria-label="Forward 30 seconds" onclick={() => player.skip(30)}><svg width="20" height="20" viewBox="0 0 20 20" fill="none" stroke="currentColor" stroke-width="1.1"><path d="M15.4 8.4A6 6 0 1 0 16 12"/><path d="M16 4.6v3.9h-3.9"/></svg></button>
      </span>
      <span class="side end">
        <button class="spd num" aria-label="Playback speed {now.speed}×" onclick={nextSpeed}>{now.speed.toFixed(1)}×</button>
        <span class="num">{left}</span>
      </span>
    </div>
  {:else}
    <p class="quiet">The room is quiet. Choose an episode on the right.</p>
  {/if}
</section>

<style>
  .listen { position: relative; padding: 64px 48px 44px; display: grid; grid-template-rows: auto 1fr auto; min-height: 0; min-width: 0; }
  .now { display: grid; grid-template-columns: 136px minmax(0, 1fr); gap: 24px; align-items: center; min-width: 0; }
  .cover { display: block; aspect-ratio: 1; width: 100%; border-radius: 9px; overflow: hidden; background: var(--hair); }
  .cover img { display: block; width: 100%; height: 100%; object-fit: cover; }
  .who { min-width: 0; }
  .title { width: 100%; text-align: left; max-width: 22ch; font-weight: 250; font-size: 28px; line-height: 1.18; letter-spacing: -0.012em; margin: 0 0 4px; text-wrap: balance;
    display: -webkit-box; -webkit-line-clamp: 3; line-clamp: 3; -webkit-box-orient: vertical; overflow: hidden; }
  .title:hover { color: var(--accent); }
  .stage { min-height: 0; display: grid; align-items: center; }
  .vz { display: flex; gap: 12px; }
  .vz button { font-family: var(--font-mono); font-size: 10.5px; letter-spacing: 0.1em; text-transform: uppercase; color: var(--text-faint); transition: color 0.14s ease; }
  .vz button:hover { color: var(--text-dim); }
  .vz button[aria-pressed="true"] { color: var(--text); }
  .tp { display: grid; grid-template-columns: 1fr auto 1fr; align-items: center; gap: 20px; margin-top: 6px; }
  .side { display: flex; align-items: center; gap: 16px; white-space: nowrap; }
  .side.end { justify-content: flex-end; }
  .ctl { display: flex; align-items: center; gap: 30px; }
  .ico { color: var(--text-dim); display: grid; place-items: center; width: 28px; height: 28px; transition: color 0.14s ease; }
  .ico:hover { color: var(--text); }
  .play { width: 50px; height: 50px; border-radius: 50%; display: grid; place-items: center; box-shadow: inset 0 0 0 1px var(--line); color: var(--text); transition: box-shadow 0.14s ease; }
  .play:hover { box-shadow: inset 0 0 0 1px var(--accent); }
  .spd { color: var(--text-faint); transition: color 0.14s ease; }
  .spd:hover { color: var(--text); }
  .quiet { align-self: center; color: var(--text-dim); font-size: 14px; grid-row: 2; }
</style>

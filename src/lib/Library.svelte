<script lang="ts">
  import { onMount } from 'svelte';
  import { listen } from '@tauri-apps/api/event';
  import { api, length, plain, short, type Episode, type Show } from '$lib/api';
  import Settings from '$lib/Settings.svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { chapters, goTo, now, player } from '$lib/now.svelte';
  import { I } from '$lib/icons';
  import { fmt } from '$lib/api';
  import { ui } from '$lib/ui.svelte';

  // Show notes as plain paragraphs: the feed's HTML is read for its text only.
  let notes = $state<string[]>([]);
  $effect(() => {
    const id = now.episode?.id;
    if (!ui.notes || !id) return;
    invoke<string | null>('episode_notes', { id }).then((html) => {
      const doc = new DOMParser().parseFromString(html ?? '', 'text/html');
      doc.querySelectorAll('br').forEach((b) => b.replaceWith('\n'));
      const blocks = [...doc.body.querySelectorAll('p, li')].map((p) => p.textContent ?? '');
      notes = (blocks.length ? blocks : (doc.body.textContent ?? '').split(/\n\s*\n|\n/))
        .map((t) => t.replace(/\s+/g, ' ').trim()).filter(Boolean);
    });
  });

  let { current = null, onplay }: { current?: number | null; onplay: (e: Episode) => void } = $props();

  let tab = $state<'new' | 'following' | 'settings'>('new');
  let open = $state<Show | null>(null);
  let newest = $state<Episode[]>([]);
  let shows = $state<Show[]>([]);
  let episodes = $state<Episode[]>([]);
  let list: HTMLElement;

  async function load() {
    [newest, shows] = await Promise.all([api.newest(), api.shows()]);
    if (open) episodes = await api.showEpisodes(open.id);
  }

  onMount(() => {
    load();
    const un = listen('library', load);
    const up = listen<{ id: number }>('episode', load);
    return () => { un.then((f) => f()); up.then((f) => f()); };
  });

  let feedFix = $state(''), fixNote = $state(''), confirming = $state(false);

  async function fix(ev: SubmitEvent) {
    ev.preventDefault();
    if (!open || !feedFix.trim()) return;
    fixNote = 'Looking…';
    try { const t = await api.correctFeed(open.id, feedFix); fixNote = `Now following ${t}.`; feedFix = ''; drill(null); }
    catch (e) { fixNote = String(e); }
  }

  async function leave() {
    if (!open) return;
    if (!confirming) { confirming = true; return; }
    await api.unfollow(open.id);
    drill(null);
  }

  async function drill(s: Show | null) {
    confirming = false; fixNote = ''; feedFix = '';
    open = s;
    episodes = s ? await api.showEpisodes(s.id) : [];
    list.scrollTop = 0;
  }

  const DAY = 86400;
  function bucket(e: Episode) {
    const now = new Date(), start = new Date(now.getFullYear(), now.getMonth(), now.getDate()).getTime() / 1000;
    const t = e.published ?? 0;
    return t >= start ? 'Today' : t >= start - 6 * DAY ? 'This week' : 'Earlier';
  }
  const grouped = $derived(
    newest.reduce<{ label: string; eps: Episode[] }[]>((g, e) => {
      const label = bucket(e);
      if (g.at(-1)?.label !== label) g.push({ label, eps: [] });
      g.at(-1)!.eps.push(e);
      return g;
    }, []),
  );
  const date = (t: number | null) =>
    t ? new Date(t * 1000).toLocaleDateString('en-GB', { day: 'numeric', month: 'short' }) : '';
</script>

{#snippet cover(url: string | null, size: number)}
  <span class="cover" style:width="{size}px">
    {#if url}<img src={url} alt="" loading="lazy" />{/if}
  </span>
{/snippet}

{#snippet row(e: Episode, withCover: boolean, meta: string)}
  <button class="row" class:plain={!withCover} class:heard={e.played} class:playing={e.id === current} onclick={() => onplay(e)}>
    {#if withCover}{@render cover(e.image_url, 34)}{/if}
    <span class="txt"><span class="t">{e.title}</span><span class="m"><span class="sub">{meta}</span> <span class="num">{length(e)}</span></span></span>
  </button>
{/snippet}

<section class="lib" aria-label="Library">
  <div class="lib-h" data-tauri-drag-region>
    <span class="tabs" role="group" aria-label="Library">
      <button aria-pressed={!ui.notes && tab === 'new'} onclick={() => { tab = 'new'; open = null; ui.notes = false; }}>New</button>
      <button aria-pressed={!ui.notes && tab === 'following'} onclick={() => { tab = 'following'; open = null; ui.notes = false; }}>Following</button>
    </span>
    <span class="modes">
      <button aria-label="Mini player" onclick={() => goTo('mini')}>{@html I.mini}</button>
      <button aria-label="Pill" onclick={() => goTo('pill')}>{@html I.pill}</button>
      <button aria-label="Settings" aria-pressed={!ui.notes && tab === 'settings'} onclick={() => { tab = 'settings'; ui.notes = false; }}>
        <svg width="15" height="15" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.1"><circle cx="8" cy="8" r="2.2"/><path d="M8 1.5v2M8 12.5v2M1.5 8h2M12.5 8h2M3.4 3.4l1.4 1.4M11.2 11.2l1.4 1.4M3.4 12.6l1.4-1.4M11.2 4.8l1.4-1.4"/></svg>
      </button>
    </span>
  </div>
  <div class="list" bind:this={list}>
    {#if ui.notes && now.episode}
      <button class="back" onclick={() => (ui.notes = false)}>← back</button>
      <h2 class="show-h">{now.episode.title}</h2>
      <p class="show-p sub">{now.episode.show_title} · {date(now.episode.published)}</p>
      <button class="keep" aria-pressed={now.episode.kept} onclick={() => { const e = now.episode!; api.keep(e.id, !e.kept); e.kept = !e.kept; }}>
        {now.episode.kept ? 'Kept offline · let it go' : 'Keep offline'}
      </button>
      {#if chapters.list.length}
        <ol class="chapters">
          {#each chapters.list as c}
            <li><button onclick={() => player.seek(c.start)}><span class="num">{fmt(c.start)}</span><span>{c.title}</span></button></li>
          {/each}
        </ol>
      {/if}
      {#each notes as n}<p class="note">{n}</p>{:else}<p class="quiet">This episode came without notes.</p>{/each}
    {:else if tab === 'settings'}
      <Settings />
    {:else if tab === 'new'}
      {#each grouped as g (g.label)}
        <div class="day">{g.label}</div>
        {#each g.eps as e (e.id)}{@render row(e, true, `${short(e.show_title)} ·`)}{/each}
      {:else}
        <p class="quiet">Nothing new yet. Bring your shows in from Settings.</p>
      {/each}
    {:else if open}
      <button class="back" onclick={() => drill(null)}>← Following</button>
      <h2 class="show-h">{open.title}</h2>
      {#if open.about}<p class="show-p">{plain(open.about)}</p>{/if}
      {#if open.spotify_only}
        <p class="show-p">No public feed was found for this show, so it can't be played here. If it has one, paste its address below.</p>
      {/if}
      {#each episodes as e (e.id)}{@render row(e, false, `${date(e.published)} ·`)}{/each}
      <div class="care">
        <form onsubmit={fix}>
          <input bind:value={feedFix} placeholder={open.spotify_only ? 'Its feed address' : 'Wrong show? Paste the right feed address'} aria-label="Feed address for this show" />
        </form>
        {#if fixNote}<p class="quiet">{fixNote}</p>{/if}
        <button class="leave" onclick={leave}>{confirming ? 'Press again to stop following' : 'Stop following'}</button>
      </div>
    {:else}
      {#each shows as s (s.id)}
        <button class="row show" onclick={() => drill(s)}>
          {@render cover(s.image_url, 40)}
          <span class="txt"><span class="t">{s.title}</span>
            <span class="m sub">
              {#if s.spotify_only}Only on Spotify{:else}{#if s.fresh}<span class="new">{s.fresh} new</span> · {/if}{s.author ?? ''}{/if}
            </span></span>
        </button>
      {:else}
        <p class="quiet">You aren't following anything yet. Bring your shows in from Settings.</p>
      {/each}
    {/if}
  </div>
</section>

<style>
  .lib { border-left: 1px solid var(--hair); padding: 12px 22px 20px 32px; display: grid; grid-template-rows: 36px 1fr; min-height: 0; position: relative; z-index: 2; }
  .lib-h { display: flex; justify-content: space-between; align-items: center; }
  .tabs { display: flex; gap: 20px; align-items: baseline; }
  .tabs button { font-size: 14.5px; color: var(--text-faint); transition: color 0.14s ease; }
  .tabs button[aria-pressed="true"] { color: var(--text); }
  .modes { display: flex; gap: 2px; }
  .modes button { width: 28px; height: 24px; display: grid; place-items: center; color: var(--text-faint); border-radius: 6px; transition: color 0.14s ease; }
  .modes button:hover { color: var(--text); }
  .modes button[aria-pressed="true"] { color: var(--text-dim); }
  .list { overflow: auto; scrollbar-width: none; min-height: 0; padding: 14px 10px 0 0; }
  .list::-webkit-scrollbar { display: none; }
  .day { font-size: 12px; color: var(--text-faint); margin: 22px 0 2px; }
  .day:first-child { margin-top: 4px; }
  .row { display: grid; grid-template-columns: 34px minmax(0, 1fr); gap: 14px; align-items: center; width: 100%; text-align: left; padding: 9px 0; }
  .row.show { grid-template-columns: 40px minmax(0, 1fr); padding: 10px 0; }
  .row.plain { grid-template-columns: minmax(0, 1fr); }
  .txt { min-width: 0; }
  .t { display: block; font-size: 14.5px; line-height: 1.35; color: var(--text); white-space: nowrap; overflow: hidden; text-overflow: ellipsis; transition: color 0.14s ease; }
  .m { display: block; margin-top: 1px; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .row:hover .t, .row.playing .t { color: var(--accent); }
  .row.heard .t, .row.heard .m { opacity: 0.5; }
  .cover { display: block; aspect-ratio: 1; border-radius: 6px; overflow: hidden; background: var(--hair); }
  .cover img { display: block; width: 100%; height: 100%; object-fit: cover; }
  .back { font-size: 12.5px; color: var(--text-dim); margin: 4px 0 18px; }
  .back:hover { color: var(--text); }
  .show-h { font-weight: 250; font-size: 24px; line-height: 1.2; margin: 0 0 8px; }
  .show-p { color: var(--text-dim); font-size: 13.5px; margin: 0 0 14px; max-width: 38ch; display: -webkit-box; -webkit-line-clamp: 5; line-clamp: 5; -webkit-box-orient: vertical; overflow: hidden; }
  .new { color: var(--accent); }
  .keep { font-size: 12.5px; color: var(--text-dim); margin: 0 0 18px; }
  .keep:hover, .keep[aria-pressed="true"] { color: var(--accent); }
  .chapters { list-style: none; padding: 0; margin: 0 0 22px; }
  .chapters button { display: grid; grid-template-columns: 52px 1fr; gap: 8px; text-align: left; padding: 5px 0; font-size: 13.5px; color: var(--text-mid); width: 100%; }
  .chapters button:hover { color: var(--accent); }
  .note { font-size: 14px; line-height: 1.6; color: var(--text-mid); margin: 0 0 12px; max-width: 42ch; user-select: text; cursor: text; }
  .care { margin: 26px 0 10px; padding-top: 18px; border-top: 1px solid var(--hair); display: grid; gap: 12px; justify-items: start; }
  .care form { width: 100%; }
  .care input { font: inherit; font-size: 13.5px; color: var(--text); background: none; border: 0; border-bottom: 1px solid var(--line); padding: 4px 0 6px; outline: none; width: 100%; }
  .care input:focus { border-bottom-color: var(--accent); }
  .care input::placeholder { color: var(--text-faint); }
  .leave { font-size: 12.5px; color: var(--text-faint); }
  .leave:hover { color: var(--failed); }
  .quiet { color: var(--text-dim); font-size: 13.5px; max-width: 32ch; margin-top: 8px; }
</style>

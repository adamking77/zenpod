<script lang="ts">
  import { onMount } from 'svelte';
  import { listen } from '@tauri-apps/api/event';
  import { api, length, plain, short, type Episode, type Show } from '$lib/api';
  import Settings from '$lib/Settings.svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { chapters, goTo, now, player, position } from '$lib/now.svelte';
  import { openUrl } from '@tauri-apps/plugin-opener';
  import { paragraphs, parseTranscript, type Cue } from '$lib/transcript';
  import { I } from '$lib/icons';
  import { fmt } from '$lib/api';
  import { ui } from '$lib/ui.svelte';

  // Show notes as paragraphs of text and links; the feed's HTML is read for nothing else.
  type Bit = { text: string; href?: string };
  let notes = $state<Bit[][]>([]);
  let cues = $state<Cue[][]>([]);
  let reading = $state<'notes' | 'transcript'>('notes');
  const linkish = /(https?:\/\/[^\s<>"')]+)/g;
  function bits(el: Element): Bit[] {
    const out: Bit[] = [];
    const walk = (n: Node) => {
      if (n instanceof HTMLAnchorElement && /^https?:/.test(n.href)) { out.push({ text: n.textContent || n.href, href: n.href }); return; }
      if (n.nodeType === Node.TEXT_NODE) {
        (n.textContent ?? '').split(linkish).forEach((t, i) => t && out.push(i % 2 ? { text: t, href: t } : { text: t.replace(/\s+/g, ' ') }));
        return;
      }
      n.childNodes.forEach(walk);
    };
    walk(el);
    return out;
  }
  $effect(() => {
    const id = now.episode?.id;
    reading = 'notes';
    cues = [];
    if (!ui.notes || !id) return;
    invoke<string | null>('episode_notes', { id }).then((html) => {
      const doc = new DOMParser().parseFromString(html ?? '', 'text/html');
      const blocks = [...doc.body.querySelectorAll('p, li')];
      if (!blocks.length) {
        doc.body.innerHTML = (doc.body.innerHTML || '').split(/<br\s*\/?>\s*<br\s*\/?>|\n\s*\n/).map((p) => `<p>${p}</p>`).join('');
        blocks.push(...doc.body.querySelectorAll('p'));
      }
      notes = blocks.map(bits).filter((b) => b.some((x) => x.text.trim()));
    });
    invoke<string | null>('transcript', { id }).then((t) => { if (t && now.episode?.id === id) cues = paragraphs(parseTranscript(t)); });
  });
  const spoken = $derived.by(() => {
    const t = position();
    let hit: Cue | null = null;
    for (const p of cues) for (const c of p) if (c.start >= 0 && c.start <= t) hit = c;
    return hit;
  });
  // Keep the line being spoken in view, once per line.
  $effect(() => {
    if (reading !== 'transcript' || !spoken) return;
    list?.querySelector('.cue.now')?.scrollIntoView({ block: 'center', behavior: matchMedia('(prefers-reduced-motion: reduce)').matches ? 'auto' : 'smooth' });
  });

  let { current = null, onplay }: { current?: number | null; onplay: (e: Episode) => void } = $props();

  let open = $state<Show | null>(null);
  let newest = $state<Episode[]>([]);
  let shows = $state<Show[]>([]);
  let episodes = $state<Episode[]>([]);
  let list: HTMLElement;
  // The pointer resting on an episode starts fetching it, so a click plays from memory. Once per episode.
  const warmed = new Set<number>();
  const warm = (id: number) => { if (id !== current && !warmed.has(id)) { warmed.add(id); invoke('warm', { id }); } };
  // Just-added shows lead the list until you move on.
  const following = $derived(ui.arrived ? [...shows].sort((a, b) => Number(ui.arrived!.ids.includes(b.id)) - Number(ui.arrived!.ids.includes(a.id))) : shows);

  async function load() {
    [newest, shows] = await Promise.all([api.newest(), api.shows()]);
    ui.empty = shows.length === 0;
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

  // A show just added opens here, once the library has it.
  $effect(() => {
    const s = ui.reveal != null && shows.find((x) => x.id === ui.reveal);
    if (s) { ui.reveal = null; ui.notes = false; drill(s).then(() => (added = s.id)); }
  });
  let added = $state<number | null>(null);
  // A show chosen from the listening side opens here.
  $effect(() => {
    const s = ui.showing != null && shows.find((x) => x.id === ui.showing);
    if (s) { ui.showing = null; ui.notes = false; ui.arrived = null; ui.tab = 'following'; drill(s); }
  });

  async function drill(s: Show | null) {
    added = null;
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
  <button class="row" class:plain={!withCover} class:heard={e.played} class:playing={e.id === current} onclick={() => onplay(e)} onpointerenter={() => warm(e.id)} onfocus={() => warm(e.id)}>
    {#if withCover}{@render cover(e.image_url, 34)}{/if}
    <span class="txt"><span class="t">{e.title}</span><span class="m"><span class="sub">{meta}</span> <span class="num">{length(e)}</span></span></span>
  </button>
{/snippet}

<section class="lib" aria-label="Library">
  <div class="lib-h">
    <span class="tabs" role="group" aria-label="Library">
      <button aria-pressed={!ui.notes && ui.tab === 'new'} onclick={() => { ui.tab = 'new'; open = null; ui.notes = false; ui.arrived = null; }}>New</button>
      <button aria-pressed={!ui.notes && ui.tab === 'following'} onclick={() => { ui.tab = 'following'; open = null; ui.notes = false; ui.arrived = null; }}>Following</button>
    </span>
    <span class="modes">
      <button aria-label="Mini player" onclick={() => goTo('mini')}>{@html I.mini}</button>
      <button aria-label="Pill" onclick={() => goTo('pill')}>{@html I.pill}</button>
      <button aria-label="Settings" aria-pressed={!ui.notes && ui.tab === 'settings'} onclick={() => { ui.tab = 'settings'; ui.notes = false; ui.arrived = null; }}>
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
      {#if cues.length}
        <span class="reading" role="group" aria-label="Read">
          <button aria-pressed={reading === 'notes'} onclick={() => (reading = 'notes')}>Notes</button>
          <button aria-pressed={reading === 'transcript'} onclick={() => (reading = 'transcript')}>Transcript</button>
        </span>
      {/if}
      {#if reading === 'transcript'}
        {#each cues as p}
          <p class="note">{#each p as c}<button class="cue" class:now={c === spoken} onclick={() => c.start >= 0 && player.seek(c.start)}>{c.text}</button>{' '}{/each}</p>
        {/each}
      {:else}
        {#each notes as n}
          <p class="note" data-no-drag>{#each n as b}{#if b.href}<a href={b.href} onclick={(e) => { e.preventDefault(); openUrl(b.href!); }}>{b.text}</a>{:else}{b.text}{/if}{/each}</p>
        {:else}<p class="quiet">This episode came without notes.</p>{/each}
      {/if}
    {:else if ui.tab === 'settings'}
      <Settings />
    {:else if ui.tab === 'new'}
      {#each grouped as g (g.label)}
        <div class="day">{g.label}</div>
        {#each g.eps as e (e.id)}{@render row(e, true, `${short(e.show_title)} ·`)}{/each}
      {:else}
        <p class="quiet">Nothing new yet. Bring your shows in from Settings.</p>
      {/each}
    {:else if open}
      <button class="back" onclick={() => drill(null)}>← Following</button>
      {#if added === open.id}
        <p class="done-word">Now following</p>
      {/if}
      <span class="show-art">{@render cover(`listener://localhost/art/${open.id}`, 88)}</span>
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
      {#if ui.arrived}
        <div class="arrived" aria-live="polite">
          <p>{ui.arrived.note}</p>
          {#if ui.arrived.unreached.length}
            <p class="sub">Couldn't reach {ui.arrived.unreached.length === 1 ? 'this one' : `these ${ui.arrived.unreached.length}`}: {ui.arrived.unreached.join(', ')}. Their feeds may be private or gone.</p>
          {/if}
        </div>
      {/if}
      {#each following as s (s.id)}
        <button class="row show" class:arrive={ui.arrived?.ids.includes(s.id)} onclick={() => drill(s)}>
          {@render cover(s.image_url, 40)}
          <span class="txt"><span class="t">{s.title}</span>
            <span class="m sub">
              {#if ui.arrived?.ids.includes(s.id)}<span class="new">Just added</span> · {/if}{#if s.spotify_only}Only on Spotify{:else}{#if s.fresh}<span class="new">{s.fresh} new</span> · {/if}{s.author ?? ''}{/if}
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
  .show-art { display: block; margin: 4px 0 16px; }
  .show-art :global(.cover) { border-radius: 9px; }
  .show-h { font-weight: 250; font-size: 24px; line-height: 1.2; margin: 0 0 8px; }
  .show-p { color: var(--text-dim); font-size: 13.5px; margin: 0 0 14px; max-width: 38ch; display: -webkit-box; -webkit-line-clamp: 5; line-clamp: 5; -webkit-box-orient: vertical; overflow: hidden; }
  .new { color: var(--accent); }
  .keep { font-size: 12.5px; color: var(--text-dim); margin: 0 0 18px; }
  .keep:hover, .keep[aria-pressed="true"] { color: var(--accent); }
  .chapters { list-style: none; padding: 0; margin: 0 0 22px; }
  .chapters button { display: grid; grid-template-columns: 52px 1fr; gap: 8px; text-align: left; padding: 5px 0; font-size: 13.5px; color: var(--text-mid); width: 100%; }
  .chapters button:hover { color: var(--accent); }
  .reading { display: flex; gap: 16px; margin: 0 0 14px; }
  .reading button { font-size: 13px; color: var(--text-faint); transition: color 0.14s ease; }
  .reading button[aria-pressed="true"] { color: var(--text); }
  .note a { color: var(--accent); text-decoration: none; overflow-wrap: anywhere; }
  .note a:hover { text-decoration: underline; }
  .cue { display: inline; text-align: left; font: inherit; color: inherit; transition: color 0.2s var(--ease); }
  .cue:hover { color: var(--text); }
  .cue.now { color: var(--accent); }
  .note { font-size: 14px; line-height: 1.6; color: var(--text-mid); margin: 0 0 12px; max-width: 42ch; user-select: text; cursor: text; }
  .care { margin: 26px 0 10px; padding-top: 18px; border-top: 1px solid var(--hair); display: grid; gap: 12px; justify-items: start; }
  .care form { width: 100%; }
  .care input { font: inherit; font-size: 13.5px; color: var(--text); background: none; border: 0; border-bottom: 1px solid var(--line); padding: 4px 0 6px; outline: none; width: 100%; }
  .care input:focus { border-bottom-color: var(--accent); }
  .care input::placeholder { color: var(--text-faint); }
  .leave { font-size: 12.5px; color: var(--text-faint); }
  .leave:hover { color: var(--failed); }
  /* 2050 motion: the summary is a quiet note, "Now following" a done word, new shows content arriving. Once per mount. */
  .arrived { padding: 4px 0 14px; margin-bottom: 8px; border-bottom: 1px solid var(--hair);
    transition: opacity var(--dur-pane) var(--ease-out), transform var(--dur-pane) var(--ease-out); }
  .done-word { margin: 0 0 10px; font-size: 13px; color: var(--accent); transition: opacity var(--dur-pane) var(--ease-out), transform var(--dur-pane) var(--ease-out); }
  .row.arrive { transition: opacity var(--dur-pane) var(--ease-out), transform var(--dur-pane) var(--ease-out); }
  @starting-style {
    .arrived { opacity: 0; transform: translateY(8px); }
    .done-word, .row.arrive { opacity: 0; transform: translateY(4px); }
  }
  @media (prefers-reduced-motion: reduce) {
    @starting-style { .arrived, .done-word, .row.arrive { transform: none; } }
  }
  .arrived p { margin: 0; font-size: 14px; color: var(--text); }
  .arrived .sub { margin-top: 6px; font-size: 13px; line-height: 1.45; }
  .quiet { color: var(--text-dim); font-size: 13.5px; max-width: 32ch; margin-top: 8px; }
</style>

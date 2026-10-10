<script lang="ts">
  import { onMount } from 'svelte';
  import { listen } from '@tauri-apps/api/event';
  import Art from '$lib/Art.svelte';
  import { home } from '$lib/home';
  import { api, length, type Episode } from '$lib/api';
  import { ui } from '$lib/ui.svelte';
  import { mins, reads, rtm, take, voiceLabel } from '$lib/reads.svelte';

  // A show that takes things one at a time has its own field: what's pasted here goes straight into it.
  let entry = $state('');
  function add(e: KeyboardEvent) {
    if (e.key !== 'Enter' || e.shiftKey || !show) return;
    e.preventDefault();
    const v = entry; entry = '';
    take(v, show.title);
  }

  // A show you made: where its episodes come from, its voice, and its episodes, newest first.
  let { id, current = null, onplay }: { id: number; current?: number | null; onplay: (e: Episode, list: Episode[]) => void } = $props();

  const show = $derived(rtm.shows.find((s) => s.id === id) ?? null);
  const pending = $derived(rtm.reads.filter((r) => r.show_id === id).toReversed());
  let episodes = $state<Episode[]>([]), armed = $state(false);

  const load = () => api.showEpisodes(id).then((e) => (episodes = e));
  $effect(() => { id; armed = false; load(); });
  onMount(() => {
    const a = listen('library', load), b = listen('episode', load);
    return () => { a.then((f) => f()); b.then((f) => f()); };
  });

  const back = $derived(ui.yours?.back === 'following' ? 'Following' : 'Read to me');
  function leave() {
    const to = ui.yours?.back ?? 'read';
    ui.yours = null;
    if (to === 'following') ui.tab = 'following';
  }
  const source = $derived(
    show?.kind === 'feed' ? `New posts from ${(show.source ?? '').replace(/^https?:\/\//, '')}` : show?.kind === 'folder' ? `New files in ${home(show.source ?? '')}`
      : show?.kind === 'read-to-me' ? 'Everything you’ve added. Move any of these into a show of your own, or delete what you’re done with.'
      : 'A show you made. Move episodes into it from Read to me.');
  // Where an episode can go: Read to me and the shows you named (feed and folder shows keep only their own).
  const lists = $derived(rtm.shows.filter((s) => (s.kind === 'read-to-me' || s.kind === 'items') && s.id !== id));
  const movable = $derived(show?.kind === 'read-to-me' || show?.kind === 'items');
  const voices = $derived([['', `Your default (${voiceLabel()})`], ...rtm.mac.map((v) => [`mac:${v.name}`, v.name])]);

  async function remove() {
    if (!armed) { armed = true; return; }
    await reads.removeShow(id);
    leave();
  }
</script>

{#if show}
  <div class="page">
    <button class="back" onclick={leave}>← {back}</button>
    <span class="show-art"><Art id={show.id} kind={show.kind} name={show.title} size={88} /></span>
    <h2 class="show-h">{show.title}</h2>
    <p class="show-p">{source}</p>
    {#if show.problem}<p class="show-p problem">{show.problem}</p>{/if}
    {#if show.kind === 'items' || show.kind === 'read-to-me'}
      <textarea class="add" rows="1" bind:value={entry} placeholder="Add to {show.title}: paste a link or text" aria-label="Add to {show.title}" spellcheck="false" onkeydown={add}></textarea>
    {/if}
    <div class="opts">
      {#if show.kind === 'feed' || show.kind === 'folder'}
        <span class="words wide"><button aria-pressed={show.auto} onclick={() => reads.setShow(id, { auto: !show.auto })}>Read new {show.kind === 'feed' ? 'posts' : 'files'} automatically</button></span>
      {/if}
      <span class="l">Voice</span>
      <span><select aria-label="Voice for this show" value={show.voice ?? ''} onchange={(e) => reads.setShow(id, { voice: e.currentTarget.value })}>
        {#each voices as [v, label] (v)}<option value={v}>{label}</option>{/each}
      </select></span>
    </div>

    {#each pending as r (r.id)}
      <div class="row waiting">
        <Art id={id} kind="items" page={r.title} size={34} />
        <span class="txt"><span class="t">{r.title}</span>
          <span class="m sub">
            {#if r.state === 'waiting'}Ready to read · <span class="num">{mins(r.chars)} min</span> <button class="act" onclick={() => reads.now(r.id)}>Read</button><button class="rm" onclick={() => reads.discard(r.id)}>· Remove</button>
            {:else if r.state === 'failed'}<span class="err">{r.error}</span> <button class="act" onclick={() => reads.now(r.id)}>Try again</button>
            {:else if r.state === 'reading'}Reading {Math.min(r.done + 1, r.pieces || 1)} of {r.pieces || '…'}<button class="rm stop" onclick={() => reads.stop(r.id)}>· Stop</button>
            {:else}Next to read<button class="rm stop" onclick={() => reads.stop(r.id)}>· Stop</button>{/if}
          </span></span>
      </div>
    {/each}
    {#each episodes as e (e.id)}
      <div class="row" class:heard={e.played} class:playing={e.id === current}>
        <button class="hit" onclick={() => onplay(e, episodes)} aria-label="Play {e.title}"></button>
        <Art id={id} kind={show.kind} page={e.title} size={34} />
        <span class="txt"><span class="t">{e.title}</span>
          <span class="m sub">{#if e.played === false && e.position === 0 && (e.published ?? 0) > Date.now() / 1000 - 7 * 86400}<span class="new">New</span> · {/if}<span class="num">{length(e)}</span>{#if movable && lists.length}<select class="move" aria-label="Move {e.title} to another show" value="" onchange={(ev) => { const to = Number(ev.currentTarget.value); if (to) reads.move(e.id, to); }}><option value="" disabled>Move to…</option>{#each lists as s (s.id)}<option value={s.id}>{s.title}</option>{/each}</select>{/if}<button class="rm" onclick={() => reads.deleteEpisode(e.id)}>· Delete</button></span></span>
      </div>
    {/each}
    {#if !pending.length && !episodes.length}
      <p class="quiet">{show.kind === 'feed' || show.kind === 'folder' ? 'Nothing here yet. New ones arrive when Zenpod next checks.' : show.kind === 'read-to-me' ? 'Nothing here yet. Add something above.' : 'Nothing here yet. Add something above, or move an episode here from Read to me.'}</p>
    {/if}

    {#if show.kind !== 'read-to-me'}
      <button class="leave" onclick={remove}>{armed ? 'Press again to remove this show and its episodes' : 'Remove this show'}</button>
    {/if}
  </div>
{/if}

<style>
  .back { font-size: 12.5px; color: var(--text-dim); margin: 4px 0 18px; display: inline-block; }
  .back:hover { color: var(--text); }
  .show-art { display: block; margin: 4px 0 16px; }
  .show-art :global(.art) { border-radius: 9px; }
  .show-h { font-weight: 250; font-size: 24px; line-height: 1.2; margin: 0 0 8px; }
  .show-p.problem { color: var(--failed); margin-top: -8px; }
  .show-p { color: var(--text-dim); font-size: 13.5px; line-height: 1.5; margin: 0 0 14px; max-width: 38ch; overflow-wrap: anywhere; }
  .add { all: unset; box-sizing: border-box; display: block; width: 100%; margin: 0 0 18px; font: 400 13.5px/1.45 var(--font-ui); color: var(--text);
    border-bottom: 1px solid var(--line); padding: 4px 0 6px; resize: none; white-space: pre-wrap; overflow-wrap: anywhere; caret-color: var(--accent);
    transition: border-color var(--dur-base) var(--ease-hover); }
  .add:focus { border-bottom-color: var(--accent); }
  .add::placeholder { color: var(--text-faint); }
  .opts { display: grid; grid-template-columns: auto minmax(0, 1fr); gap: 10px 16px; align-items: baseline; margin: 4px 0 18px; font-size: 13.5px; }
  .l { color: var(--text-faint); font-size: 12.5px; }
  .words { display: flex; gap: 14px; flex-wrap: wrap; }
  .words.wide { grid-column: 1 / -1; }
  .words button { font-size: 13.5px; color: var(--text-faint); transition: color var(--dur-base) var(--ease-hover); }
  .words button:hover { color: var(--text-dim); }
  .words button[aria-pressed="true"] { color: var(--text); }
  select { all: unset; cursor: pointer; color: var(--text); border-bottom: 1px solid var(--line); padding: 0 16px 3px 0; max-width: 100%;
    white-space: nowrap; overflow: hidden; text-overflow: ellipsis;
    background: linear-gradient(45deg, transparent 50%, var(--text-faint) 50%) right 4px top 55% / 5px 5px no-repeat,
      linear-gradient(-45deg, transparent 50%, var(--text-faint) 50%) right 0 top 55% / 5px 5px no-repeat; }
  .row { position: relative; display: grid; grid-template-columns: 34px minmax(0, 1fr); gap: 14px; align-items: center; padding: 9px 0; }
  .hit { position: absolute; inset: 0; z-index: 0; }
  .txt { min-width: 0; position: relative; z-index: 1; pointer-events: none; }
  .txt button { pointer-events: auto; }
  .t { display: block; font-size: 14.5px; line-height: 1.35; color: var(--text); white-space: nowrap; overflow: hidden; text-overflow: ellipsis; transition: color var(--dur-base) var(--ease-hover); }
  .m { display: block; margin-top: 1px; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .row:hover .t, .row.playing .t { color: var(--accent); }
  .row.waiting .t { color: var(--text-mid); }
  .row.waiting:hover .t { color: var(--text-mid); }
  .row.heard .t, .row.heard .m { opacity: 0.5; }
  .new { color: var(--accent); }
  .err { color: var(--text-dim); white-space: normal; }
  .act { font-size: inherit; color: var(--accent); margin-left: 4px; }
  /* Move to and Delete are always there, quietly: nobody should have to hover to find them */
  .rm { font-size: inherit; color: var(--text-faint); margin-left: 6px; transition: color var(--dur-base) var(--ease-hover); }
  .move { all: unset; pointer-events: auto; cursor: pointer; font-size: inherit; color: var(--text-faint); margin-left: 8px; transition: color var(--dur-base) var(--ease-hover); }
  .move:hover { color: var(--text); }
  .rm:hover { color: var(--failed); }
  .rm.stop:hover { color: var(--text); }
  .quiet { color: var(--text-dim); font-size: 13.5px; max-width: 32ch; margin-top: 8px; }
  .leave { display: block; font-size: 12.5px; color: var(--text-faint); margin: 26px 0 16px; }
  .leave:hover { color: var(--failed); }
</style>

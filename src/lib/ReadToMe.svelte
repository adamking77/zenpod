<script lang="ts">
  import { tick } from 'svelte';
  import { open as pick } from '@tauri-apps/plugin-dialog';
  import Art from '$lib/Art.svelte';
  import { home } from '$lib/home';
  import { ui } from '$lib/ui.svelte';
  import { makeShow, mins, readCard, reads, rtm, take, takeFile, takeSource, voiceLabel, type Read } from '$lib/reads.svelte';
  import { api, length, type Show } from '$lib/api';
  import { now, player } from '$lib/now.svelte';

  // Read to me, start to finish, in one place: paste or drop something, make the episode, and it plays when it's
  // ready. The one you just asked for stays in a single block that changes as it goes; nothing needs a click elsewhere.

  let field = $state<HTMLTextAreaElement>(), entry = $state('');
  // The field takes the cursor whenever the pane is asked for (R, the icon, a card put away).
  $effect(() => { rtm.focus; if (!rtm.card) tick().then(() => field?.focus({ preventScroll: true })); });

  function keyed(e: KeyboardEvent) {
    if (e.key === 'Enter' && !e.shiftKey) { e.preventDefault(); const v = entry; entry = ''; take(v); }
  }
  const grow = (el: HTMLTextAreaElement) => { el.style.height = 'auto'; el.style.height = `${el.scrollHeight}px`; };
  // Fits a textarea to its words as they change: titles wrap instead of running off the edge.
  function fit(el: HTMLTextAreaElement) { tick().then(() => grow(el)); }

  // Enter makes the episode from anywhere on the card (but not from the show menu), so paste, Enter, Enter is the
  // whole thing. Nothing moves the focus, so nothing gets a focus ring it didn't ask for.
  function enter(e: KeyboardEvent) {
    if (rtm.card?.kind !== 'item' || e.key !== 'Enter' || e.shiftKey || e.target instanceof HTMLSelectElement) return;
    if (e.target instanceof HTMLInputElement && e.target.classList.contains('sm') && !rtm.card.newName.trim()) return;
    e.preventDefault();
    readCard();
  }

  async function chooseFolder() {
    const d = await pick({ directory: true, multiple: false, title: 'Make a show from a folder' }).catch(() => null);
    if (typeof d === 'string') takeSource('folder', d);
  }
  async function chooseFile() {
    const f = await pick({ multiple: false, title: 'Read to me', filters: [{ name: 'Documents', extensions: ['pdf', 'md', 'markdown', 'txt', 'docx', 'doc', 'rtf', 'html', 'htm'] }] }).catch(() => null);
    if (typeof f === 'string') takeFile(f);
  }

  const cur = $derived(rtm.current);
  const curRead = $derived(cur ? rtm.reads.find((r) => r.id === cur.read) ?? null : null);
  const pct = (r: Read | null) => (r?.done && r.pieces ? Math.round((r.done / r.pieces) * 100) : 0);
  const others = $derived(rtm.reads.filter((r) => r.id !== cur?.read && (r.state === 'reading' || r.state === 'queued' || r.state === 'failed')));
  const ready = $derived(rtm.ready.filter((e) => e.id !== cur?.episode));
  const lists = $derived(rtm.shows.filter((s) => s.kind === 'read-to-me' || s.kind === 'items'));
  const cardShow = $derived(rtm.card?.kind === 'item' && rtm.card.to !== 'new' ? rtm.shows.find((s) => s.id === (rtm.card as { to: number }).to) : null);

  const count = (n: number) => `${n} ${n === 1 ? 'episode' : 'episodes'}`;
  // What each show is, in plain words.
  function about(s: Show) {
    if (s.kind === 'read-to-me') return s.episodes ? `Articles and files you've added · ${count(s.episodes)}` : 'Articles and files you add are kept here';
    if (s.kind === 'feed') return `New posts from ${(s.source ?? '').replace(/^https?:\/\/(www\.)?/, '').split('/')[0]} · ${count(s.episodes)}`;
    if (s.kind === 'folder') return `New files in ${home(s.source ?? '')} · ${count(s.episodes)}`;
    return `A show you made · ${s.episodes ? count(s.episodes) : 'empty'}`;
  }
  const busy = (s: Show) => rtm.reads.some((r) => r.show_id === s.id && (r.state === 'reading' || r.state === 'queued'));
  const waiting = (s: Show) => rtm.reads.filter((r) => r.show_id === s.id && r.state === 'waiting').length;

  // A podcast pasted here is followed the usual way, then shown in Following.
  async function follow(url: string) {
    rtm.card = { kind: 'busy', what: 'Following…' };
    try { const [id] = await api.addShow(url); rtm.card = null; ui.reveal = id; ui.tab = 'following'; }
    catch (e) { rtm.card = { kind: 'error', message: String(e) }; }
  }
  const play = (id: number) => player.choose(id, ready.map((e) => e.id));
  function done() { rtm.current = null; rtm.focus++; }

  let naming = $state(false), showName = $state('');
  async function newShow() {
    const n = showName.trim();
    naming = false; showName = '';
    if (n) await reads.newShow(n);
  }
</script>

<svelte:window onkeydown={enter} />

<div class="pane">
  <h2 class="show-h">Read to me</h2>
  <p class="show-p">Paste a link or text, or drop a file, and Zenpod reads it to you as an episode.</p>

  {#if rtm.card?.kind === 'item'}
    {@const c = rtm.card}
    <div class="check">
      <span class="k">New episode</span>
      <div class="ep">
        <Art id={0} kind="items" page={c.title} size={56} />
        <textarea class="name" rows="1" bind:value={c.title} aria-label="Title" spellcheck="false" use:fit oninput={(e) => grow(e.currentTarget)}></textarea>
      </div>
      <span class="from">{c.from} · about {mins(c.text.length)} min</span>
      <span class="to">Goes into
        <select aria-label="Show" bind:value={c.to}>
          {#each lists as s (s.id)}<option value={s.id}>{s.title}</option>{/each}
          <option value="new">A new show…</option>
        </select>
      </span>
      {#if c.to === 'new'}
        <!-- svelte-ignore a11y_autofocus -->
        <input class="name sm" bind:value={c.newName} placeholder="Name the new show" aria-label="New show name" spellcheck="false" autofocus />
      {/if}
      <span class="by">Read by {voiceLabel(cardShow)} <button class="ch" onclick={() => { ui.tab = 'settings'; }}>· Change</button></span>
      <span class="go"><button class="read" onclick={readCard}>Listen</button><button class="cancel" onclick={() => { rtm.card = null; rtm.focus++; }}>Cancel</button></span>
      <span class="hint">Press Enter to listen. It starts as soon as it's ready.</span>
    </div>
  {:else if rtm.card?.kind === 'busy'}
    <p class="quiet busy" aria-live="polite">{rtm.card.what}</p>
  {:else if rtm.card?.kind === 'error'}
    <div class="check">
      <p class="lede">{rtm.card.message}</p>
      <span class="go"><button class="cancel" onclick={() => { rtm.card = null; rtm.focus++; }}>Try something else</button></span>
    </div>
  {:else if rtm.card?.kind === 'podcast'}
    {@const c = rtm.card}
    <div class="check">
      <span class="k">A podcast</span>
      <span class="name">{c.name}</span>
      <span class="from">{c.url.replace(/^https?:\/\//, '')}</span>
      <p class="lede">This feed already has audio, so there's nothing to read. Follow it like any podcast.</p>
      <span class="go">
        <button class="read" onclick={() => follow(c.url)}>Follow it</button>
        <button class="cancel" onclick={() => (rtm.card = null)}>Cancel</button>
      </span>
    </div>
  {:else if rtm.card?.kind === 'feed' || rtm.card?.kind === 'folder'}
    {@const c = rtm.card}
    {@const word = c.kind === 'feed' ? 'posts' : 'files'}
    <div class="check">
      <span class="k">{c.kind === 'feed' ? 'A blog or newsletter' : 'A folder'}</span>
      <input class="name" bind:value={c.name} aria-label="Show name" spellcheck="false" onkeydown={(e) => e.key === 'Enter' && makeShow()} />
      <span class="from">{c.kind === 'folder' ? home(c.source) : c.source.replace(/^https?:\/\//, '')}</span>
      <p class="lede">Make it a show, and its new {word} become episodes. These are its latest:</p>
      <ul class="posts">{#each c.items as t}<li>{t}</li>{/each}</ul>
      <span class="to">New {word}</span>
      <span class="words">
        <button aria-pressed={!c.auto} onclick={() => (c.auto = false)}>Wait for me</button>
        <button aria-pressed={c.auto} onclick={() => (c.auto = true)}>Read them automatically</button>
      </span>
      <span class="by">Read by {voiceLabel()} <button class="ch" onclick={() => { ui.tab = 'settings'; }}>· Change</button></span>
      <span class="go"><button class="read" onclick={makeShow}>Make it a show</button><button class="cancel" onclick={() => { rtm.card = null; rtm.focus++; }}>Cancel</button></span>
    </div>
  {:else}
    <div class="take">
      <textarea bind:this={field} bind:value={entry} rows="1" placeholder="Paste a link or text" aria-label="A link or text to read" spellcheck="false" onkeydown={keyed} oninput={(e) => grow(e.currentTarget)}></textarea>
      <span class="ways"><button class="choose" onclick={chooseFile}>Choose a file</button><span class="or">or drop one on the window</span></span>
    </div>

    <!-- The one you just asked for: making it, then playing it, in the same place. -->
    {#if cur}
      <div class="check now" aria-live="polite">
        <span class="k">{cur.episode ? 'Playing now' : curRead?.state === 'failed' ? 'Couldn’t make the episode' : curRead?.state === 'queued' ? 'Waiting its turn' : 'Making the episode'}</span>
        <div class="ep">
          <Art id={0} kind="items" page={cur.title} size={56} />
          <span class="name wrap">{cur.title}</span>
        </div>
        {#if cur.episode}
          <span class="lede">It's in your Read to me list too.</span>
          <span class="go">
            <button class="read" onclick={() => player.toggle()}>{now.episode?.id === cur.episode && now.playing ? 'Pause' : 'Play'}</button>
            <button class="cancel" onclick={done}>Done</button>
            <button class="cancel del" onclick={() => { const id = cur.episode!; done(); reads.deleteEpisode(id); }}>Delete</button>
          </span>
        {:else if curRead?.state === 'failed'}
          <span class="lede err">{curRead.error}</span>
          <span class="go"><button class="read" onclick={() => reads.now(cur.read)}>Try again</button><button class="cancel" onclick={() => { reads.discard(cur.read); done(); }}>Remove</button></span>
        {:else}
          <span class="meter"><i style:width="{pct(curRead)}%"></i></span>
          <span class="size">{curRead?.state === 'queued' ? 'Another one is being made first.' : pct(curRead) ? `${pct(curRead)}% · it plays when it's done` : 'Starting · it plays when it’s done'}</span>
          <span class="go"><button class="cancel" onclick={() => { reads.discard(cur.read); done(); }}>Cancel</button></span>
        {/if}
      </div>
    {/if}
  {/if}

  {#if others.length}
    <div class="day">Also being made</div>
    {#each others as r (r.id)}
      <div class="row making" class:failed={r.state === 'failed'}>
        <Art id={r.show_id} kind="items" page={r.title} size={34} />
        <span class="txt"><span class="t">{r.title}</span>
          {#if r.state === 'failed'}
            <span class="m err"><span>{r.error}</span> <button class="act" onclick={() => reads.now(r.id)}>Try again</button> <button class="rm" onclick={() => reads.discard(r.id)}>· Remove</button></span>
          {:else}
            <span class="m sub">{r.show_title} · {r.state === 'queued' ? 'Next' : `${pct(r)}%`}<button class="stop" onclick={() => reads.stop(r.id)}>· Stop</button></span>
          {/if}
        </span>
        {#if r.state === 'reading'}<span class="bar"><i style:width="{pct(r)}%"></i></span>{/if}
      </div>
    {/each}
  {/if}

  <!-- Your latest episodes: play, move or delete each one right here, without opening its show. -->
  {#if ready.length}
    <div class="day">Recently added</div>
    {#each ready as e (e.id)}
      {@const movable = (e.show_kind === 'read-to-me' || e.show_kind === 'items') && lists.length > 1}
      <div class="row ready" class:playing={now.episode?.id === e.id} class:heard={e.played && now.episode?.id !== e.id}>
        <button class="hit" onclick={() => play(e.id)} aria-label="Play {e.title}"></button>
        <Art id={e.show_id} kind={e.show_kind} page={e.title} size={34} />
        <span class="txt"><span class="t">{e.title}</span>
          <span class="m sub">{e.show_title} · <span class="num">{length(e)}</span> <span class="go-play">{now.episode?.id === e.id ? (now.playing ? 'Playing' : 'Paused') : e.played ? 'Heard' : 'Play'}</span></span>
          <span class="acts">
            {#if movable}<select class="move" aria-label="Move {e.title} to another show" value="" onchange={(ev) => { const to = Number(ev.currentTarget.value); if (to) reads.move(e.id, to); }}><option value="" disabled>Move to…</option>{#each lists.filter((s) => s.id !== e.show_id) as s (s.id)}<option value={s.id}>{s.title}</option>{/each}</select>{/if}
            <button class="del" onclick={() => { if (cur?.episode === e.id) done(); reads.deleteEpisode(e.id); }}>Delete</button>
          </span></span>
      </div>
    {/each}
  {/if}

  <div class="day">Your shows</div>
  {#each rtm.shows as s (s.id)}
    <button class="row show" onclick={() => (ui.yours = { id: s.id, back: 'read' })} title="Open {s.title}">
      <Art id={s.id} kind={s.kind} name={s.title} size={40} />
      <span class="txt"><span class="t">{s.title}</span>
        <span class="m sub">{#if busy(s)}<span class="new">Making an episode · </span>{:else if waiting(s)}<span class="new">{waiting(s)} ready to read · </span>{:else if s.fresh}<span class="new">{s.fresh} new · </span>{/if}<span class="src">{about(s)}</span></span></span>
    </button>
  {/each}
  {#if naming}
    <!-- svelte-ignore a11y_autofocus -->
    <input class="name sm newshow" bind:value={showName} placeholder="Name the show" aria-label="New show name" spellcheck="false" autofocus
      onkeydown={(e) => { if (e.key === 'Enter') newShow(); if (e.key === 'Escape') { naming = false; showName = ''; } }} onblur={newShow} />
  {:else}
    <span class="ways more"><button class="choose" onclick={() => (naming = true)}>New show</button><button class="choose" onclick={chooseFolder}>Show from a folder</button></span>
  {/if}
  {#if !rtm.shows.some((s) => s.kind === 'feed' || s.kind === 'folder')}
    <p class="quiet tip">A show from a folder, or from a blog's address pasted above, gets new episodes by itself.</p>
  {/if}
</div>

<style>
  .show-h { font-weight: 250; font-size: 24px; line-height: 1.2; margin: 4px 0 8px; }
  .show-p { color: var(--text-dim); font-size: 13.5px; line-height: 1.5; margin: 0 0 14px; max-width: 38ch; }
  .take { display: grid; gap: 10px; margin: 6px 0 22px; }
  .ways { display: flex; gap: 14px; align-items: baseline; flex-wrap: wrap; }
  .or { font-size: 12.5px; color: var(--text-faint); }
  .posts { list-style: none; margin: 2px 0; padding: 0; display: grid; gap: 4px; }
  .posts li { font-size: 13px; color: var(--text-dim); white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .posts li::before { content: '·'; color: var(--text-faint); margin-right: 8px; }
  .words { display: flex; gap: 14px; flex-wrap: wrap; margin-top: -2px; }
  .words button { font-size: 13.5px; color: var(--text-faint); transition: color var(--dur-base) var(--ease-hover); }
  .words button:hover { color: var(--text-dim); }
  .words button[aria-pressed="true"] { color: var(--text); }
  textarea { all: unset; box-sizing: border-box; flex: 1; min-width: 0; font: 400 13.5px/1.45 var(--font-ui); color: var(--text); border-bottom: 1px solid var(--line);
    padding: 4px 0 6px; resize: none; max-height: 120px; overflow-y: auto; scrollbar-width: none; white-space: pre-wrap; overflow-wrap: anywhere; caret-color: var(--accent);
    transition: border-color var(--dur-base) var(--ease-hover); }
  textarea:focus { border-bottom-color: var(--accent); }
  textarea::placeholder { color: var(--text-faint); }
  .choose { font-size: 13px; color: var(--text-dim); white-space: nowrap; transition: color var(--dur-base) var(--ease-hover); }
  .choose:hover { color: var(--text); }
  .busy { margin: 6px 0 22px; }

  .check { margin: 0 0 24px; padding: 14px 0 16px; border-top: 1px solid var(--hair); border-bottom: 1px solid var(--hair); display: grid; gap: 8px; }
  .k { font-family: var(--font-mono); font-size: 10.5px; letter-spacing: 0.1em; color: var(--text-faint); text-transform: uppercase; }
  .now .k { color: var(--accent); }
  /* the episode to be: its page beside its title, which wraps rather than running off the edge */
  .ep { display: grid; grid-template-columns: 56px minmax(0, 1fr); gap: 14px; align-items: start; margin: 2px 0 2px; }
  .name { font: 250 19px/1.3 var(--font-ui); letter-spacing: -0.005em; color: var(--text); }
  .name.wrap { overflow-wrap: anywhere; display: -webkit-box; -webkit-line-clamp: 3; line-clamp: 3; -webkit-box-orient: vertical; overflow: hidden; }
  textarea.name { font: 250 19px/1.3 var(--font-ui); border-bottom: 1px solid transparent; padding: 0 0 3px; max-height: calc(1.3em * 3 + 3px); }
  textarea.name:hover { border-bottom-color: var(--hair); }
  textarea.name:focus { border-bottom-color: var(--accent); }
  input.name { all: unset; box-sizing: border-box; width: 100%; font: 250 19px/1.3 var(--font-ui); letter-spacing: -0.005em; color: var(--text);
    border-bottom: 1px solid transparent; padding: 0 0 3px; caret-color: var(--accent); transition: border-color var(--dur-base) var(--ease-hover); }
  input.name:hover { border-bottom-color: var(--hair); }
  input.name:focus { border-bottom-color: var(--accent); }
  input.name.sm { font-size: 15px; border-bottom-color: var(--line); }
  input.name::placeholder { color: var(--text-faint); }
  .from { font-size: 12.5px; color: var(--text-faint); white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .size, .by, .to, .lede { font-size: 13.5px; color: var(--text-mid); }
  .lede { line-height: 1.5; margin: 0; max-width: 40ch; }
  .lede.err { color: var(--text-dim); }
  .hint { font-size: 12px; color: var(--text-faint); }
  .meter { display: block; height: 2px; background: var(--hair); border-radius: 1px; overflow: hidden; margin: 6px 0 2px; }
  .meter i { display: block; height: 100%; background: var(--accent); transition: width 0.5s var(--ease); }
  select { all: unset; cursor: pointer; color: var(--text); border-bottom: 1px solid var(--line); padding: 0 16px 2px 0; margin-left: 4px; max-width: 220px;
    white-space: nowrap; overflow: hidden; text-overflow: ellipsis;
    background: linear-gradient(45deg, transparent 50%, var(--text-faint) 50%) right 4px top 55% / 5px 5px no-repeat,
      linear-gradient(-45deg, transparent 50%, var(--text-faint) 50%) right 0 top 55% / 5px 5px no-repeat; }
  .ch { font-size: 13.5px; color: var(--text-faint); transition: color var(--dur-base) var(--ease-hover); }
  .ch:hover { color: var(--text); }
  .go { display: flex; gap: 22px; margin-top: 6px; font-size: 14px; }
  .go button { font-size: 14px; }
  .read { color: var(--accent); }
  .cancel { color: var(--text-faint); transition: color var(--dur-base) var(--ease-hover); }
  .cancel:hover { color: var(--text); }

  .day { font-size: 12px; color: var(--text-faint); margin: 22px 0 2px; }
  .row { position: relative; display: grid; grid-template-columns: 34px minmax(0, 1fr); gap: 14px; align-items: center; width: 100%; text-align: left; padding: 9px 0; }
  .row.show { grid-template-columns: 40px minmax(0, 1fr); padding: 10px 0; }
  .txt { min-width: 0; }
  .t { display: block; font-size: 14.5px; line-height: 1.35; color: var(--text); white-space: nowrap; overflow: hidden; text-overflow: ellipsis; transition: color var(--dur-base) var(--ease-hover); }
  .m { display: block; margin-top: 1px; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  button.row:hover .t { color: var(--accent); }
  .row.making .t { color: var(--text-mid); }
  .src { color: var(--text-faint); }
  .go-play { color: var(--accent); margin-left: 4px; }
  .heard .go-play { color: var(--text-faint); }
  .row.heard .t { opacity: 0.55; }
  /* the row plays; its own words (Move to, Delete) sit above the hit area */
  .row.ready { align-items: start; }
  .hit { position: absolute; inset: 0; z-index: 0; }
  .row.ready .txt { position: relative; z-index: 1; pointer-events: none; }
  .row.ready:hover .t { color: var(--accent); }
  .acts { display: flex; gap: 14px; margin-top: 3px; font-size: 12.5px; pointer-events: auto; }
  .acts .move { all: unset; cursor: pointer; color: var(--text-faint); transition: color var(--dur-base) var(--ease-hover); }
  .acts .move:hover { color: var(--text); }
  .del { font-size: inherit; color: var(--text-faint); transition: color var(--dur-base) var(--ease-hover); }
  .del:hover { color: var(--failed); }
  .row.playing .t { color: var(--accent); }
  .new { color: var(--accent); }
  .bar { position: absolute; left: 48px; right: 0; bottom: 2px; height: 1px; background: var(--hair); overflow: hidden; }
  .bar i { display: block; height: 100%; background: var(--accent); transition: width 0.5s var(--ease); }
  .stop, .rm { font-size: inherit; color: var(--text-faint); margin-left: 6px; opacity: 0; transition: opacity var(--dur-base) var(--ease-hover), color var(--dur-base) var(--ease-hover); }
  .row:hover .stop, .row:hover .rm, .stop:focus-visible, .rm:focus-visible { opacity: 1; }
  .stop:hover { color: var(--text); }
  .rm:hover { color: var(--failed); }
  .row.failed .m { white-space: normal; line-height: 1.45; font-size: 12.5px; color: var(--text-dim); }
  .act { font-size: inherit; color: var(--accent); margin-left: 2px; }
  .more { display: block; margin: 8px 0 0; }
  .newshow { margin: 8px 0 0; }
  .quiet { color: var(--text-dim); font-size: 13.5px; line-height: 1.5; max-width: 34ch; margin: 8px 0 0; }
  .tip { font-size: 12.5px; color: var(--text-faint); margin-top: 14px; }
  @media (prefers-reduced-motion: reduce) { .bar i, .meter i { transition: none; } }
</style>

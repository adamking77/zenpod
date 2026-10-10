<script lang="ts">
  import { tick } from 'svelte';
  import { open as pick } from '@tauri-apps/plugin-dialog';
  import Art from '$lib/Art.svelte';
  import { home } from '$lib/home';
  import { ui } from '$lib/ui.svelte';
  import { makeShow, mins, readCard, reads, rtm, take, takeFile, takeSource, voiceLabel, type Read } from '$lib/reads.svelte';
  import { api, length, type Show } from '$lib/api';
  import { now, player } from '$lib/now.svelte';

  let field = $state<HTMLTextAreaElement>(), entry = $state('');
  // The field takes the cursor whenever the pane is asked for (R, the icon, a card put away).
  $effect(() => { rtm.focus; if (!rtm.card) tick().then(() => field?.focus({ preventScroll: true })); });

  function keyed(e: KeyboardEvent) {
    if (e.key === 'Enter' && !e.shiftKey) { e.preventDefault(); const v = entry; entry = ''; take(v); }
  }
  function grow() { if (field) { field.style.height = 'auto'; field.style.height = `${field.scrollHeight}px`; } }

  async function chooseFolder() {
    const d = await pick({ directory: true, multiple: false, title: 'Make a show from a folder' }).catch(() => null);
    if (typeof d === 'string') takeSource('folder', d);
  }
  async function chooseFile() {
    const f = await pick({ multiple: false, title: 'Read to me', filters: [{ name: 'Documents', extensions: ['pdf', 'md', 'markdown', 'txt', 'docx', 'doc', 'rtf', 'html', 'htm'] }] }).catch(() => null);
    if (typeof f === 'string') takeFile(f);
  }

  const working = $derived(rtm.reads.filter((r) => r.state === 'reading' || r.state === 'queued' || r.state === 'failed'));
  const cardShow = $derived(rtm.card?.kind === 'item' && rtm.card.to !== 'new' ? rtm.shows.find((s) => s.id === (rtm.card as { to: number }).to) : null);

  function showLine(s: Show) {
    const mine = rtm.reads.filter((r) => r.show_id === s.id);
    const lead = mine.some((r) => r.state === 'reading' || r.state === 'queued') ? 'Reading… · ' : s.fresh ? `${s.fresh} new · ` : '';
    const src = s.kind === 'feed' || s.kind === 'folder' ? `${s.kind === 'folder' ? home(s.source ?? '') : (s.source ?? '').replace(/^https?:\/\//, '')} · ${s.auto ? 'read automatically' : `new ${s.kind === 'feed' ? 'posts' : 'files'} wait for you`}` : 'Things you add one at a time';
    return { lead, src };
  }
  // A podcast pasted here is followed the usual way, then shown in Following.
  async function follow(url: string) {
    rtm.card = { kind: 'busy', what: 'Following…' };
    try { const [id] = await api.addShow(url); rtm.card = null; ui.reveal = id; ui.tab = 'following'; }
    catch (e) { rtm.card = { kind: 'error', message: String(e) }; }
  }
  const progress = (r: Read) => r.state === 'queued' ? 'Next to read' : r.done && r.pieces ? `Reading · ${Math.round((r.done / r.pieces) * 100)}%` : 'Starting to read';
  // After the field, Enter again reads: the card's Read takes the focus.
  let readButton = $state<HTMLButtonElement>();
  $effect(() => { if (rtm.card?.kind === 'item') tick().then(() => readButton?.focus({ preventScroll: true })); });
  const play = (id: number) => player.choose(id, rtm.ready.map((e) => e.id));
</script>

<div class="pane">
  <h2 class="show-h">Read to me</h2>
  <p class="show-p">Turn articles, feeds, files and folders into shows of your own.</p>

  {#if !rtm.card}
    <div class="take">
      <textarea bind:this={field} bind:value={entry} rows="1" placeholder="Paste a link, a feed or text" aria-label="A link, a feed or text to read" spellcheck="false" onkeydown={keyed} oninput={grow}></textarea>
      <span class="ways"><button class="choose" onclick={chooseFile}>Choose file</button><button class="choose" onclick={chooseFolder}>Choose folder</button><span class="or">or drop one on the window</span></span>
    </div>
  {:else if rtm.card.kind === 'busy'}
    <p class="quiet busy" aria-live="polite">{rtm.card.what}</p>
  {:else if rtm.card.kind === 'error'}
    <div class="check">
      <p class="lede">{rtm.card.message}</p>
      <span class="go"><button class="cancel" onclick={() => { rtm.card = null; rtm.focus++; }}>Try something else</button></span>
    </div>
  {:else if rtm.card.kind === 'podcast'}
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
  {:else if rtm.card.kind === 'feed' || rtm.card.kind === 'folder'}
    {@const c = rtm.card}
    {@const word = c.kind === 'feed' ? 'posts' : 'files'}
    <div class="check">
      <span class="k">{c.kind === 'feed' ? 'A feed of articles' : 'A folder'}</span>
      <input class="name" bind:value={c.name} aria-label="Show name" spellcheck="false" onkeydown={(e) => e.key === 'Enter' && makeShow()} />
      <span class="from">{c.kind === 'folder' ? home(c.source) : c.source.replace(/^https?:\/\//, '')}</span>
      <span class="size">{c.items.length} recent {word}{c.count > c.items.length ? ` of ${c.count}` : ''}</span>
      <ul class="posts">{#each c.items as t}<li>{t}</li>{/each}</ul>
      <span class="to">New {word}</span>
      <span class="words">
        <button aria-pressed={!c.auto} onclick={() => (c.auto = false)}>Wait for me</button>
        <button aria-pressed={c.auto} onclick={() => (c.auto = true)}>Read them automatically</button>
      </span>
      <span class="by">Read by {voiceLabel()} <button class="ch" onclick={() => { ui.tab = 'settings'; }}>· Change</button></span>
      <span class="go"><button class="read" onclick={makeShow}>Make it a show</button><button class="cancel" onclick={() => { rtm.card = null; rtm.focus++; }}>Cancel</button></span>
    </div>
  {:else if rtm.card.kind === 'item'}
    {@const c = rtm.card}
    <div class="check">
      <span class="k">Ready to read</span>
      <input class="name" bind:value={c.title} aria-label="Title" spellcheck="false" onkeydown={(e) => e.key === 'Enter' && readCard()} />
      <span class="from">{c.from}</span>
      <span class="size"><span class="num">{c.text.length.toLocaleString('en-GB')}</span> characters · about {mins(c.text.length)} min</span>
      <span class="to">Goes into
        <select aria-label="Show" bind:value={c.to}>
          {#each rtm.shows.filter((s) => s.kind !== 'feed' && s.kind !== 'folder') as s (s.id)}<option value={s.id}>{s.title}</option>{/each}
          <option value="new">New show…</option>
        </select>
      </span>
      {#if c.to === 'new'}
        <!-- svelte-ignore a11y_autofocus -->
        <input class="name sm" bind:value={c.newName} placeholder="Name the new show" aria-label="New show name" spellcheck="false" autofocus onkeydown={(e) => e.key === 'Enter' && readCard()} />
      {/if}
      <span class="by">Read by {voiceLabel(cardShow)} <button class="ch" onclick={() => { ui.tab = 'settings'; }}>· Change</button></span>
      <span class="go"><button class="read" bind:this={readButton} onclick={readCard}>Read</button><button class="cancel" onclick={() => { rtm.card = null; rtm.focus++; }}>Cancel</button></span>
    </div>
  {/if}

  {#if working.length}
    <div class="day">Reading</div>
    {#each working as r (r.id)}
      <div class="row making" class:failed={r.state === 'failed'}>
        <Art id={r.show_id} kind="items" page={r.title} size={34} />
        <span class="txt"><span class="t">{r.title}</span>
          {#if r.state === 'failed'}
            <span class="m err"><span>{r.error}</span> <button class="act" onclick={() => reads.now(r.id)}>Try again</button> <button class="rm" onclick={() => reads.discard(r.id)}>· Remove</button></span>
          {:else}
            <span class="m sub">{r.show_title} · {progress(r)}<button class="stop" onclick={() => reads.stop(r.id)}>· Stop</button></span>
          {/if}
        </span>
        {#if r.state === 'reading'}<span class="bar"><i style:width="{(r.done / Math.max(1, r.pieces)) * 100}%"></i></span>{/if}
      </div>
    {/each}
  {/if}

  <!-- A finished reading lands here, a click from playing; it leaves once heard. -->
  {#if rtm.ready.length}
    <div class="day">Ready to listen</div>
    {#each rtm.ready as e (e.id)}
      <button class="row ready" class:playing={now.episode?.id === e.id} onclick={() => play(e.id)}>
        <Art id={e.show_id} kind={e.show_kind} page={e.title} size={34} />
        <span class="txt"><span class="t">{e.title}</span>
          <span class="m sub">{e.show_title} · <span class="num">{length(e)}</span> <span class="go-play">{now.episode?.id === e.id ? (now.playing ? 'Playing' : 'Paused') : 'Play'}</span></span></span>
      </button>
    {/each}
  {/if}

  <div class="day">Your shows</div>
  {#each rtm.shows as s (s.id)}
    {@const l = showLine(s)}
    <button class="row show" onclick={() => (ui.yours = { id: s.id, back: 'read' })}>
      <Art id={s.id} kind={s.kind} name={s.title} size={40} />
      <span class="txt"><span class="t">{s.title}</span><span class="m sub">{#if l.lead}<span class="new">{l.lead}</span>{/if}<span class="src">{l.src}</span></span></span>
    </button>
  {:else}
    <p class="quiet">Anything you add becomes an episode, with its shape, a transcript and chapters.</p>
  {/each}
</div>

<style>
  .show-h { font-weight: 250; font-size: 24px; line-height: 1.2; margin: 4px 0 8px; }
  .show-p { color: var(--text-dim); font-size: 13.5px; line-height: 1.5; margin: 0 0 14px; max-width: 38ch; }
  .take { display: grid; gap: 10px; margin: 6px 0 22px; }
  .ways { display: flex; gap: 8px; align-items: baseline; flex-wrap: wrap; }
  .ways { gap: 14px; }
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
  .name { font: 250 19px/1.3 var(--font-ui); letter-spacing: -0.005em; color: var(--text); }
  input.name { all: unset; box-sizing: border-box; width: 100%; font: 250 19px/1.3 var(--font-ui); letter-spacing: -0.005em; color: var(--text);
    border-bottom: 1px solid transparent; padding: 0 0 3px; caret-color: var(--accent); transition: border-color var(--dur-base) var(--ease-hover); }
  input.name:hover { border-bottom-color: var(--hair); }
  input.name:focus { border-bottom-color: var(--accent); }
  input.name.sm { font-size: 15px; border-bottom-color: var(--line); }
  input.name::placeholder { color: var(--text-faint); }
  .from { font-size: 12.5px; color: var(--text-faint); white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .size, .by, .to, .lede { font-size: 13.5px; color: var(--text-mid); }
  .lede { line-height: 1.5; margin: 0; max-width: 40ch; }
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
  .quiet { color: var(--text-dim); font-size: 13.5px; line-height: 1.5; max-width: 34ch; margin: 8px 0 0; }
  @media (prefers-reduced-motion: reduce) { .bar i { transition: none; } }
</style>

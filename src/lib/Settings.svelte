<script lang="ts">
  import { api, type Imported } from '$lib/api';
  import { ACCENTS, prefs, setPref } from '$lib/prefs.svelte';
  import { ui } from '$lib/ui.svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { open as pick } from '@tauri-apps/plugin-dialog';
  import { home } from '$lib/home';

  // Notes folder: each episode's notes also written there as Markdown.
  let folderNote = $state('');
  async function chooseFolder() {
    const dir = await pick({ directory: true, multiple: false, title: 'Notes folder' }).catch(() => null);
    if (typeof dir !== 'string') return;
    const n = await invoke<number>('set_notes_folder', { path: dir });
    folderNote = n ? `${n} episode ${n === 1 ? 'file' : 'files'} written.` : 'Notes will be written here.';
  }
  async function stopWriting() {
    await invoke('set_notes_folder', { path: null });
    folderNote = 'Notes stay in Zenpod. The files already written are left where they are.';
  }
  let field: HTMLInputElement;
  $effect(() => { if (ui.paste && field) { field.focus(); ui.paste = false; } });

  let address = $state('');
  let note = $state('');
  let busy = $state(false);

  // Say what arrived, then show it: Following leads with the new shows.
  const said = (i: Imported) => {
    const parts = [i.added ? `${i.added} new ${i.added === 1 ? 'show' : 'shows'}` : 'No new shows'];
    if (i.had) parts.push(`${i.had} you already had`);
    if (i.spotify_only) parts.push(`${i.spotify_only} only on Spotify`);
    const note = parts.join(', ') + '.';
    ui.arrived = { note, ids: i.ids, unreached: i.unreached };
    ui.tab = 'following';
    return note;
  };

  async function run(task: () => Promise<string>) {
    busy = true;
    note = 'Bringing them in…';
    try { note = await task(); } catch (e) { note = String(e); } finally { busy = false; }
  }

  function add(ev: SubmitEvent) {
    ev.preventDefault();
    const v = address.trim();
    if (!v) return;
    run(async () => { const [id, t] = await api.addShow(v); address = ''; ui.reveal = id; ui.tab = 'following'; return `${t} is in Following.`; });
  }

  function file(kind: 'opml' | 'spotify') {
    return async (ev: Event) => {
      const input = ev.currentTarget as HTMLInputElement, f = input.files?.[0];
      input.value = '';
      if (!f) return;
      const text = await f.text();
      run(async () => said(await (kind === 'opml' ? api.importOpml(text) : api.importSpotify(text))));
    };
  }
</script>

<div class="set">
  <h2>Settings</h2>
  <section>
    <div class="lbl">Accent <b>{prefs.accent}</b></div>
    <div class="sw" role="group" aria-label="Accent colour">
      {#each ACCENTS as a}
        <button aria-label={a} aria-pressed={prefs.accent === a} style:--c="var(--{a.toLowerCase()})" onclick={() => setPref('accent', a)}></button>
      {/each}
    </div>
  </section>
  <section>
    <div class="lbl">Appearance</div>
    <div class="words" role="group" aria-label="Appearance">
      <button aria-pressed={prefs.look === ''} onclick={() => setPref('look', '')}>Follow the system</button>
      <button aria-pressed={prefs.look === 'day'} onclick={() => setPref('look', 'day')}>Day</button>
      <button aria-pressed={prefs.look === 'night'} onclick={() => setPref('look', 'night')}>Night</button>
    </div>
  </section>
  <section>
    <div class="lbl">Light</div>
    <div class="words" role="group" aria-label="Light">
      <button aria-pressed={prefs.light === 'cast'} onclick={() => setPref('light', 'cast')}>Cast from the artwork</button>
      <button aria-pressed={prefs.light === 'flat'} onclick={() => setPref('light', 'flat')}>Flat</button>
    </div>
  </section>
  <section>
    <div class="lbl">When an episode ends</div>
    <div class="words" role="group" aria-label="When an episode ends">
      <button aria-pressed={prefs.autoplay !== 'off'} onclick={() => setPref('autoplay', 'on')}>Play the next one</button>
      <button aria-pressed={prefs.autoplay === 'off'} onclick={() => setPref('autoplay', 'off')}>Stop</button>
    </div>
  </section>
  <section>
    <div class="lbl">Notes folder</div>
    {#if prefs.notes_folder}
      <p class="path">{home(prefs.notes_folder)}</p>
      <p class="hint">Each episode you take notes in gets one Markdown file here, rewritten whenever its notes change.</p>
      <div class="words"><button class="act" onclick={chooseFolder}>Change</button><button class="act stop" onclick={stopWriting}>Stop writing</button></div>
    {:else}
      <p class="hint">Notes stay in Zenpod. Choose a folder and each episode's notes are also written there as a Markdown file.</p>
      <div class="words"><button class="act" onclick={chooseFolder}>Choose a folder…</button></div>
    {/if}
    {#if folderNote}<p class="done" aria-live="polite">{folderNote}</p>{/if}
  </section>
  <section>
    <div class="lbl">Bring your shows</div>
    <div class="imp">
      <form onsubmit={add}>
        <input bind:this={field} bind:value={address} disabled={busy} placeholder="A feed address, a show's name or an Apple Podcasts link" aria-label="A feed address, a show's name or an Apple Podcasts link" />
      </form>
      <button class="from" disabled={busy} onclick={() => run(async () => said(await api.importApple()))}><span>From Apple Podcasts</span><span class="sub">The shows you follow in Apple's app on this Mac</span></button>
      <label><span>From a list of shows</span><span class="sub">An OPML file, from Overcast, Pocket Casts or most other podcast apps</span>
        <input type="file" accept=".opml,.xml,text/xml" disabled={busy} onchange={file('opml')} /></label>
      <label><span>From your Spotify data</span><span class="sub">YourLibrary.json, from Spotify's privacy page</span>
        <input type="file" accept=".json,application/json" disabled={busy} onchange={file('spotify')} /></label>
    </div>
    {#if note}<p class="done" aria-live="polite">{note}</p>{/if}
  </section>
</div>

<style>
  h2 { font-weight: 250; font-size: 24px; line-height: 1.2; margin: 4px 0 26px; }
  section { padding: 0 0 22px; margin: 0 0 22px; border-bottom: 1px solid var(--hair); }
  section:last-child { border-bottom: 0; }
  .lbl { display: flex; justify-content: space-between; align-items: baseline; font-size: 12px; color: var(--text-faint); margin-bottom: 12px; }
  .lbl b { font-weight: 400; color: var(--text-dim); }
  .sw { display: grid; grid-template-columns: repeat(7, 22px); gap: 14px 18px; }
  .sw button { width: 22px; height: 22px; border-radius: 50%; background: var(--c); position: relative; transition: transform 0.14s var(--ease); }
  .sw button:hover { transform: scale(1.12); }
  .sw button[aria-pressed="true"]::after { content: ""; position: absolute; inset: -5px; border-radius: 50%; box-shadow: 0 0 0 1px var(--c); }
  .words { display: flex; gap: 18px; flex-wrap: wrap; }
  .words button { font-size: 14px; color: var(--text-faint); transition: color 0.14s ease; }
  .words button:hover { color: var(--text-dim); }
  .words button[aria-pressed="true"] { color: var(--text); }
  .imp { display: grid; gap: 12px; }
  .imp label, .imp .from { font-size: 14px; color: var(--text); cursor: pointer; display: grid; gap: 1px; position: relative; }
  .imp label:hover span:first-child, .imp label:focus-within span:first-child, .imp .from:hover span:first-child, .imp .from:focus-visible span:first-child { color: var(--accent); }
  .imp .from { text-align: left; }
  .imp input[type="file"] { position: absolute; opacity: 0; width: 1px; height: 1px; }
  .imp input:not([type]) { font: inherit; font-size: 14px; color: var(--text); background: none; border: 0; border-bottom: 1px solid var(--line); padding: 4px 0 6px; outline: none; width: 100%; }
  .imp input:not([type]):focus { border-bottom-color: var(--accent); }
  .imp input::placeholder { color: var(--text-faint); }
  .path { font-family: var(--font-mono); font-size: 12px; color: var(--text); margin: 0 0 6px; overflow-wrap: anywhere; }
  .hint { font-size: 12.5px; line-height: 1.5; color: var(--text-dim); margin: 0 0 12px; max-width: 40ch; }
  .words button.act { color: var(--text); }
  .words button.act:hover { color: var(--accent); }
  .words button.stop { color: var(--text-dim); }
  .words button.stop:hover { color: var(--failed); }
  .done { font-size: 13px; color: var(--text-dim); margin: 12px 0 0; max-width: 40ch; }
</style>

<script lang="ts">
  import { api, type Imported } from '$lib/api';
  import { ACCENTS, prefs, setPref } from '$lib/prefs.svelte';
  import { ui } from '$lib/ui.svelte';
  let field: HTMLInputElement;
  $effect(() => { if (ui.paste && field) { field.focus(); ui.paste = false; } });

  let address = $state('');
  let note = $state('');
  let busy = $state(false);

  const said = (i: Imported) => {
    const parts = [`${i.added} ${i.added === 1 ? 'show is' : 'shows are'} in Following`];
    if (i.had) parts.push(`${i.had} you already had`);
    if (i.spotify_only) parts.push(`${i.spotify_only} only on Spotify`);
    if (i.failed) parts.push(`${i.failed} couldn't be reached`);
    return parts.join(', ') + '.';
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
    run(async () => { const t = await api.addShow(v); address = ''; return `${t} is in Following.`; });
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
    <div class="lbl">Bring your shows</div>
    <div class="imp">
      <form onsubmit={add}>
        <input bind:this={field} bind:value={address} disabled={busy} placeholder="A feed address, or a show's name" aria-label="A feed address, or a show's name" />
      </form>
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
  .imp label { font-size: 14px; color: var(--text); cursor: pointer; display: grid; gap: 1px; position: relative; }
  .imp label:hover span:first-child, .imp label:focus-within span:first-child { color: var(--accent); }
  .imp input[type="file"] { position: absolute; opacity: 0; width: 1px; height: 1px; }
  .imp input:not([type]) { font: inherit; font-size: 14px; color: var(--text); background: none; border: 0; border-bottom: 1px solid var(--line); padding: 4px 0 6px; outline: none; width: 100%; }
  .imp input:not([type]):focus { border-bottom-color: var(--accent); }
  .imp input::placeholder { color: var(--text-faint); }
  .done { font-size: 13px; color: var(--text-dim); margin: 12px 0 0; max-width: 40ch; }
</style>

<script lang="ts">
  // The playing episode's notes, in the episode pane: play, write, edit or remove each one.
  import { onMount, tick } from 'svelte';
  import { fmt } from '$lib/api';
  import { now, player, position } from '$lib/now.svelte';
  import { marks, notesFile, removeNote, setNote, type Mark } from '$lib/marks.svelte';
  import { prefs } from '$lib/prefs.svelte';
  import { home } from '$lib/home';

  let t = $state(0), editing = $state<number | null>(null), draft = $state(''), armed = $state<number | null>(null), file = $state<string | null>(null);
  let field = $state<HTMLTextAreaElement>();
  onMount(() => { let r = 0; const f = () => { t = position(); r = requestAnimationFrame(f); }; f(); return () => cancelAnimationFrame(r); });

  // Where these notes are written, kept current with the folder and the list.
  $effect(() => { void [prefs.notes_folder, marks.list.length]; const id = now.episode?.id; if (id) notesFile(id).then((f) => (file = f)); });

  const play = (m: Mark) => { player.seek(m.start); if (!now.playing) player.toggle(); };
  async function edit(m: Mark) {
    editing = m.id; draft = m.note; armed = null;
    await tick();
    if (field) { grow(); field.focus(); field.setSelectionRange(draft.length, draft.length); }
  }
  const grow = () => { if (field) { field.style.height = 'auto'; field.style.height = `${field.scrollHeight}px`; } };
  function done() { if (editing != null) setNote(editing, draft); editing = null; }
  function keyed(e: KeyboardEvent) { if ((e.key === 'Enter' && !e.shiftKey) || e.key === 'Escape') { e.preventDefault(); done(); } }
  function remove(m: Mark) { if (armed === m.id) { armed = null; removeNote(m.id); } else armed = m.id; }
</script>

{#if marks.list.length}
  <ol class="notes">
    {#each marks.list as m (m.id)}
      <!-- The whole note plays it, as in the prototype; its own buttons and field keep their clicks. -->
      <!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_noninteractive_element_interactions -->
      <li class:on={t >= m.start && t <= m.end} class:armed={armed === m.id}
        onclick={(e) => { if (editing !== m.id && !(e.target as Element).closest('button, textarea') && !getSelection()?.toString()) play(m); }}>
        <button class="when" onclick={() => play(m)}><span class="num">{fmt(m.start)}–{fmt(m.end)}</span>{#if m.chapter}<span class="ch">{m.chapter}</span>{/if}</button>
        {#if m.quote}<q>{m.quote}</q>{/if}
        {#if editing === m.id}
          <span class="field"><textarea bind:this={field} bind:value={draft} rows="1" placeholder="Add a note" aria-label="Your note" oninput={grow} onkeydown={keyed} onblur={done}></textarea></span>
        {:else if m.note}
          <p class="words">{m.note}</p>
        {:else}
          <button class="add" onclick={() => edit(m)}>Add a note</button>
        {/if}
        <span class="acts">
          <button onclick={() => play(m)}>Play</button>
          {#if m.note && editing !== m.id}<button onclick={() => edit(m)}>Edit</button>{/if}
          <button class="rm" onclick={() => remove(m)}>{armed === m.id ? 'Press again to remove' : 'Remove'}</button>
        </span>
      </li>
    {/each}
  </ol>
  <p class="where">{file ? `Written to ${home(file)}` : 'Kept in Zenpod. Choose a notes folder in Settings to write these out as Markdown.'}</p>
{:else}
  <p class="quiet">No notes in this episode yet. Press M while it plays to keep the last 30 seconds, or use the note button to stop and write.</p>
{/if}

<style>
  .notes { list-style: none; padding: 0; margin: 0 0 18px; }
  li { cursor: pointer; padding: 12px 0 13px; border-bottom: 1px solid var(--hair); display: grid; gap: 6px; justify-items: start; }
  li:first-child { padding-top: 2px; }
  .when { display: flex; gap: 10px; align-items: baseline; min-width: 0; max-width: 100%; font-size: 12.5px; color: var(--text-faint); text-align: left; }
  .ch { white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .when:hover .num, li.on .when .num { color: var(--accent); }
  q { font-size: 13.5px; line-height: 1.5; color: var(--text-mid); quotes: "\201C" "\201D"; max-width: 42ch;
    display: -webkit-box; -webkit-line-clamp: 3; line-clamp: 3; -webkit-box-orient: vertical; overflow: hidden; }
  .words { margin: 0; font-size: 14px; line-height: 1.5; color: var(--text); max-width: 42ch; overflow-wrap: anywhere; user-select: text; cursor: text; }
  .add { font-size: 13px; color: var(--text-faint); transition: color var(--dur-base) var(--ease-hover); }
  .add:hover { color: var(--text-dim); }
  .field { display: block; width: 100%; border-bottom: 1px solid var(--accent); padding: 2px 0 6px; }
  textarea { display: block; width: 100%; box-sizing: border-box; margin: 0; padding: 0; border: 0; outline: none; background: none; resize: none; overflow: hidden;
    font: 400 14px/1.5 var(--font-ui); letter-spacing: 0.01em; color: var(--text); caret-color: var(--accent); }
  textarea::placeholder { color: var(--text-faint); }
  .acts { display: flex; gap: 16px; font-size: 12.5px; color: var(--text-faint); opacity: 0; transition: opacity var(--dur-base) var(--ease-hover); }
  li:hover .acts, li:focus-within .acts, li.armed .acts { opacity: 1; }
  li:hover .when .num { color: var(--accent); }
  .acts button { transition: color var(--dur-base) var(--ease-hover); }
  .acts button:hover { color: var(--text); }
  .acts .rm:hover, li.armed .rm { color: var(--failed); }
  .where { font-size: 12.5px; color: var(--text-faint); margin: 0 0 12px; max-width: 42ch; overflow-wrap: anywhere; }
  .quiet { color: var(--text-dim); font-size: 13.5px; max-width: 32ch; margin-top: 8px; }
</style>

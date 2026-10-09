<script lang="ts">
  // The Mini's note: it has no room beside its Orbit, so while you write the note takes the title's place (the note
  // itself lands on the ring). M keeps playing; typing pauses; Enter or Esc saves and gives the title back.
  import { onMount, tick } from 'svelte';
  import gsap from 'gsap';
  import { fmt } from '$lib/api';
  import { hooks, now, player } from '$lib/now.svelte';
  import { marks, plate, plateApi, removeNote, setNote, takeNote } from '$lib/marks.svelte';
  import { prefs } from '$lib/prefs.svelte';

  let { title, show }: { title: string; show: string } = $props();

  let who: HTMLElement, box: HTMLElement;
  let field = $state<HTMLTextAreaElement>(), saved = $state<HTMLElement>();
  let text = $state('');
  let resume = false, timer = 0, pending = false, early = '';
  const motion = () => (matchMedia('(prefers-reduced-motion: reduce)').matches ? 0 : 1);
  const mark = $derived(marks.list.find((m) => m.id === plate.id) ?? null);
  const writing = $derived(plate.id != null && plate.mode !== 'peek');

  async function take(byButton: boolean) {
    const carry = writing && resume;
    if (writing && plate.id != null && text.trim()) await setNote(plate.id, text);
    const wasPlaying = now.playing;
    pending = true; early = '';
    const m = await takeNote();
    if (!m) { pending = false; return; }
    if (byButton && wasPlaying) await player.toggle();
    clearTimeout(timer);
    plate.id = m.id; plate.mode = byButton ? 'typing' : 'new'; resume = carry || (byButton && wasPlaying); text = '';
    await tick();
    const d = motion();
    gsap.killTweensOf([who, box]);
    gsap.to(who, { autoAlpha: 0, y: -4, duration: 0.25 * d, ease: 'power3.out' });
    gsap.fromTo(box, { autoAlpha: 0, y: 4 }, { autoAlpha: 1, y: 0, duration: 0.3 * d, delay: 0.15 * d, ease: 'power3.out', onComplete: ready });
    // Untouched, it gives the title back on its own: the note stays, with no words.
    if (plate.mode === 'new') timer = window.setTimeout(() => { if (plate.mode === 'new') close(); }, 5000);
  }

  /** Focus once the field can be seen (WebKit draws no caret otherwise); anything typed before then goes in now. */
  async function ready() {
    if (!field || (plate.mode !== 'new' && plate.mode !== 'typing')) return;
    field.focus({ preventScroll: true });
    pending = false;
    if (early) { text = early; early = ''; await tick(); field.setSelectionRange(text.length, text.length); typed(); }
  }

  function typed() {
    if (field) { field.style.height = 'auto'; field.style.height = `${field.scrollHeight}px`; }
    if (plate.mode !== 'new') return;
    clearTimeout(timer);
    plate.mode = 'typing';
    if (now.playing) { player.toggle(); resume = true; }
  }

  function keyed(e: KeyboardEvent) {
    if (e.key === 'Enter' && !e.shiftKey) { e.preventDefault(); save(); }
    else if (e.key === 'Escape') { e.preventDefault(); if (text.trim()) save(); else close(); }
    else if (!text && plate.mode === 'new' && e.code === 'Space') { e.preventDefault(); player.toggle(); }
  }

  function blurred() { if (plate.mode === 'typing' && text.trim() && document.hasFocus()) save(); }

  async function save() {
    const id = plate.id;
    if (id == null) return;
    if (!text.trim()) return close();
    clearTimeout(timer);
    plate.mode = 'saved';
    await setNote(id, text);
    if (resume && !now.playing) player.toggle();
    resume = false;
    await tick();
    if (saved) gsap.fromTo(saved, { opacity: 0, y: 3 }, { opacity: 1, y: 0, duration: 0.35 * motion(), ease: 'power3.out' });
    timer = window.setTimeout(() => { if (plate.id === id && plate.mode === 'saved') close(); }, 1600);
  }

  function close(immediate = false) {
    clearTimeout(timer);
    if (resume && !now.playing) player.toggle();
    resume = false;
    const id = plate.id, d = immediate ? 0 : motion();
    const done = () => { if (plate.id === id) { plate.id = null; plate.mode = null; } };
    gsap.killTweensOf([who, box]);
    gsap.to(box, { autoAlpha: 0, y: 4, duration: 0.22 * d, ease: 'power3.out', onComplete: done });
    gsap.to(who, { autoAlpha: 1, y: 0, duration: 0.3 * d, delay: 0.12 * d, ease: 'power3.out' });
    if (!d) done();
  }

  async function undo() {
    const id = plate.id;
    if (id == null) return;
    close(true);
    await removeNote(id);
  }

  // Ahead of the player's keys: while a note is being made, letters (and spaces after the first) belong to it.
  function catchEarly(e: KeyboardEvent) {
    if (!pending || e.metaKey || e.ctrlKey || e.altKey || e.key.length !== 1 || (e.key === ' ' && !early)) return;
    e.preventDefault(); e.stopImmediatePropagation();
    early += e.key;
  }

  plateApi.take = take;
  // A note removed elsewhere, or another episode starting, gives the title back.
  $effect(() => { if (plate.id != null && marks.for != null && !mark) close(true); });
  let shown = now.episode?.id;
  $effect(() => { const id = now.episode?.id; if (id !== shown) { shown = id; if (plate.id != null) close(true); } });

  onMount(() => {
    hooks.note = () => void take(false);
    window.addEventListener('keydown', catchEarly, true);
    return () => { window.removeEventListener('keydown', catchEarly, true); clearTimeout(timer); plate.id = null; plate.mode = null; };
  });
</script>

<div class="swap" class:writing>
  <div class="who" bind:this={who}>
    <p class="t">{title}</p>
    <p class="s sub">{show}</p>
  </div>
  <div class="note" bind:this={box} data-no-drag role="group" aria-label="Note">
    {#if mark && writing}
      <span class="k">NOTE · {fmt(mark.start)}–{fmt(mark.end)}</span>
      {#if plate.mode === 'saved'}
        <span class="field done"><span class="words">{mark.note}</span></span>
        <span class="under"><span class="saved" bind:this={saved}>{prefs.notes_folder ? 'Saved to your notes' : 'Saved in Zenpod'}</span></span>
      {:else}
        <span class="field" class:has={text}>
          <textarea bind:this={field} bind:value={text} rows="1" placeholder="Add a note" aria-label="Your note" spellcheck="true"
            oninput={typed} onkeydown={keyed} onblur={blurred}></textarea><span class="ret" aria-hidden="true">↵</span>
        </span>
        <span class="under"><button class="undo" onmousedown={(e) => e.preventDefault()} onclick={undo}>Undo</button></span>
      {/if}
    {/if}
  </div>
</div>

<style>
  /* the title and the note take turns in the same place */
  .swap { position: relative; display: grid; margin: 2px 0 0; }
  .swap > * { grid-area: 1 / 1; }
  .t { text-align: center; font-weight: 300; font-size: 16.5px; line-height: 1.3; margin: 0 0 2px; text-wrap: balance;
    display: -webkit-box; -webkit-line-clamp: 2; line-clamp: 2; -webkit-box-orient: vertical; overflow: hidden; }
  .s { text-align: center; margin: 0; }
  .note { display: grid; justify-items: center; gap: 3px; text-align: center; visibility: hidden; opacity: 0; }
  .k { font-family: var(--font-mono); font-size: 10px; letter-spacing: 0.1em; color: var(--text-faint); }
  .field { display: flex; align-items: flex-end; gap: 6px; width: 100%; padding: 1px 0 5px; border-bottom: 1px solid var(--accent); }
  .field.done { border-bottom-color: transparent; }
  textarea, .words { flex: 1; min-width: 0; box-sizing: border-box; margin: 0; padding: 0; border: 0; outline: none; background: none; resize: none; overflow: hidden;
    font: 400 14px/1.35 var(--font-ui); letter-spacing: 0.01em; color: var(--text); text-align: center; caret-color: var(--accent);
    white-space: pre-wrap; overflow-wrap: anywhere; max-height: 2.7em; }
  /* a note wraps to two lines here, then scrolls; saved, it shows its first two */
  .words { display: -webkit-box; -webkit-line-clamp: 2; line-clamp: 2; -webkit-box-orient: vertical; }
  textarea::placeholder { color: var(--text-faint); }
  .ret { font-family: var(--font-mono); font-size: 11px; color: var(--text-faint); opacity: 0; width: 10px; transition: opacity var(--dur-base) var(--ease-hover); }
  .field.has .ret { opacity: 1; }
  .under { min-height: 16px; font-size: 12px; }
  .undo { font-size: 12px; color: var(--text-faint); transition: color var(--dur-base) var(--ease-hover); }
  .undo:hover { color: var(--failed); }
  .saved { display: inline-block; color: var(--text-dim); }
</style>

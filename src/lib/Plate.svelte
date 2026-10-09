<script lang="ts">
  // The note's plate on the main window's shape, as the film drew it: a leader out of the note, then words. No box.
  // M opens it with the field ready and keeps playing; typing pauses; Enter or Esc saves and picks playback up again.
  import { onMount, tick } from 'svelte';
  import gsap from 'gsap';
  import { fmt } from '$lib/api';
  import { hooks, now, player } from '$lib/now.svelte';
  import { marks, plate, plateApi, removeNote, setNote, shapeGeo, takeNote } from '$lib/marks.svelte';
  import { prefs } from '$lib/prefs.svelte';
  import { maxHeight, placePlate } from '$lib/plate';

  let el: HTMLDivElement, lead: SVGPathElement;
  let field = $state<HTMLTextAreaElement>(), saved = $state<HTMLElement>();
  let text = $state(''), side = $state(false);
  // Whether Zenpod paused playback for this note, and so should start it again.
  let resume = false, timer = 0, raf = 0;
  // Keys typed between M and the field being ready, so the first words of a quick note aren't lost.
  let pending = false, early = '';
  const motion = () => (matchMedia('(prefers-reduced-motion: reduce)').matches ? 0 : 1);
  const mark = $derived(marks.list.find((m) => m.id === plate.id) ?? null);

  async function show(id: number, mode: 'new' | 'typing' | 'peek', r = false) {
    clearTimeout(timer);
    gsap.killTweensOf([el, lead]);
    const fresh = plate.id !== id, d = motion();
    plate.id = id; plate.mode = mode; resume = r;
    if (mode !== 'peek') text = '';
    await tick();
    loop();
    if (fresh) {
      // As in the film: the leader draws itself out of the note, then the plate arrives at its end.
      gsap.fromTo(lead, { attr: { 'stroke-dashoffset': 1 } }, { attr: { 'stroke-dashoffset': 0 }, duration: (mode === 'peek' ? 0.3 : 0.45) * d, ease: 'power3.inOut' });
      gsap.fromTo(el, { opacity: 0 }, { opacity: 1, duration: 0.3 * d, delay: (mode === 'peek' ? 0.15 : 0.35) * d, ease: 'none', onComplete: ready });
    } else { gsap.set(el, { opacity: 1 }); gsap.set(lead, { attr: { 'stroke-dashoffset': 0 } }); ready(); }
    if (mode === 'peek') return;
    // Untouched, a quick note's plate leaves on its own: the note stays, with no words.
    if (mode === 'new') timer = window.setTimeout(() => { if (plate.mode === 'new') close(); }, 5000);
  }

  /** The field takes focus once the plate can be seen: WebKit won't draw a caret in a field focused while invisible.
   * Anything typed before then was kept, and goes in now. */
  async function ready() {
    if (!field || (plate.mode !== 'new' && plate.mode !== 'typing')) return;
    field.focus({ preventScroll: true });
    pending = false;
    if (early) { text = early; early = ''; await tick(); field.setSelectionRange(text.length, text.length); typed(); }
  }

  /** M (keeps playing) or the note button (pauses first). */
  async function take(byButton: boolean) {
    const carry = plate.mode !== 'peek' && resume;
    if (plate.id != null && plate.mode !== 'peek' && text.trim()) await setNote(plate.id, text);
    const wasPlaying = now.playing;
    pending = true; early = '';
    const m = await takeNote();
    if (!m) { pending = false; return; }
    if (byButton && wasPlaying) await player.toggle();
    show(m.id, byButton ? 'typing' : 'new', carry || (byButton && wasPlaying));
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
    else if (!text && plate.mode === 'new') {
      // Until the first word, the field leaves the player's keys alone.
      if (e.code === 'Space') { e.preventDefault(); player.toggle(); }
      else if (e.key === 'ArrowLeft') { e.preventDefault(); player.skip(-15); }
      else if (e.key === 'ArrowRight') { e.preventDefault(); player.skip(30); }
    }
  }

  // Clicking away keeps the words; leaving the app for another doesn't count as clicking away.
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
    if (saved) gsap.fromTo(saved, { opacity: 0, y: 4 }, { opacity: 1, y: 0, duration: 0.35 * motion(), ease: 'power3.out' });
    timer = window.setTimeout(() => { if (plate.id === id && plate.mode === 'saved') close(); }, 1600);
  }

  function close(immediate = false) {
    clearTimeout(timer);
    if (plate.mode !== 'peek' && resume && !now.playing) player.toggle();
    resume = false;
    const id = plate.id;
    const done = () => { if (plate.id === id) { plate.id = null; plate.mode = null; shapeGeo.clear = null; shapeGeo.v++; } };
    gsap.killTweensOf([el, lead]);
    if (immediate || !motion()) { gsap.set(el, { opacity: 0 }); gsap.set(lead, { attr: { 'stroke-dashoffset': 1 } }); done(); return; }
    gsap.to(el, { opacity: 0, duration: 0.25, ease: 'power3.out', onComplete: done });
    gsap.to(lead, { attr: { 'stroke-dashoffset': 1 }, duration: 0.35, delay: 0.05, ease: 'power3.inOut' });
  }

  async function undo() {
    const id = plate.id;
    if (id == null) return;
    close(true);
    await removeNote(id);
  }

  /** The plate's height at a width, fitted to the room it has: a one-line quote, then a note that scrolls inside its field. */
  function fit(w: number, max: number) {
    el.style.width = `${w}px`;
    el.style.setProperty('--qlines', '2');
    el.style.setProperty('--fmax', '66px');
    let h = el.offsetHeight;
    if (h > max && el.querySelector('q.full')) { el.style.setProperty('--qlines', '1'); h = el.offsetHeight; }
    if (h > max && field) { el.style.setProperty('--fmax', `${Math.max(22, field.offsetHeight - (h - max))}px`); h = el.offsetHeight; }
    return h;
  }

  function place() {
    const c = shapeGeo.canvas, stage = el?.parentElement;
    const hit = shapeGeo.hits.find((h) => h.id === plate.id);
    if (!c || !stage || !hit) return;
    const ox = c.offsetLeft, oy = c.offsetTop, x = ox + hit.x, y = oy + hit.y, stageH = stage.clientHeight, orbit = shapeGeo.orbit;
    side = !!orbit;
    const out = placePlate({ x, y, room: stage.clientWidth, stageH, ox, oy, orbit, height: (w) => fit(w, maxHeight({ y, stageH, orbit })) });
    el.style.left = `${out.left}px`;
    el.style.top = `${out.top}px`;
    lead.setAttribute('d', out.leader);
    const k = +getComputedStyle(el).opacity, clear = out.clear ? { ...out.clear, k } : null;
    if (JSON.stringify(clear) !== JSON.stringify(shapeGeo.clear)) { shapeGeo.clear = clear; shapeGeo.v++; }
  }

  function loop() {
    cancelAnimationFrame(raf);
    const f = () => { if (plate.id == null) return; place(); raf = requestAnimationFrame(f); };
    f();
  }

  plateApi.take = take;
  plateApi.peek = (id) => {
    if (id == null) { if (plate.mode === 'peek') close(); return; }
    if (plate.mode && plate.mode !== 'peek') return;
    show(id, 'peek');
  };

  // A note removed elsewhere, or another episode starting, takes the plate with it.
  $effect(() => { if (plate.id != null && marks.for != null && !mark) close(true); });
  let shown = now.episode?.id;
  $effect(() => { const id = now.episode?.id; if (id !== shown) { shown = id; if (plate.id != null) close(true); } });

  // Ahead of the player's keys: while a note is being made, letters (and spaces after the first) belong to it.
  function catchEarly(e: KeyboardEvent) {
    if (!pending || e.metaKey || e.ctrlKey || e.altKey || e.key.length !== 1 || (e.key === ' ' && !early)) return;
    e.preventDefault(); e.stopImmediatePropagation();
    early += e.key;
  }

  onMount(() => {
    hooks.note = () => void take(false);
    window.addEventListener('keydown', catchEarly, true);
    return () => { window.removeEventListener('keydown', catchEarly, true); cancelAnimationFrame(raf); clearTimeout(timer); plate.id = null; plate.mode = null; shapeGeo.clear = null; };
  });
</script>

<svg class="lead" aria-hidden="true"><path bind:this={lead} pathLength="1" stroke-dasharray="1" stroke-dashoffset="1" /></svg>
<div class="plate" class:side class:open={plate.id != null} bind:this={el} data-no-drag role="group" aria-label="Note">
  {#if mark}
    <span class="k">NOTE · {fmt(mark.start)}–{fmt(mark.end)}</span>
    {#if plate.mode === 'peek'}
      {#if mark.note}<span class="one note">{mark.note}</span>{:else if mark.quote}<q class="one">{mark.quote}</q>{/if}
    {:else}
      {#if mark.quote}<q class="full">{mark.quote}</q>{/if}
      {#if plate.mode === 'saved'}
        <span class="field"><span class="note">{mark.note}</span></span>
        <span class="under"><span class="saved" bind:this={saved}>{prefs.notes_folder ? 'Saved to your notes' : 'Saved in Zenpod'}</span></span>
      {:else}
        <span class="field on" class:has={text}>
          <textarea class="note" bind:this={field} bind:value={text} rows="1" placeholder="Add a note" aria-label="Your note" spellcheck="true"
            oninput={typed} onkeydown={keyed} onblur={blurred}></textarea><span class="ret" aria-hidden="true">↵</span>
        </span>
        <span class="under"><button class="undo" onmousedown={(e) => e.preventDefault()} onclick={undo}>Undo</button></span>
      {/if}
    {/if}
  {/if}
</div>

<style>
  .lead { position: absolute; inset: 0; width: 100%; height: 100%; overflow: visible; pointer-events: none; z-index: 2; }
  .lead path { fill: none; stroke: var(--accent); stroke-width: 1; opacity: 0.8; }
  .plate { position: absolute; z-index: 3; display: grid; grid-template-columns: minmax(0, 1fr); gap: 6px; visibility: hidden; opacity: 0; }
  .plate.open { visibility: visible; }
  .k { font-family: var(--font-mono); font-size: 10.5px; letter-spacing: 0.1em; color: var(--text-faint); white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .side .k { white-space: normal; line-height: 1.5; }
  q { font-size: 15px; line-height: 1.45; color: var(--text-mid); quotes: "\201C" "\201D";
    display: -webkit-box; -webkit-line-clamp: var(--qlines, 2); line-clamp: var(--qlines, 2); -webkit-box-orient: vertical; overflow: hidden; }
  .one { display: block; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; font-size: 15px; line-height: 1.45; }
  .side .one { white-space: normal; display: -webkit-box; -webkit-line-clamp: 2; line-clamp: 2; -webkit-box-orient: vertical; }
  .note { font: 400 15px/1.45 var(--font-ui); letter-spacing: 0.01em; color: var(--text); overflow-wrap: anywhere; }
  .field { display: flex; align-items: flex-end; gap: 8px; padding: 2px 0 6px; border-bottom: 1px solid transparent; }
  .field.on { border-bottom-color: var(--accent); }
  textarea.note { flex: 1; min-width: 0; display: block; box-sizing: border-box; margin: 0; padding: 0; border: 0; outline: none; background: none;
    resize: none; overflow-y: auto; scrollbar-width: none; white-space: pre-wrap; max-height: var(--fmax, 66px); caret-color: var(--accent); }
  textarea.note::-webkit-scrollbar { display: none; }
  textarea.note::placeholder { color: var(--text-faint); }
  .ret { font-family: var(--font-mono); font-size: 12px; color: var(--text-faint); opacity: 0; transition: opacity var(--dur-base) var(--ease-hover); }
  .field.has .ret { opacity: 1; }
  .under { min-height: 18px; margin-top: 2px; }
  .saved { display: inline-block; font-size: 12.5px; color: var(--text-dim); }
  .undo { font-size: 12.5px; color: var(--text-faint); transition: color var(--dur-base) var(--ease-hover); }
  .undo:hover { color: var(--failed); }
</style>

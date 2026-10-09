<script lang="ts">
  import { onMount } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { listen } from '@tauri-apps/api/event';
  import { INSET, drawChapters, drawField, drawMargins, drawMarks, drawOrbit, drawTether, drawThread, feed, makeViz, resolveColors, updateSignal } from '$lib/signal.js';
  import { chapters, now, player, position } from '$lib/now.svelte';
  import { grown, loadMarks, marks, plate, plateApi, shapeGeo } from '$lib/marks.svelte';

  // `variant` picks the surface: the main stage, the Mini's Orbit, the Pill's small Orbit, or the panel's tether.
  let { mode = 'orbit', variant = 'main' }: { mode?: 'field' | 'thread' | 'orbit'; variant?: 'main' | 'mini' | 'pill' | 'tether' } = $props();

  // Until the episode has been measured, the shape is quiet and flat: never an invented one.
  const FLAT = new Array(360).fill(0.04);
  let canvas: HTMLCanvasElement;
  let peaks: number[] = FLAT;
  let settle = 60;

  const load = (id: number) =>
    invoke<number[] | null>('peaks', { id }).then((p) => { if (p && now.episode?.id === id) { peaks = p; settle = 60; } });

  $effect(() => {
    const id = now.episode?.id;
    peaks = FLAT;
    if (id) { load(id); loadMarks(id); }
  });
  // Anything that changes the picture draws it again and lets it settle.
  $effect(() => { void [mode, now.playing, now.position, now.duration, chapters.list, marks.list, plate.id, plate.mode]; settle = 60; });

  onMount(() => {
    const V = makeViz(canvas, 'main');
    const reduced = matchMedia('(prefers-reduced-motion: reduce)');
    const dark = matchMedia('(prefers-color-scheme: dark)');
    const recolor = () => requestAnimationFrame(() => { resolveColors(); settle = 40; });
    recolor();
    dark.addEventListener('change', recolor);
    const mo = new MutationObserver(recolor);
    mo.observe(document.documentElement, { attributes: true, attributeFilter: ['data-flavor', 'style'] });
    const bc = new BroadcastChannel('signal');
    bc.onmessage = (e) => feed(e.data);
    const un = listen<number>('peaks', (e) => { if (e.payload === now.episode?.id) load(e.payload); });
    if (variant === 'main') shapeGeo.canvas = canvas;

    let last = performance.now(), raf = 0, seen = -1;
    function frame(t: number) {
      const el = Math.max(8, Math.min(64, t - last));
      last = t;
      const prog = now.duration ? Math.min(1, position() / now.duration) : 0;
      const active = now.playing && !reduced.matches;
      updateSignal(el, active, peaks[Math.min(359, Math.floor(prog * 360))] || 0);
      // A note still growing onto the shape, or the plate moving or fading, keeps a still shape drawing.
      const growing = marks.list.some((m) => t - (grown.get(m.id) ?? -1e9) < 400);
      if (active || settle > 0 || growing || (variant === 'main' && shapeGeo.v !== seen)) {
        seen = shapeGeo.v;
        const r = Math.min(devicePixelRatio || 1, 2), w = Math.max(1, Math.round(canvas.clientWidth)), h = Math.max(1, Math.round(canvas.clientHeight));
        if (canvas.width !== Math.round(w * r) || canvas.height !== Math.round(h * r)) { canvas.width = Math.round(w * r); canvas.height = Math.round(h * r); }
        const c = canvas.getContext('2d')!;
        c.setTransform(r, 0, 0, r, 0, 0); c.clearRect(0, 0, w, h); c.globalAlpha = 1;
        const starts = chapters.list.map((k) => k.start);
        const notes = { grown, reduced: reduced.matches, lit: plate.id ?? undefined };
        if (variant === 'mini') {
          drawOrbit(V, c, w, h, prog, peaks, active, el, { inner: 0.165, maxRay: 0.32, rays: 84 });
          drawMarks(c, w, h, 'orbit', marks.list, now.duration, { ...notes, inner: 0.165 });
        } else if (variant === 'pill') {
          drawOrbit(V, c, w, h, prog, peaks, active, el, { small: true, inner: 0.2, maxRay: 0.28, rays: 30 });
          drawMarks(c, w, h, 'orbit', marks.list, now.duration, { ...notes, inner: 0.2, small: true });
        } else if (variant === 'tether') drawTether(V, c, w, h, h / 2, prog, active);
        else if (mode === 'orbit') {
          drawOrbit(V, c, w, h, prog, peaks, active, el);
          clearBehind(c);
          drawChapters(c, w, h, mode, starts, now.duration, prog);
          shapeGeo.hits = drawMarks(c, w, h, 'orbit', marks.list, now.duration, notes);
          shapeGeo.orbit = { cx: w / 2, cy: h / 2, rout: Math.min(w, h) * 0.49, ring: Math.min(w, h) * 0.165 };
        } else {
          // The episode starts and ends just inside the faded edges, never within them.
          const i = Math.round(w * INSET), iw = w - 2 * i;
          if (mode === 'field') drawField(c, w, h, h / 2, prog, peaks, i);
          else { drawMargins(c, w, h); c.save(); c.translate(i, 0); drawThread(V, c, iw, h, h / 2, prog, peaks, active, el); c.restore(); }
          clearBehind(c);
          c.save(); c.translate(i, 0); drawChapters(c, iw, h, mode, starts, now.duration, prog);
          shapeGeo.hits = drawMarks(c, iw, h, mode, marks.list, now.duration, notes).map((p) => ({ ...p, x: p.x + i }));
          c.restore();
          shapeGeo.orbit = null;
        }
        if (!active) settle--;
      }
      raf = requestAnimationFrame(frame);
    }
    raf = requestAnimationFrame(frame);
    return () => { cancelAnimationFrame(raf); bc.close(); mo.disconnect(); dark.removeEventListener('change', recolor); un.then((f) => f()); };
  });

  // Behind a plate's words the shape steps back, softly; the line, chapters and notes are drawn after, untouched.
  function clearBehind(c: CanvasRenderingContext2D) {
    const cl = shapeGeo.clear;
    if (variant !== 'main' || !cl || cl.k <= 0) return;
    c.save(); c.globalCompositeOperation = 'destination-out'; c.globalAlpha = 0.94 * cl.k; c.filter = 'blur(8px)'; c.fillRect(cl.x, cl.y, cl.w, cl.h); c.restore();
  }

  const at = (e: MouseEvent) => { const r = canvas.getBoundingClientRect(), k = canvas.clientWidth / r.width; return { x: (e.clientX - r.left) * k, y: (e.clientY - r.top) * k }; };
  const noteAt = (e: MouseEvent) => { const p = at(e); return variant === 'main' ? shapeGeo.hits.find((h) => Math.hypot(h.x - p.x, h.y - p.y) < 11) : undefined; };

  // Pointing at a note shows it; clicking one plays from its start.
  function hover(e: PointerEvent) {
    if (variant !== 'main' || (plate.mode && plate.mode !== 'peek')) return;
    const hit = noteAt(e);
    if ((hit?.id ?? null) !== (plate.mode === 'peek' ? plate.id : null)) plateApi.peek(hit?.id ?? null);
  }

  function seekAt(e: MouseEvent) {
    if (!now.duration) return;
    const hit = noteAt(e);
    if (hit) {
      const m = marks.list.find((x) => x.id === hit.id);
      if (m) { plateApi.peek(null); player.seek(m.start); if (!now.playing) player.toggle(); }
      return;
    }
    const r = canvas.getBoundingClientRect();
    let f = (e.clientX - r.left - r.width * INSET) / (r.width * (1 - 2 * INSET));
    if (variant === 'pill') return;
    if (variant === 'tether') f = (e.clientX - r.left) / r.width;
    else if (mode === 'orbit' || variant === 'mini') {
      // The Mini's orb fills most of the Mini, so only the ring itself seeks; the rest is for carrying it.
      const d = Math.min(r.width, r.height), dist = Math.hypot(e.clientX - (r.left + r.width / 2), e.clientY - (r.top + r.height / 2));
      if (variant === 'mini' && (dist < d * 0.14 || dist > d * 0.4)) return;
      let a = Math.atan2(e.clientY - (r.top + r.height / 2), e.clientX - (r.left + r.width / 2)) + Math.PI / 2;
      if (a < 0) a += Math.PI * 2;
      f = a / (Math.PI * 2);
    }
    player.seek(Math.max(0, Math.min(1, f)) * now.duration);
  }
</script>

<canvas bind:this={canvas} class:orbit={mode === 'orbit'} class:fill={variant !== 'main'} onclick={seekAt} onpointermove={hover}
  onpointerleave={() => { if (plate.mode === 'peek') plateApi.peek(null); }}
  aria-label={mode === 'orbit' ? 'Orbit. Click to move through the episode.' : 'Waveform. Click to move through the episode.'}></canvas>

<style>
  canvas { display: block; width: 100%; height: 280px; cursor: pointer;
    -webkit-mask-image: linear-gradient(90deg, transparent 0, #000 10%, #000 90%, transparent 100%);
    mask-image: linear-gradient(90deg, transparent 0, #000 10%, #000 90%, transparent 100%); }
  canvas.orbit { height: 340px; -webkit-mask-image: none; mask-image: none; }
  canvas.fill { width: 100%; height: 100%; -webkit-mask-image: none; mask-image: none; }
</style>

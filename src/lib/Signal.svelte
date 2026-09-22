<script lang="ts">
  import { onMount } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { listen } from '@tauri-apps/api/event';
  import { INSET, drawChapters, drawField, drawMargins, drawOrbit, drawTether, drawThread, feed, makeViz, resolveColors, updateSignal } from '$lib/signal.js';
  import { chapters, now, player, position } from '$lib/now.svelte';

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
    if (id) load(id);
  });
  // Anything that changes the picture draws it again and lets it settle.
  $effect(() => { void [mode, now.playing, now.position, now.duration, chapters.list]; settle = 60; });

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

    let last = performance.now(), raf = 0;
    function frame(t: number) {
      const el = Math.max(8, Math.min(64, t - last));
      last = t;
      const prog = now.duration ? Math.min(1, position() / now.duration) : 0;
      const active = now.playing && !reduced.matches;
      updateSignal(el, active, peaks[Math.min(359, Math.floor(prog * 360))] || 0);
      if (active || settle > 0) {
        const r = Math.min(devicePixelRatio || 1, 2), w = Math.max(1, Math.round(canvas.clientWidth)), h = Math.max(1, Math.round(canvas.clientHeight));
        if (canvas.width !== Math.round(w * r) || canvas.height !== Math.round(h * r)) { canvas.width = Math.round(w * r); canvas.height = Math.round(h * r); }
        const c = canvas.getContext('2d')!;
        c.setTransform(r, 0, 0, r, 0, 0); c.clearRect(0, 0, w, h); c.globalAlpha = 1;
        const marks = chapters.list.map((k) => k.start);
        if (variant === 'mini') drawOrbit(V, c, w, h, prog, peaks, active, el, { inner: 0.165, maxRay: 0.32, rays: 84 });
        else if (variant === 'pill') drawOrbit(V, c, w, h, prog, peaks, active, el, { small: true, inner: 0.2, maxRay: 0.28, rays: 30 });
        else if (variant === 'tether') drawTether(V, c, w, h, h / 2, prog, active);
        else if (mode === 'orbit') {
          drawOrbit(V, c, w, h, prog, peaks, active, el);
          drawChapters(c, w, h, mode, marks, now.duration, prog);
        } else {
          // The episode starts and ends just inside the faded edges, never within them.
          const i = Math.round(w * INSET), iw = w - 2 * i;
          if (mode === 'field') drawField(c, w, h, h / 2, prog, peaks, i);
          else { drawMargins(c, w, h); c.save(); c.translate(i, 0); drawThread(V, c, iw, h, h / 2, prog, peaks, active, el); c.restore(); }
          c.save(); c.translate(i, 0); drawChapters(c, iw, h, mode, marks, now.duration, prog); c.restore();
        }
        if (!active) settle--;
      }
      raf = requestAnimationFrame(frame);
    }
    raf = requestAnimationFrame(frame);
    return () => { cancelAnimationFrame(raf); bc.close(); mo.disconnect(); dark.removeEventListener('change', recolor); un.then((f) => f()); };
  });

  function seekAt(e: MouseEvent) {
    if (!now.duration) return;
    const r = canvas.getBoundingClientRect();
    let f = (e.clientX - r.left - r.width * INSET) / (r.width * (1 - 2 * INSET));
    if (variant === 'tether') f = (e.clientX - r.left) / r.width;
    if (variant === 'pill') return;
    if (mode === 'orbit' || variant === 'mini') {
      let a = Math.atan2(e.clientY - (r.top + r.height / 2), e.clientX - (r.left + r.width / 2)) + Math.PI / 2;
      if (a < 0) a += Math.PI * 2;
      f = a / (Math.PI * 2);
    }
    player.seek(Math.max(0, Math.min(1, f)) * now.duration);
  }
</script>

<canvas bind:this={canvas} class:orbit={mode === 'orbit'} class:fill={variant !== 'main'} onclick={seekAt}
  aria-label={mode === 'orbit' ? 'Orbit. Click to move through the episode.' : 'Waveform. Click to move through the episode.'}></canvas>

<style>
  canvas { display: block; width: 100%; height: 280px; cursor: pointer;
    -webkit-mask-image: linear-gradient(90deg, transparent 0, #000 10%, #000 90%, transparent 100%);
    mask-image: linear-gradient(90deg, transparent 0, #000 10%, #000 90%, transparent 100%); }
  canvas.orbit { height: 340px; -webkit-mask-image: none; mask-image: none; }
  canvas.fill { width: 100%; height: 100%; -webkit-mask-image: none; mask-image: none; }
</style>

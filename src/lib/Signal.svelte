<script lang="ts">
  import { onMount } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { listen } from '@tauri-apps/api/event';
  import { drawField, drawOrbit, drawThread, feed, makeViz, resolveColors, updateSignal } from '$lib/signal.js';
  import { now, player, position } from '$lib/now.svelte';

  let { mode }: { mode: 'field' | 'thread' | 'orbit' } = $props();

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
  $effect(() => { void [mode, now.playing, now.position, now.duration]; settle = 60; });

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
    let heard = false;
    bc.onmessage = (e) => { feed(e.data); if (!heard) { heard = true; invoke('log', { msg: 'signal reaches the main window' }); } };
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
        if (mode === 'field') drawField(c, w, h, h / 2, prog, peaks);
        else if (mode === 'thread') drawThread(V, c, w, h, h / 2, prog, peaks, active, el);
        else drawOrbit(V, c, w, h, prog, peaks, active, el);
        if (!active) settle--;
      }
      raf = requestAnimationFrame(frame);
    }
    raf = requestAnimationFrame(frame);
    return () => { cancelAnimationFrame(raf); bc.close(); mo.disconnect(); dark.removeEventListener('change', recolor); un.then((f) => f()); };
  });

  function seekAt(e: PointerEvent) {
    if (!now.duration) return;
    const r = canvas.getBoundingClientRect();
    let f = (e.clientX - r.left) / r.width;
    if (mode === 'orbit') {
      let a = Math.atan2(e.clientY - (r.top + r.height / 2), e.clientX - (r.left + r.width / 2)) + Math.PI / 2;
      if (a < 0) a += Math.PI * 2;
      f = a / (Math.PI * 2);
    }
    player.seek(Math.max(0, Math.min(1, f)) * now.duration);
  }
</script>

<canvas bind:this={canvas} class:orbit={mode === 'orbit'} onpointerdown={seekAt}
  aria-label={mode === 'orbit' ? 'Orbit. Click to move through the episode.' : 'Waveform. Click to move through the episode.'}></canvas>

<style>
  canvas { display: block; width: 100%; height: 280px; cursor: pointer;
    -webkit-mask-image: linear-gradient(90deg, transparent 0, #000 16%, #000 84%, transparent 100%);
    mask-image: linear-gradient(90deg, transparent 0, #000 16%, #000 84%, transparent 100%); }
  canvas.orbit { height: 340px; -webkit-mask-image: none; mask-image: none; }
</style>

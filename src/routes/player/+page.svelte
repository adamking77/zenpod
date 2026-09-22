<script lang="ts">
  // The hidden window that plays. It does what the core says and reports what the audio actually does.
  import { onMount } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { listen } from '@tauri-apps/api/event';

  type Cmd =
    | { kind: 'load'; src: string; position: number; speed: number; play: boolean; title: string; show: string; art: string }
    | { kind: 'play' } | { kind: 'pause' } | { kind: 'seek'; position: number } | { kind: 'speed'; speed: number };

  let audio: HTMLAudioElement;
  let id = 0;
  let last = 0;
  // Nothing is reported until a freshly loaded episode sits at its saved position.
  let placed = false;

  // The analyser sits here beside the audio; the visible windows draw what it hears (pass 7's reading).
  const signal = new BroadcastChannel('signal');
  let ctx: AudioContext | null = null, an: AnalyserNode | null = null;
  let td = new Uint8Array(0), fd = new Uint8Array(0);
  const bands = new Float32Array(48);

  async function graph() {
    if (an || ctx) { ctx?.resume(); return; }
    try {
      ctx = new AudioContext();
      await ctx.resume();
      // Only route the audio through the graph once it's running, or the episode would go silent.
      if (ctx.state !== 'running') { ctx.close(); ctx = null; return; }
      const src = ctx.createMediaElementSource(audio);
      an = ctx.createAnalyser(); an.fftSize = 1024; an.smoothingTimeConstant = 0.72;
      src.connect(an); an.connect(ctx.destination);
      td = new Uint8Array(an.fftSize); fd = new Uint8Array(an.frequencyBinCount);
    } catch (e) { invoke('log', { msg: `analyser: ${e}` }); }
  }

  function listenIn() {
    if (!an || !ctx || audio.paused) return;
    an.getByteTimeDomainData(td); an.getByteFrequencyData(fd);
    let sq = 0; for (let i = 0; i < td.length; i++) { const v = (td[i] - 128) / 128; sq += v * v; }
    const te = Math.max(0, Math.min(1, (Math.sqrt(sq / td.length) - 0.012) * 6.8));
    const ny = ctx.sampleRate / 2, fl = fd.length;
    const lo = Math.max(1, Math.floor((180 / ny) * fl)), up = Math.min(fl - 1, Math.ceil((3200 / ny) * fl));
    let ps = 0; for (let i = lo; i <= up; i++) ps += fd[i] / 255;
    const tp = ps / Math.max(1, up - lo + 1);
    for (let i = 0; i < 48; i++) {
      const q = i / 47, f = 120 * Math.pow(3800 / 120, q), bin = Math.max(1, Math.min(fl - 2, Math.round((f / ny) * fl)));
      const mag = (fd[bin - 1] + fd[bin] * 2 + fd[bin + 1]) / (4 * 255);
      bands[i] = Math.min(1, Math.pow(Math.max(0, Math.min(1, (mag - 0.025) * 2.2)), 0.62) * (0.84 + q * 0.28));
    }
    signal.postMessage({ te, tp, bands });
  }

  const report = (ended = false) =>
    id && placed && invoke('report', { id, position: audio.currentTime, duration: audio.duration || 0, playing: !audio.paused, ended })
      .catch((e) => invoke('log', { msg: `report failed: ${e}` }));

  function run(c: Cmd) {
    if (c.kind === 'load') {
      id = Number(c.src.split('/').pop());
      placed = false;
      audio.src = c.src;
      audio.playbackRate = c.speed;
      audio.addEventListener('loadedmetadata', () => {
        audio.currentTime = c.position;
        audio.playbackRate = c.speed;
        placed = true;
      }, { once: true });
      if (c.play) audio.play().catch((e) => { invoke('log', { msg: `play() ${e}` }); report(); });
      navigator.mediaSession.metadata = new MediaMetadata({ title: c.title, artist: c.show, artwork: [{ src: c.art, sizes: '600x600' }] });
    }
    if (c.kind === 'play') audio.play().catch(() => report());
    if (c.kind === 'pause') audio.pause();
    if (c.kind === 'seek') { audio.currentTime = c.position; placed = true; }
    if (c.kind === 'speed') audio.playbackRate = c.speed;
  }

  onMount(() => {
    audio.addEventListener('timeupdate', () => {
      const t = performance.now();
      if (t - last > 250) { last = t; report(); }
    });
    audio.addEventListener('play', graph);
    const tick = setInterval(listenIn, 33);
    for (const ev of ['play', 'pause', 'seeked', 'loadedmetadata', 'error']) audio.addEventListener(ev, () => report());
    audio.addEventListener('error', () => invoke('log', { msg: `audio error ${audio.error?.code} ${audio.error?.message} ${audio.src}` }));
    audio.addEventListener('ended', () => report(true));

    // Media keys and Now Playing, whichever window is open or none.
    const ms = navigator.mediaSession;
    ms.setActionHandler('play', () => invoke('toggle'));
    ms.setActionHandler('pause', () => invoke('toggle'));
    ms.setActionHandler('seekbackward', () => invoke('skip', { by: -15 }));
    ms.setActionHandler('seekforward', () => invoke('skip', { by: 30 }));
    ms.setActionHandler('previoustrack', () => invoke('skip', { by: -15 }));
    ms.setActionHandler('nexttrack', () => invoke('skip', { by: 30 }));
    ms.setActionHandler('seekto', (d) => d.seekTime != null && invoke('seek', { position: d.seekTime }));

    invoke('log', { msg: 'player mounted' });
    const un = listen<Cmd>('cmd', (e) => run(e.payload));
    un.then(() => invoke('player_ready'));
    return () => { clearInterval(tick); un.then((f) => f()); };
  });
</script>

<audio bind:this={audio} crossorigin="anonymous" preload="auto"></audio>

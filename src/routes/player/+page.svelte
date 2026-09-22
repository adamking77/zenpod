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

  const report = (ended = false) =>
    id && invoke('report', { id, position: audio.currentTime, duration: audio.duration || 0, playing: !audio.paused, ended })
      .catch((e) => invoke('log', { msg: `report failed: ${e}` }));

  function run(c: Cmd) {
    invoke('log', { msg: `cmd ${c.kind}` });
    if (c.kind === 'load') {
      id = Number(c.src.split('/').pop());
      audio.src = c.src;
      audio.playbackRate = c.speed;
      audio.addEventListener('loadedmetadata', () => {
        audio.currentTime = c.position;
        audio.playbackRate = c.speed;
      }, { once: true });
      if (c.play) audio.play().catch((e) => { invoke('log', { msg: `play() ${e}` }); report(); });
      navigator.mediaSession.metadata = new MediaMetadata({ title: c.title, artist: c.show, artwork: [{ src: c.art, sizes: '600x600' }] });
    }
    if (c.kind === 'play') audio.play().catch(() => report());
    if (c.kind === 'pause') audio.pause();
    if (c.kind === 'seek') audio.currentTime = c.position;
    if (c.kind === 'speed') audio.playbackRate = c.speed;
  }

  onMount(() => {
    audio.addEventListener('timeupdate', () => {
      const t = performance.now();
      if (t - last > 250) { last = t; report(); }
    });
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
    return () => un.then((f) => f());
  });
</script>

<audio bind:this={audio} crossorigin="anonymous" preload="auto"></audio>

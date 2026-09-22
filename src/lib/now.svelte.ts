import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import type { Episode } from '$lib/api';

export type Now = { episode: Episode | null; playing: boolean; position: number; duration: number; speed: number };

// What the core says is playing, plus when we heard it, so the time can run smoothly between reports.
export const now = $state<Now & { at: number }>({ episode: null, playing: false, position: 0, duration: 0, speed: 1, at: 0 });

const take = (n: Now) => Object.assign(now, n, { at: performance.now() });

export async function follow() {
  take(await invoke<Now>('playback'));
  return listen<Now>('state', (e) => take(e.payload));
}

export const position = () =>
  now.playing ? Math.min(now.duration || Infinity, now.position + ((performance.now() - now.at) / 1000) * now.speed) : now.position;

export const player = {
  choose: (id: number) => invoke('choose', { id }),
  toggle: () => invoke('toggle'),
  seek: (position: number) => invoke('seek', { position }),
  skip: (by: number) => invoke('skip', { by }),
  speed: (speed: number) => invoke('set_speed', { speed }),
};

export type Chapter = { title: string; start: number };

// The chapters of whatever is loaded, from the feed. Empty when the feed has none.
export const chapters = $state<{ list: Chapter[]; for: number | null }>({ list: [], for: null });

export async function loadChapters(id: number) {
  chapters.for = id;
  chapters.list = [];
  const list = await invoke<Chapter[]>('chapters', { id }).catch(() => [] as Chapter[]);
  if (chapters.for === id) chapters.list = list;
}

export const chapterAt = (t: number) => chapters.list.findLast((c) => c.start <= t + 0.5) ?? null;

/** Space plays and pauses, arrows skip; the same on every surface. */
export function keys(e: KeyboardEvent) {
  if (e.target instanceof HTMLInputElement || e.metaKey || e.ctrlKey) return;
  if (e.code === 'Space' && !(e.target instanceof HTMLButtonElement)) { e.preventDefault(); player.toggle(); }
  if (e.key === 'ArrowLeft') player.skip(-15);
  if (e.key === 'ArrowRight') player.skip(30);
}

/** Leave this surface for another, with the closing surface folding away first. */
export function goTo(to: 'win' | 'mini' | 'pill') {
  const root = document.body;
  const reduce = matchMedia('(prefers-reduced-motion: reduce)').matches;
  root.classList.add('leaving');
  setTimeout(() => { invoke('set_mode', { to }); root.classList.remove('leaving'); }, reduce ? 0 : 200);
}

/** Replay the arrival when this window becomes the one showing. */
export function arrivals(me: string) {
  return listen<string>('mode', (e) => {
    if (e.payload !== me) return;
    document.body.classList.remove('arriving'); void document.body.offsetWidth; document.body.classList.add('arriving');
  });
}

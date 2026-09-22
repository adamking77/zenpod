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
  const list = await invoke<Chapter[]>('chapters', { id });
  if (chapters.for === id) chapters.list = list;
}

export const chapterAt = (t: number) => chapters.list.findLast((c) => c.start <= t + 0.5) ?? null;

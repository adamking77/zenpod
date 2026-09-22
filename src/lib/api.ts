import { invoke } from '@tauri-apps/api/core';

export type Show = {
  id: number; title: string; author: string | null; about: string | null;
  image_url: string | null; spotify_only: boolean; fresh: number; latest: number | null;
};

export type Episode = {
  id: number; show_id: number; show_title: string; title: string;
  published: number | null; duration: number | null; position: number;
  played: boolean; kept: boolean; image_url: string | null;
};

export type Imported = { added: number; had: number; spotify_only: number; failed: number };

export const api = {
  addShow: (input: string) => invoke<string>('add_show', { input }),
  refresh: () => invoke<number>('refresh'),
  importOpml: (text: string) => invoke<Imported>('import_opml', { text }),
  importSpotify: (text: string) => invoke<Imported>('import_spotify', { text }),
  shows: () => invoke<Show[]>('shows'),
  newest: () => invoke<Episode[]>('newest'),
  showEpisodes: (showId: number) => invoke<Episode[]>('show_episodes', { showId }),
  unfollow: (showId: number) => invoke<void>('unfollow', { showId }),
  correctFeed: (showId: number, url: string) => invoke<string>('correct_feed', { showId, url }),
  settings: () => invoke<Record<string, string>>('settings'),
  setSetting: (key: string, value: string) => invoke<void>('set_setting', { key, value }),
};

export const fmt = (v: number) => {
  v = Math.max(0, Math.floor(v));
  const h = Math.floor(v / 3600), m = Math.floor((v % 3600) / 60), s = v % 60;
  return h ? `${h}:${String(m).padStart(2, '0')}:${String(s).padStart(2, '0')}` : `${m}:${String(s).padStart(2, '0')}`;
};

export const minutesLeft = (e: Pick<Episode, 'duration' | 'position'>) =>
  `${Math.max(1, Math.round(((e.duration ?? 0) - e.position) / 60))} min left`;

export const length = (e: Episode) =>
  e.position > 0 && !e.played && e.duration ? minutesLeft(e) : e.duration ? `${Math.round(e.duration / 60)} min` : '';

export const plain = (html: string | null) =>
  html ? (new DOMParser().parseFromString(html, 'text/html').body.textContent ?? '').replace(/\s+/g, ' ').trim() : '';

/** "The AI Daily Brief: Artificial Intelligence News…" → "The AI Daily Brief" */
export const short = (title: string) => title.replace(/\s*[:|(].*$/, '') || title;

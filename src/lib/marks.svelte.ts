import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { chapterAt, hooks, now, position } from '$lib/now.svelte';
import { parseTranscript, type Cue } from '$lib/transcript';

// Notes: the last 30 seconds, kept with a key, and the listener's words beside them. "Mark" is the code's name for
// one, because the show's own notes already own "notes" (`episode_notes`); nothing on screen says mark.
export type Mark = {
  id: number; episode_id: number; start: number; end: number; note: string; quote: string | null; chapter: string | null;
  created: number; show_id: number; show_title: string; episode_title: string; image_url: string | null;
};
export const SPAN = 30;

/** The loaded episode's notes, by start. Every window keeps its own copy current. */
export const marks = $state<{ list: Mark[]; for: number | null }>({ list: [], for: null });
/** Bumped whenever any note anywhere changes, for lists of every note. */
export const changes = $state({ n: 0 });
/** When each note appeared on this surface, so it can grow onto the shape. */
export const grown = new Map<number, number>();

let cues: { for: number | null; list: Cue[] } = { for: null, list: [] };

export async function loadMarks(id: number) {
  marks.for = id;
  const list = await invoke<Mark[]>('marks', { episodeId: id }).catch(() => [] as Mark[]);
  if (marks.for === id) marks.list = list;
  // The transcript, when the feed has one, so a note can keep the words it caught.
  if (cues.for !== id) {
    cues = { for: id, list: [] };
    invoke<string | null>('transcript', { id })
      .then((t) => { if (t && cues.for === id) cues.list = parseTranscript(t).filter((c) => c.start >= 0); })
      .catch(() => {});
  }
}

export function followMarks() {
  return listen<number>('marks', (e) => {
    changes.n++;
    if (e.payload === marks.for) loadMarks(e.payload);
  });
}

/** The last few transcript lines spoken inside [start, end]. */
function quoteFor(start: number, end: number) {
  const L = cues.list, out: string[] = [];
  L.forEach((c, i) => { if ((L[i + 1]?.start ?? Infinity) > start + 1 && c.start < end - 1) out.push(c.text); });
  return out.slice(-3).join(' ').replace(/\s+/g, ' ').trim() || null;
}

/** Keep the last 30 seconds of what's playing. */
export async function takeNote(): Promise<Mark | null> {
  const ep = now.episode;
  if (!ep) return null;
  const end = position(), start = Math.max(0, end - SPAN);
  const m = await invoke<Mark>('add_mark', { episodeId: ep.id, start, end, quote: quoteFor(start, end), chapter: chapterAt(end)?.title ?? null });
  grown.set(m.id, performance.now());
  if (marks.for === ep.id && !marks.list.some((x) => x.id === m.id)) marks.list = [...marks.list, m].sort((a, b) => a.start - b.start);
  return m;
}

export function setNote(id: number, note: string) {
  const m = marks.list.find((x) => x.id === id);
  if (m) m.note = note.trim();
  return invoke('set_mark_note', { id, note });
}

export function removeNote(id: number) {
  marks.list = marks.list.filter((x) => x.id !== id);
  return invoke('remove_mark', { id });
}

export const allMarks = () => invoke<Mark[]>('all_marks');
export const notesFile = (episodeId: number) => invoke<string | null>('notes_file', { episodeId });

/** The plate on the main window's shape: which note it shows, and in what state. */
export const plate = $state<{ id: number | null; mode: 'new' | 'typing' | 'saved' | 'peek' | null }>({ id: null, mode: null });

/** Where the shape put things, written by the main window's shape each frame it draws, read by the plate. */
export const shapeGeo = {
  canvas: null as HTMLCanvasElement | null,
  hits: [] as { id: number; x: number; y: number }[],
  orbit: null as { cx: number; cy: number; rout: number; ring: number } | null,
  /** The patch behind the plate's words, in canvas pixels, and how present it is (the plate's opacity). */
  clear: null as { x: number; y: number; w: number; h: number; k: number } | null,
  /** Bumped when the plate moves or fades, so a still shape draws again. */
  v: 0,
};

/** The plate's own controls, registered by the plate; the shape and the keys call through here. */
export const plateApi = {
  take: (_byButton: boolean) => {},
  peek: (_id: number | null) => {},
};

// Surfaces without a plate (the Mini and the Pill) keep a note with no words.
hooks.note = () => void takeNote();

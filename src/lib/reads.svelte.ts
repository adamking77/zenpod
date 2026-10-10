import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { api, type Show } from '$lib/api';
import { fromHtml, fromText, host, isLink } from '$lib/extract';
import { prefs } from '$lib/prefs.svelte';
import { ui } from '$lib/ui.svelte';

export type Read = {
  id: number; show_id: number; show_title: string; episode_id: number | null; title: string; source: string;
  chars: number; state: 'waiting' | 'queued' | 'reading' | 'failed' | 'done'; pieces: number; done: number; error: string | null; created: number;
};
export type ServiceInfo = { id: string; name: string; model: string; voice: string; keyed: boolean };
export type MacVoice = { name: string; locale: string };

/** What's waiting to be read: the check card in the pane. */
export type Card =
  | { kind: 'busy'; what: string }
  | { kind: 'error'; message: string }
  | { kind: 'item'; title: string; from: string; source: string; text: string; to: number | 'new'; newName: string }
  | { kind: 'podcast'; name: string; url: string }
  | { kind: 'feed'; name: string; url: string };

export const rtm = $state({
  reads: [] as Read[],
  /** Your shows, Read to me first. */
  shows: [] as Show[],
  card: null as Card | null,
  services: [] as ServiceInfo[],
  mac: [] as MacVoice[],
  /** Bumped to put the cursor in the pane's field. */
  focus: 0,
});

export const mins = (chars: number) => Math.max(1, Math.round(chars / 900));

export async function loadReads() {
  let [reads, shows] = await Promise.all([invoke<Read[]>('reads'), api.shows()]);
  // Read to me is always there to send things into.
  if (!shows.some((s) => s.kind === 'read-to-me')) { await invoke('read_to_me'); shows = await api.shows(); }
  rtm.reads = reads;
  rtm.shows = shows.filter((s) => s.kind != null).sort((a, b) => Number(b.kind === 'read-to-me') - Number(a.kind === 'read-to-me') || a.title.localeCompare(b.title));
}

export async function loadVoices() {
  [rtm.services, rtm.mac] = await Promise.all([invoke<ServiceInfo[]>('voice_services'), rtm.mac.length ? rtm.mac : invoke<MacVoice[]>('mac_voices')]);
}

export function followReads() {
  loadReads();
  const a = listen('reads', loadReads), b = listen('library', loadReads);
  // Files opened with Zenpod and zenpod://read links, including any that arrived before this window loaded.
  const c = listen('incoming', takeIncoming);
  takeIncoming();
  return Promise.all([a, b, c]).then((fs) => () => fs.forEach((f) => f()));
}

/** Who reads: a show's own voice, or the default from Settings. */
export function voiceLabel(show?: Show | null) {
  if (show?.voice?.startsWith('mac:')) return show.voice.slice(4);
  if (prefs.voice_from === 'service') {
    const id = prefs.voice_service || 'openai', s = rtm.services.find((x) => x.id === id);
    return `${prefs[`voice_${id}_voice`] || s?.voice || ''} on ${s?.name ?? 'the service'}`;
  }
  return prefs.voice_mac || 'the Mac’s voice';
}

/** Open Read to me with the field ready. */
export function openRead() {
  ui.notes = false;
  ui.yours = null;
  ui.tab = 'read';
  rtm.focus++;
}

async function homeShow() {
  return rtm.shows.find((s) => s.kind === 'read-to-me')?.id ?? (await invoke<number>('read_to_me'));
}

/** A link or text, from the field, ⌘V, a drop or a zenpod://read link. */
export async function take(input: string, into?: string | null) {
  const v = input.trim();
  if (!v) return;
  openRead();
  const to = (into && rtm.shows.find((s) => s.title.toLowerCase() === into.toLowerCase())?.id) || (await homeShow());
  if (!isLink(v)) {
    const d = fromText(v);
    rtm.card = { kind: 'item', title: d.title, from: 'Pasted text', source: 'Pasted text', text: d.text, to, newName: '' };
    return;
  }
  rtm.card = { kind: 'busy', what: `Opening ${host(v)}…` };
  try {
    const page = await invoke<{ url: string; kind: 'page' | 'podcast' | 'feed'; html: string; title: string | null }>('fetch_page', { url: v });
    if (rtm.card?.kind !== 'busy') return; // cancelled meanwhile
    if (page.kind === 'podcast') { rtm.card = { kind: 'podcast', name: page.title ?? host(page.url), url: v }; return; }
    if (page.kind === 'feed') { rtm.card = { kind: 'feed', name: page.title ?? host(page.url), url: v }; return; }
    const d = fromHtml(page.html, page.url);
    rtm.card = d
      ? { kind: 'item', title: d.title, from: `${host(page.url)} · article`, source: page.url, text: d.text, to, newName: '' }
      : { kind: 'error', message: 'There’s no article text on that page.' };
  } catch (e) {
    if (rtm.card?.kind === 'busy') rtm.card = { kind: 'error', message: String(e) };
  }
}

/** A file from Choose file, a drop, the Dock or Open With. */
export async function takeFile(path: string) {
  openRead();
  const name = path.split('/').pop() ?? path;
  rtm.card = { kind: 'busy', what: `Opening ${name}…` };
  try {
    const [d, to] = await Promise.all([invoke<{ title: string; text: string }>('read_file', { path }), homeShow()]);
    if (rtm.card?.kind !== 'busy') return;
    rtm.card = { kind: 'item', title: d.title, from: name, source: name, text: d.text, to, newName: '' };
  } catch (e) {
    if (rtm.card?.kind === 'busy') rtm.card = { kind: 'error', message: String(e) };
  }
}

async function takeIncoming() {
  const items = await invoke<{ kind: 'file' | 'link'; value: string; show: string | null }[]>('take_incoming');
  const last = items.at(-1);
  if (last?.kind === 'file') takeFile(last.value);
  else if (last) take(last.value, last.show);
}

/** Read it: into the show chosen, or a new one named on the card. */
export async function readCard() {
  const c = rtm.card;
  if (c?.kind !== 'item') return;
  const newShow = c.to === 'new' ? c.newName.trim() || 'New show' : null;
  await invoke('add_read', { showId: c.to === 'new' ? null : c.to, newShow, title: c.title, source: c.source, text: c.text });
  rtm.card = null;
  rtm.focus++;
}

export const reads = {
  now: (id: number) => invoke('read_now', { id }),
  stop: (id: number) => invoke('stop_read', { id }),
  discard: (id: number) => invoke('discard_read', { id }),
  setShow: (id: number, patch: { title?: string; auto?: boolean; voice?: string }) => invoke('set_show', { id, ...patch }),
  removeShow: (id: number) => invoke('remove_show', { id }),
  deleteEpisode: (id: number) => invoke('delete_episode', { id }),
};

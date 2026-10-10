import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { api, type Episode, type Show } from '$lib/api';
import { feedLink, fromHtml, fromText, host, isLink } from '$lib/extract';
import { prefs } from '$lib/prefs.svelte';
import { ui } from '$lib/ui.svelte';
import { player } from '$lib/now.svelte';

export type Read = {
  id: number; show_id: number; show_title: string; episode_id: number | null; title: string; source: string;
  chars: number; state: 'waiting' | 'queued' | 'reading' | 'failed' | 'done'; pieces: number; done: number; error: string | null; created: number;
  link: string | null; needs_text: boolean; preview: boolean;
};
export type ServiceInfo = { id: string; name: string; model: string; voice: string; keyed: boolean };
export type MacVoice = { name: string; locale: string };

/** What's waiting to be read: the check card in the pane. */
export type Card =
  | { kind: 'busy'; what: string }
  | { kind: 'error'; message: string }
  | { kind: 'item'; title: string; from: string; source: string; text: string; to: number | 'new'; newName: string }
  | { kind: 'podcast'; name: string; url: string }
  | { kind: 'feed' | 'folder'; name: string; source: string; items: string[]; count: number; auto: boolean };

export const rtm = $state({
  reads: [] as Read[],
  /** Your shows, Read to me first. */
  shows: [] as Show[],
  /** Your latest episodes, heard or not, newest first: playable, movable and deletable right in the pane. */
  ready: [] as Episode[],
  card: null as Card | null,
  services: [] as ServiceInfo[],
  mac: [] as MacVoice[],
  /** The one you just asked for: followed in place from making to playing. */
  current: null as { read: number; title: string; episode: number | null; error: string | null } | null,
  /** Bumped to put the cursor in the pane's field. */
  focus: 0,
});

export const mins = (chars: number) => Math.max(1, Math.round(chars / 900));

export async function loadReads() {
  let [reads, shows, newest] = await Promise.all([invoke<Read[]>('reads'), api.shows(), api.newest()]);
  rtm.ready = newest.filter((e) => e.show_kind).slice(0, 6);
  await follow(reads);
  // Read to me is always there to send things into.
  if (!shows.some((s) => s.kind === 'read-to-me')) { await invoke('read_to_me'); shows = await api.shows(); }
  rtm.reads = reads;
  rtm.shows = shows.filter((s) => s.kind != null).sort((a, b) => Number(b.kind === 'read-to-me') - Number(a.kind === 'read-to-me') || a.title.localeCompare(b.title));
}

export async function loadVoices() {
  [rtm.services, rtm.mac] = await Promise.all([invoke<ServiceInfo[]>('voice_services'), rtm.mac.length ? rtm.mac : invoke<MacVoice[]>('mac_voices')]);
}

export function followReads() {
  const a = listen('reads', () => loadReads().then(fetchWhole)), b = listen('library', loadReads), d = listen('episode', loadReads);
  loadReads().then(fetchWhole);
  // Files opened with Zenpod and zenpod://read links, including any that arrived before this window loaded.
  const c = listen('incoming', takeIncoming);
  takeIncoming();
  return Promise.all([a, b, c, d]).then((fs) => () => fs.forEach((f) => f()));
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

/** The reading you just asked for: when it's an episode, it plays. Nothing else starts by itself. */
async function follow(reads: Read[]) {
  const c = rtm.current;
  if (!c || c.episode) return;
  const pending = reads.find((r) => r.id === c.read);
  if (pending) { c.error = pending.state === 'failed' ? pending.error : null; return; }
  const r = await invoke<Read | null>('read_status', { id: c.read });
  if (r?.state === 'done' && r.episode_id && rtm.current === c) {
    c.episode = r.episode_id;
    player.choose(r.episode_id, [r.episode_id]);
  } else if (!r) rtm.current = null; // removed meanwhile
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
    if (page.kind === 'feed') { await takeSource('feed', page.url); return; }
    // A blog's front page, or a page with no article but a feed: the feed becomes a show.
    const feed = feedLink(page.html, page.url), front = new URL(page.url).pathname.replace(/\/$/, '') === '';
    const d = front && feed ? null : fromHtml(page.html, page.url);
    if (!d && feed) { await takeSource('feed', feed); return; }
    rtm.card = d
      ? { kind: 'item', title: d.title, from: `${host(page.url)} · article`, source: page.url, text: d.text, to, newName: '' }
      : { kind: 'error', message: 'There’s no article text on that page.' };
  } catch (e) {
    if (rtm.card?.kind === 'busy') rtm.card = { kind: 'error', message: String(e) };
  }
}

/** A feed or a folder: what it holds now, for the card that makes it a show. */
export async function takeSource(kind: 'feed' | 'folder', source: string, name?: string) {
  openRead();
  rtm.card = { kind: 'busy', what: kind === 'feed' ? `Reading the feed…` : `Looking in ${source.split('/').pop()}…` };
  try {
    const p = await invoke<{ name: string; items: string[]; count: number }>('preview_source', { kind, source });
    if (rtm.card?.kind !== 'busy') return;
    if (!p.count) { rtm.card = { kind: 'error', message: kind === 'feed' ? 'That feed has no posts yet.' : 'There’s nothing Zenpod can read in that folder.' }; return; }
    rtm.card = { kind, name: name || p.name, source, items: p.items, count: p.count, auto: false };
  } catch (e) {
    if (rtm.card?.kind === 'busy') rtm.card = { kind: 'error', message: String(e) };
  }
}

/** Make it a show: it opens on its own page. */
export async function makeShow() {
  const c = rtm.card;
  if (c?.kind !== 'feed' && c?.kind !== 'folder') return;
  rtm.card = { kind: 'busy', what: 'Making the show…' };
  try {
    const id = await invoke<number>('make_source_show', { kind: c.kind, source: c.source, name: c.name, auto: c.auto });
    await loadReads();
    rtm.card = null;
    ui.yours = { id, back: 'read' };
  } catch (e) { rtm.card = { kind: 'error', message: String(e) }; }
}

// A feed that only carries summaries: each post's page is read here, as an article is, before the post is read aloud.
const fetching = new Set<number>();
async function fetchWhole() {
  for (const r of rtm.reads.filter((x) => x.needs_text && x.link && !fetching.has(x.id))) {
    fetching.add(r.id);
    let text: string | null = null, preview = true;
    try {
      const page = await invoke<{ url: string; kind: string; html: string }>('fetch_page', { url: r.link });
      const d = page.kind === 'page' ? fromHtml(page.html, page.url) : null;
      if (d && d.text.length > r.chars) {
        text = d.text;
        // Short, and the page asks you to subscribe or sign in: a paywall let only the start through.
        preview = d.text.length < 1500 && /subscribe|sign in|log in|paid subscribers/i.test(page.html);
      }
    } catch { /* the summary will do */ }
    await invoke('set_read_text', { id: r.id, text, preview }).catch(() => {});
    fetching.delete(r.id);
  }
}

/** A file from Choose file, a drop, the Dock or Open With. A folder becomes a show of its own. */
export async function takeFile(path: string) {
  if (await invoke<boolean>('is_folder', { path })) return takeSource('folder', path);
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
  const id = await invoke<number>('add_read', { showId: c.to === 'new' ? null : c.to, newShow, title: c.title, source: c.source, text: c.text });
  rtm.current = { read: id, title: c.title, episode: null, error: null };
  rtm.card = null;
}

export const reads = {
  now: (id: number) => invoke('read_now', { id }),
  stop: (id: number) => invoke('stop_read', { id }),
  discard: (id: number) => invoke('discard_read', { id }),
  setShow: (id: number, patch: { title?: string; auto?: boolean; voice?: string }) => invoke('set_show', { id, ...patch }),
  removeShow: (id: number) => invoke('remove_show', { id }),
  deleteEpisode: (id: number) => invoke('delete_episode', { id }),
  newShow: (name: string) => invoke<number>('make_empty_show', { name }),
  move: (id: number, showId: number) => invoke('move_episode', { id, showId }),
};

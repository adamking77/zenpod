import { getCurrentWindow } from '@tauri-apps/api/window';

export type Look = '' | 'day' | 'night';

// The native window theme drives both the traffic lights and the webview's
// prefers-color-scheme, so the flavor only ever follows that one signal.
const dark = matchMedia('(prefers-color-scheme: dark)');
const sync = () => (document.documentElement.dataset.flavor = dark.matches ? 'mocha' : 'flat');
dark.addEventListener('change', sync);

// ponytail: localStorage until the SQLite settings table lands (M4); then Rust applies it before show.
export function savedLook(): Look {
  try { return (localStorage.getItem('listener.look') as Look) || ''; } catch { return ''; }
}

export async function setLook(v: Look) {
  await getCurrentWindow().setTheme(v === 'day' ? 'light' : v === 'night' ? 'dark' : null);
  sync();
  try { localStorage.setItem('listener.look', v); } catch {}
}

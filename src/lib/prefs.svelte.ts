import { getCurrentWindow } from '@tauri-apps/api/window';
import { listen } from '@tauri-apps/api/event';
import { api } from '$lib/api';

// Settings live in SQLite; every window applies them here and follows changes.
export const ACCENTS = ['Rosewater', 'Flamingo', 'Pink', 'Mauve', 'Red', 'Maroon', 'Peach', 'Yellow',
  'Green', 'Teal', 'Sky', 'Sapphire', 'Blue', 'Lavender'] as const;

export const prefs = $state<Record<string, string>>({ look: '', light: 'cast', accent: 'Blue' });

const root = document.documentElement;
const dark = matchMedia('(prefers-color-scheme: dark)');
// The native window theme drives the traffic lights and prefers-color-scheme together.
const syncFlavor = () => (root.dataset.flavor = dark.matches ? 'mocha' : 'flat');
dark.addEventListener('change', syncFlavor);

function apply(key: string, value: string) {
  prefs[key] = value;
  if (key === 'look') {
    getCurrentWindow().setTheme(value === 'day' ? 'light' : value === 'night' ? 'dark' : null).then(syncFlavor);
  }
  if (key === 'light') root.dataset.light = value;
  if (key === 'accent') root.style.setProperty('--accent', `var(--${value.toLowerCase()})`);
}

export async function loadPrefs() {
  const s = await api.settings();
  for (const [k, v] of Object.entries({ ...prefs, ...s })) apply(k, v);
  listen<[string, string]>('settings', (e) => apply(...e.payload));
}

export const setPref = (key: string, value: string) => { apply(key, value); api.setSetting(key, value); };

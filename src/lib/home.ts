import { homeDir } from '@tauri-apps/api/path';

// Paths are shown from the home folder: ~/vault/inbox/zenpod rather than /Users/…/vault/inbox/zenpod.
let dir = '';
homeDir().then((d) => (dir = d.replace(/\/$/, ''))).catch(() => {});

export const home = (path: string) => (dir && path.startsWith(dir + '/') ? `~${path.slice(dir.length)}` : path);

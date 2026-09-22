# Listener · Build plan

**What we're building:** a local-first macOS podcast player. One window (Listening, New, Following), a Mini with the Orbit, a Pill, and a small Settings pane. No accounts, no sync, no telemetry.

**Design reference:** `mockups/listener-pass7.html`, the approved prototype. `SCOPE.md` still holds the earlier locked decisions. Where they differ, this file and pass 7 win.

---

## 1 · Stack

| Layer | Choice | Why |
| --- | --- | --- |
| Shell | Tauri 2 | Small native app, Rust core, system WebKit. Already decided. |
| UI | Svelte 5 + Vite + TypeScript | No runtime weight; the prototype's markup and CSS port almost directly. |
| Tokens | `2050-design-system/tokens.css` | Flat White / Mocha plus the 14 accents. The prototype's `--acc-l` / `--acc-d` pattern carries over. |
| Feeds | `reqwest` + an RSS parser crate | Browser User-Agent; parse iTunes and Podcast Namespace tags (chapters, transcripts), by hand where the crate lacks them. |
| Store | SQLite via `rusqlite` | Shows, episodes, positions, peaks, settings. One file on disk. |
| Peaks | `symphonia` | Decode each episode once and store 360 loudness points. Drives the Field and the resting Orbit. |
| Audio | HTMLMediaElement in a hidden player window, audio served through a Tauri custom protocol | Same-origin audio means Web Audio's analyser works live on every episode. Podcast hosts rarely send the CORS headers that would allow it otherwise. |
| System | `souvlaki` | Media keys and Now Playing. |
| Windows | `tauri-nspanel` (Mini, Pill), `window-vibrancy` (their material), `tauri-plugin-window-state`, `tauri-plugin-positioner` | Floating panels that don't steal focus, the desktop's material, and remembered positions. |
| Search and matching | Podcast Index API (free key) | Spotify import matching now; search later. |

## 2 · Architecture

```
Rust core (single source of truth)
  feeds · store · downloads · peaks · import · now-playing
        │  events (state flows one way)
        ▼
Player window (hidden, always alive): the <audio> element, the analyser
Main window · Mini · Pill: render state, send commands
```

- The Rust core owns position, queue, speed and settings. Windows only render and send commands, so the three surfaces can never disagree.
- Audio lives in its own hidden window, so closing or folding any visible window never stops playback.
- Every audio URL goes through `listener://audio/<id>`, which serves the local file if it's downloaded and streams it with range requests if not.

## 3 · Visualizations

Port the Zen audio reader's draw functions (Field, Thread, Orbit, the tether thread) into one Svelte component: `<Signal mode="field|thread|orbit|tether" />`. They share one live-signal store, fed by the analyser. Two changes from the component, both already in pass 7:

1. **Resting Orbit:** when paused, the rays hold the episode's loudness shape, with the played part in the accent.
2. **Light-flavor ink:** 1.6× alpha on Flat White and Latte.

## 4 · Production notes (differ from the prototype)

- **Controls:** the Field / Thread / Orbit switch and the speed control sit down with the player controls, not in the top-right. Suggested row: `time · field thread orbit · ⟲ ▶ ⟳ · 1.4× · time left`. Settle the exact layout in M3.
- **Settings:** accent (14 Calmppuccin colours, Flat White value by day, Mocha by night), appearance, light, bring your shows. Stored in SQLite, not browser storage.
- **Waveform edges** fade into the ground, 16% each side, and never touch the window edge.
- **Morph between modes:** the artwork travels between window, Mini and Pill. In Tauri these are separate windows, so the travel needs care: animate within the window that's closing, then open the next one already in place. Prototype the timing in M6.

## 5 · Bringing shows in

| Source | How | Notes |
| --- | --- | --- |
| Any podcast app | OPML file | Overcast, Pocket Casts and most others export it. The feed addresses are included. |
| Spotify | `YourLibrary.json` from Spotify's "Download your data" | Names and publishers only. Match each show against Podcast Index by name and publisher. Shows that exist only on Spotify stay listed as Spotify-only. |
| Apple Podcasts | No native export on Mac, as far as we know | A later helper could read its local library. Not v1. |
| A feed address | Paste it | Always available. |

We skip the Spotify API itself: since February 2026 it requires Premium and caps apps at 5 test users, and it returns no feed addresses anyway.

## 6 · Milestones

Each ends with the same gate: WebKit renders on disk, full keyboard operation, and your review against pass 7.

| # | Delivers | Done when |
| --- | --- | --- |
| M0 | Scaffold: Tauri 2 + Svelte, tokens wired, main window without a title bar, Day and Night | The empty window matches pass 7's frame. |
| M1 | Feeds and store: add by address, parse, refresh, SQLite | A real feed shows in New and Following. |
| M2 | Playback: hidden player window, custom protocol, position memory, speed, media keys | Resume works after quitting; keys work with every window closed. |
| M3 | Listening pane: Signal component (Field, Thread, Orbit), analyser, peaks, final control row | The visualizations move with real audio and settle on pause. |
| M4 | Settings and import: accent, appearance, light, OPML, Spotify matching | Your real subscriptions are in the app. |
| M5 | Downloads and chapters | An offline episode plays; chapters come from the feed. |
| M6 | Mini and Pill: panels, material, the morph, window memory | All three share one state and remember their places. |
| M7 | Polish: motion pass, reduced motion, empty states, transcripts when present | The polish pass is complete. |
| M8 | Signed app bundle | Opens cold and plays within one click. |

## 7 · Risks to check early

- `tauri-nspanel`: an open issue reports panels taking focus like normal windows on macOS 27. Test in M0 with an empty panel.
- A hidden WebKit window keeps playing audio: verify in M2 before building on it.
- Range requests through the custom protocol: seeking into a streamed episode must stay instant.
- Spotify matching accuracy: expect some misses; show the match and allow a correction.

## 8 · How we work

A new repo, `listener`, with `SCOPE.md`, this file and the pass 7 prototype checked in as the reference. Claude builds one milestone at a time in that repo; you review at each gate. Nothing moves to the next milestone until the current one is done.

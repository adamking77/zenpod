# Listener · Scope

**Working name:** Listener (placeholder; the two-node mark holds the seat).
**One sentence:** A local-first podcast player for macOS where the interface recedes, the artwork is the light, and listening has a room.

Local-only. No accounts, no sync service, no telemetry. OPML is the portability layer. Built by Adam and Claude, September 2026.

---

## 1 · Locked decisions

### Product

| Decision | Call |
| --- | --- |
| Framework | Tauri 2: Rust core + system WKWebView. One player core, three windows. |
| v1 features | Core (RSS follows, episode lists, playback with speed/skip/resume, OPML import/export) + offline downloads + chapters + transcripts-when-present + up-next queue. |
| Deferred to v1.1 | Search & discovery (iTunes Search API). Migration covered by OPML export from Apple Podcasts. |
| Window modes | Main (resizable), Mini (380 wide, always-on-top when chosen), Pill (frameless 240–468 × 44, always-on-top, skip-taskbar, opens upward on click only, never grows on its own). |
| Platforms | macOS on Apple Silicon first; the web UI is portable by construction. |

### Design

| Decision | Call |
| --- | --- |
| Composition | Fusion: Zen OS layouts over round-1 players and materials. |
| Main page | One scroll: the Room (first viewport) → the Shelf (covers as kept things) → the Queue (vertical spine). ≤ 45 words per section; marginalia, not lists, on the front surface. |
| Players | Round-1 set, unchanged in spirit: persistent dock (orbit + transport + title + time + speed), Show side-by-side (Field waveform + full schematic timeline + episode rows), Mini (artwork, orbit, spine, transport, episodes dropdown that folds the artwork aside), Pill (thread strip, opens upward). |
| Type | Hanken Grotesk (reading) + Fragment Mono (notation). No serif; the zen round's Newsreader was explored and set aside. |
| Color | 2050 Calmppuccin machine: 7 flavors, 14 accents with a picker, contrast steps (Calm/Clear/Strong), one/two planes. |
| Accent law (amends 2050 rule 9) | The accent marks where you are and what you touched: played spans in every waveform and progress rule, active toggles, hover and selection tints, primary transport. Unplayed material stays quiet ink. |
| Light | Flat by default (2050-pure). Cast — the artwork throwing an ambient glow — is the person's choice in Settings, on the same precedent as the second plane. |
| Viz per surface | Field (48 bands) in the full player · Orbit in the dock and mini · Thread along the pill. One family: same played-ink, same accent head, three scales. (2050 Pulse precedent.) |
| Voice | Sentences, second person, invitation never demand. Time values appear on player surfaces only. Empty states are scenes, not prompts ("The room is quiet."). |
| Schematic grammar | ZF: 1px rules, open circles for chapters, one filled accent head, sparse ticks, plate labels with one-bend leaders. Solid = current, dashed = former. |
| Motion | Nothing ticks at rest; visualizations move only while audio plays and settle on pause. Reduced motion = complete final state. |

### Engineering lessons (binding)

1. **Flow-first shell.** The app shell is normal document flow; the window scrolls. Only the strip, dock, and full-screen stages use `position: fixed`. No `position: absolute` with `left/right` anchored to the viewport. (Proven necessary by the in-app-browser bisect; robust everywhere.)
2. **Non-blocking fonts.** Font stylesheets load `media="print"` + `onload` swap; the page paints instantly in fallback type.
3. **Structural audit after every markup change.** An HTML balance check runs with every build; a silent no-op in a string replace caused the jumble bug.
4. **`color-mix()` carries plain fallbacks** declared before every mixed value.
5. **Verify in WebKit and Chromium** for every frame (Playwright WebKit + Chrome headless), plus static renders on disk per frame.

---

## 2 · Architecture

```
┌ Tauri 2 ──────────────────────────────────────────────┐
│  Windows: Main · Mini · Pill (multiwebview, one app)   │
│  Web UI: TypeScript + Vite + Svelte 5 (compile-step,   │
│  no runtime weight); zen-audio-reader patterns reused  │
├───────────────────────────────────────────────────────┤
│  Rust core                                             │
│   feeds: reqwest + rss/quick-xml, refresh scheduler,   │
│          browser User-Agent (some CDNs block default)  │
│   store: rusqlite (shows, episodes, positions, queue,  │
│          downloads) + artwork cache via custom         │
│          protocol; OPML import/export                  │
│   downloads: streaming to disk, per-show rules,        │
│          auto-cleanup                                  │
│   playback state: single source of truth; all windows  │
│          render it via events                          │
│   system: souvlaki (media keys, Now Playing)           │
└───────────────────────────────────────────────────────┘
```

- Audio plays through the webview's `HTMLMediaElement` (HTTP streaming or local file via custom protocol). The Rust core owns *state* (position, queue, rates), the webview owns *rendering*; they never disagree because state flows one way.
- Media keys and Now Playing work regardless of which window is visible.
- Chapters: Podcast Namespace `chapters` JSON first, ID3v2 chapters second. Transcripts: Podcast Namespace `transcript` tag; render when the feed provides it, never fabricated.

## 3 · Screens

| Surface | Anatomy |
| --- | --- |
| Main | Room (eyebrow + corner words · artwork 300 + sentence + mono subline · schematic timeline · two marginalia facts) → Shelf (covers on a line, mono captions, "bring another show") → Queue (vertical spine, accent top node, quiet close). Dock persistent. |
| Show | Side-by-side: artwork + name + about + mono meta + follow choices | continue-player (Field + timeline) + episode rows with thumbnails. |
| Mini | Artwork full-bleed → orbit with flanking time → spine → transport → episodes dropdown (artwork folds aside when open). |
| Pill | Thread strip doubling as progress + time + one control; click opens the folded-room panel upward; identity from the artwork. |
| First Light | Empty state as a scene: "The room is quiet." + two choices (Bring my list · Paste an address). |
| Settings | Flavor, accent, contrast, planes, light (Flat/Cast), speed defaults, skip intervals, download rules. |

## 4 · Build plan

| Milestone | Delivers | Done when |
| --- | --- | --- |
| M0 Scaffold | Tauri 2 + Vite + Svelte, tokens.css wired, flavor/accent/contrast switching, flow shell | All flavors render Main in WebKit + Chromium; audit green |
| M1 Feeds | Add by URL, parse, store, refresh; Shelf + Show data-real | Real feed renders shows/episodes; refresh quiet |
| M2 Playback | HTMLMediaElement, position memory, speed/skip, media keys + Now Playing | Resume across relaunch; keys work with window hidden |
| M3 Library UX | Main (Room/Shelf/Queue) + Show real-state; OPML in/out | Apple Podcasts OPML imports cleanly |
| M4 Downloads + Queue | Rust download manager, per-show rules; queue drain | Offline episode plays; queue auto-advances |
| M5 Chapters + Transcripts | Namespace chapters + ID3; transcript surface when present | Timeline nodes from real chapters |
| M6 Modes | Mini and Pill windows, mode switching, window-state memory | All three modes share one player state |
| M7 Polish | Motion pass, keyboard everywhere, reduced motion, First Light, Settings | Full keyboard operation; reduced motion complete |
| M8 Package | Signed app bundle, local distribution | Opens cold to the Room, playing within one click |

Every milestone ends with the verification gate: WebKit + Chromium frame renders on disk, structural audit, and the blend's quality list — one signal, every mark explains, a place to rest, hierarchy survives grayscale.

## 5 · Risks

- Feed CDNs blocking non-browser clients → browser User-Agent, retry backoff.
- Chapter/transcript coverage is spotty in the wild → features degrade to silence, never to stubs.
- Pill window behaviors (always-on-top, click-through zones) need macOS validation early in M6.
- In-app browser preview quirirk (absolute-positioned shells) → already designed out; WebKit verification stands guard.

## 6 · Sources

2050-design-system (PRINCIPLES, THEMES, COMPONENTS, tokens) · Zen Futurism (DESIGN.md, audio-reader) · GenZen OS / Astra concepts (Desktop) · BLEND-MAP T7 adoption precedent · mockups: `index.html` (round 1–2), `zen.html` (zen round), `fusion.html` (the direction), `renders/` (verified frames).

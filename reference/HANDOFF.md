# Handoff · Listener (podcast app)

From a Cowork session with Adam, 22 September 2026, to Claude Code. Read this first, then `BUILD.md`.

## What this is

A local-first macOS podcast player, so Adam doesn't need Spotify or Apple Podcasts on desktop. It has one window (Listening · New · Following), a Mini with the Orbit, a Pill, and a small Settings pane. No accounts, no sync, no telemetry. Built on Tauri 2.

## How to work (binding)

- **Run every milestone through `/ponytail` and `/graft`.** Don't overcomplicate, and don't build from scratch what an existing plugin, crate, template or piece of Adam's own code already does. Both skills are installed globally in `~/.claude/skills`.
- **One milestone at a time.** Stop at each gate for Adam's review. Don't start the next milestone unasked.
- **Diagnose before executing.** Keep reports terse and direct.
- **Adam works with PDA and energy limits.** Keep each ask small and bounded, with a clear done signal. Offer options, never pressure.
- **Keep communication calm.** No exclamation marks, no "you should".

## Where things are (`~/projects/podcast app/`)

| Path | What |
| --- | --- |
| `BUILD.md` | **The plan:** stack, architecture, production notes, import, milestones M0–M8, risks. |
| `SCOPE.md` | Earlier locked decisions. Where it conflicts with `BUILD.md` or pass 7, those win. |
| `mockups/listener-pass7.html` | **The approved design.** Open it in a browser; it's interactive. Also published at https://claude.ai/artifact/K3xaHwoAJjqSuqdX3Gm32B (private to Adam). |
| `mockups/soft-instrument.html`, `listener-pass6.html` | Earlier passes, for history only. |
| `DIRECTION-soft-instrument.md` | Pass 5 reasoning and stack research. Partly superseded. |
| `2050-design-system/` | The design system. `tokens.css` holds the Calmppuccin flavors and the 14 accents; `PRINCIPLES.md` and `QUIET.md` hold the rules. |
| `Zen Futurism/audio-reader/component/` | `<zen-audio-reader>`, Adam's framework-neutral web component (MIT) with the Field, Thread and Orbit visualizations. **Graft it; don't port it.** |
| `Zen OS/` | Visual references (the Fable and Astra concepts) for the soft, sparse feel. |

## Decisions made in this session

**Design (pass 7):**
- **Layout:** the main window has no title bar; the traffic lights sit in the content. On the left: cover (136px square), then title and show, then the waveform, then time and controls. On the right: New (grouped Today / This week / Earlier) or Following, which drills into a show.
- **Visualizations:** Field, Thread and Orbit from the Zen audio reader, unchanged except for two things:
  - When paused, the Orbit's rays show the episode's loudness shape, with the played part in the accent.
  - Ink alphas are 1.6× on light flavors.
- **Mini and Pill:** the Mini uses the Orbit with the cover disc in its centre. The Pill uses a small Orbit and opens a panel upward with the tether thread.
- **Material and light:** only the Mini and Pill use the desktop's frosted material; the main window stays matte. Light glows from the cover's position in the show's colour, with a flat option.
- **Waveform edges** fade into the background (16% each side) and never touch the window edges.
- **Mode morph:** switching between window, Mini and Pill moves the artwork continuously from one to the next.
- **Settings:** accent (all 14 Calmppuccin colours, Flat White value by day and Mocha by night, used everywhere), appearance (system / day / night), light (cast / flat), and bringing shows in.
- **Type:** Hanken Grotesk and Fragment Mono. Mono only for times and numbers.

**Production-only note, not in the prototype:** the Field / Thread / Orbit switch and the speed control move down into the player controls row.

**Importing shows:**
- **OPML:** read directly; the feed addresses are included.
- **Spotify:** read `YourLibrary.json` from Spotify's "Download your data", then match each show to its public feed through the Podcast Index API by name and publisher. Shows that exist only on Spotify are listed as such. Don't use the Spotify Web API: it returns no feed addresses, and since February 2026 it requires Premium and caps apps at 5 test users.
- **Apple Podcasts:** no OPML export on Mac, as far as we know. Not in v1.

**Architecture (see `BUILD.md` §2):**
- The Rust core is the single source of truth.
- A hidden player window hosts the `<audio>` element, so playback survives any window closing.
- All audio goes through a custom protocol, `listener://audio/<id>`, so it's same-origin and the analyser works.
- `symphonia` computes the loudness peaks once per episode.

## Open questions for Adam

1. Peach, Red and Green are reserved roles in 2050 (question, failed, ok). The prototype lets all 14 be chosen as the accent. Keep that, or hide those three?
2. Exact order of the production controls row. The suggested order is in `BUILD.md` §4; settle it in M3.
3. Adam wants `/ponytail` and `/graft` also available in Cowork. Copying the two skill folders from `~/.claude/skills` into `podcast app/skills-import/` lets a Cowork session read them and add them as account skills.

## Next step: M0

Scaffold with `create-tauri-app` (Svelte + TypeScript). Then:

1. Wire in `2050-design-system/tokens.css` (Flat White and Mocha, the accent variables).
2. Build the main window with no title bar and the traffic lights inset.
3. Add Day, Night and system switching.
4. Make a throwaway `tauri-nspanel` test panel to check the macOS 27 focus issue early (`BUILD.md` §7).

**Done when:** the empty window matches pass 7's frame (proportions, divider, ground colour, light in both themes), and the nspanel test result is reported.

Run `/ponytail` and `/graft` before writing code. Report back in a few lines and stop at the gate.

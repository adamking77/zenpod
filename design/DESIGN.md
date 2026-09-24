# Zenpod design system

Version 0.3.0. This file describes the app as it ships, not as the mockups imagined it. The code is the source of truth: `src/lib/tokens.css` (flavors and roles), `src/lib/app.css` (Zenpod's layer on top), and each component's `<style>`. `design/tokens.json` mirrors them for tools. `design/guide.html` shows all of it rendered.

Brand, voice, the mark and the wordmark (ZENPOD in Lexend Giga ExtraLight) live in [BRAND.md](BRAND.md).

## Principles

1. **One shape per episode.** Each episode is drawn once, from its loudness. Every surface shows the same shape at a different scale. Nothing else in the interface competes with it.
2. **Never invent.** Until an episode has been measured, its shape is flat and quiet (360 points at 0.04). No placeholder waveform, no fake progress.
3. **Colour means something.** The accent marks exactly four things: the played part, what is playing, what you're pointing at, and links. Red appears only while you hover a word that removes something.
4. **Words, not widgets.** A choice is a row of words. The chosen one is in full ink and the rest are faint. No switches, no checkboxes, no pills, no filled tabs.
5. **Out of the way.** The floating surfaces never take focus from the app you're working in. You can drag them from anywhere that isn't a button.
6. **Motion is continuity.** Things arrive and leave; nothing spins. Nothing you trigger takes longer than 300ms. Reduced motion keeps the states and drops the movement.

## Colour

### Two looks

| | Day | Night |
| --- | --- | --- |
| Flavor | Flat White (`data-flavor="flat"`) | Mocha (`data-flavor="mocha"`) |
| Ground | `#fbfaf8` | `#1b1b29` |
| Surface | `#ffffff` | `#20202f` |
| Text | `#3f3f48` | `#dfe4f0` |
| Muted | `#62626b` | `#8a90a6` |
| Line | `#d8d5d0` | `#2a2a3d` |
| Hair | `rgba(0,0,0,.07)` | `rgba(255,255,255,.06)` |

Settings offers Follow the system, Day or Night. The native window theme follows too, so the traffic lights match.

### Ink steps

Components never use a raw grey. They use one of five ink steps, mixed toward the ground so every flavor keeps the same steps:

| Role | Mix | Use |
| --- | --- | --- |
| `--text` | 100% text | Titles, the chosen word, primary controls |
| `--text-mid` | 80% text | Reading: notes, transcripts, chapters |
| `--text-dim` | 72% muted | Meta lines, icons at rest, numbers |
| `--text-faint` | 44% muted | Labels, unchosen words, placeholders |
| `--hair` | flavor | Dividers, empty cover, hover fill |

A played or finished episode drops to 50% opacity rather than a new colour.

### Accent

Fourteen accents, chosen in Settings, default **Blue** (`#355aa0` Day, `#7fa6e6` Night). The accent is applied as `--accent: var(--blue)` on the root, so it follows Day and Night by itself. The accent is used for:

- the played part of the shape and its head
- the playing row, and a row's title on hover
- a hovered play-button ring, the library resize edge
- links in notes, the line being spoken in a transcript, "3 new", "Just added", "Now following"
- the focus ring

It is never a fill behind text and never a background.

### The light

The main window is lit from the cover. `--cast` is taken from the artwork at run time: its most saturated pixels, averaged, then set to `hsl(h, clamp(25%, s, 60%), 60%)`. Near-grey artwork falls back to `#6f8fc9`. It is drawn as one radial gradient centred on the cover (116px, 132px), 1000px wide at 34% in Night and 880px at 30% in Day. The Mini carries a smaller version under its Orbit. Settings → Light → Flat turns it off.

## Type

Two faces, both bundled with the app through Fontsource:

- **Hanken Grotesk Variable** for everything you read. Light weights (250–300) for anything 16px or larger.
- **Fragment Mono** for numbers, with tabular figures.

The wordmark's face, Lexend Giga, is brand only. The app doesn't bundle it, and no interface text uses it.

| Token | Weight / size / leading | Use |
| --- | --- | --- |
| display | 250 / 28 / 1.18, −0.012em | Now-playing title (max 22ch, 3 lines), the empty room |
| heading | 250 / 24 / 1.2 | Show page, episode notes, Settings |
| title-s | 300 / 16.5 / 1.3 | Mini title (2 lines, centred) |
| row | 400 / 14.5 / 1.35 | List rows, library tabs |
| body | 400 / 14 / 1.5 | Default; reading text at 1.6, max 42ch |
| small | 400 / 13.5 | Show description, chapters, quiet notes |
| sub | 400 / 12.5 | The meta line under a title |
| label | 400 / 12 | Day headers, Settings labels, always faint |
| num | Mono 11, +0.02em | Times, durations, speed |
| caps | Mono 10.5, +0.1em, uppercase | Only the Field / Thread / Orbit switch |

Body letter-spacing is 0.01em. Sentence case everywhere. Headings use `text-wrap: balance`.

## Layout

The main window is two panes, **Listen** and **Library**, with no sidebar and no toolbar. The title bar is an overlay, so the whole window is the plane.

- **Listen** (left, at least 620px): cover and title at the top, the shape in the middle, the transport at the bottom. Padding is 64 / 48 / 44. When it narrows below 576px, the transport moves onto its own line and the readouts sit beneath it.
- **Library** (right, 300–560px, default 400): tabs (New, Following) and the three mode icons (Mini, Pill, Settings) in a 36px header, then one scrolling list with no visible scrollbar. The panes are divided by one hairline. Drag the edge to resize; double-click it to reset.

Rows sit 9px apart vertically, with a 14px gap between cover and text. Sections in Settings are separated by a hairline with 22px on each side. The app has no cards.

## Components

**Play button.** A circle with a 1px `--line` inset ring and no fill. The ring turns accent on hover. It is 50px in the window, 42 in the Mini, 40 in the panel, and 32 with no ring in the Pill (a `--hair` fill on hover).

**Icon button.** A 28px hit area with the glyph in `--text-dim`, turning `--text` on hover. Previous and next episode are `--text-faint`, at 35% when there's nothing to step to.

**Word group** (`.words`, `.tabs`, `.reading`, `.vz`). A row of plain words 18px apart, marked with `aria-pressed`. Unchosen words are faint, dim on hover, and the chosen one is full ink. Used for tabs, sorting, Notes/Transcript, Appearance, Light and autoplay.

**Row.** A cover (34px for an episode, 40px for a show, 6px radius), a title in row type, and a meta line in `sub` plus a `num`. The title turns accent on hover and while playing. Titles cut off with an ellipsis and never wrap.

**Text field.** No box. A 1px `--line` underline that turns accent on focus, with a faint placeholder that says what to paste.

**Accent swatch.** A 22px circle in a 7-column grid. The chosen one gets a 1px ring 5px out, in its own colour. It scales to 1.12 on hover.

**Remove.** Removing something is a word, never a dialog: "Stop following" becomes "Press again to stop following" and turns red on hover.

**Quiet note.** A result sentence under what caused it ("3 new shows, 2 you already had."). It arrives from 4–8px below with `@starting-style`, once.

**Focus.** A 1.5px accent outline, 3px offset, 4px radius, on keyboard focus only.

### Icons

Line glyphs in `currentColor`, no fills, 1.1px stroke (1.2px for play and pause), round joins, on 14/16/20px grids. They are all in `src/lib/icons.ts`. Draw new ones to match: open, light and geometric, with no filled shapes.

## Surfaces

| Surface | Size | Radius | Holds |
| --- | --- | --- | --- |
| Window | resizable | system | Listen + Library, matte ground, cast light |
| Mini | 240px Orbit + text | 18 | Orbit around a 72px artwork disc, title, transport. Window and Pill buttons appear on hover |
| Pill | 48px tall | 24 | 46px Orbit around a 13px disc, title and time, play |
| Panel | under the Pill | 18 | 44px cover, title, tether line, five controls |

The floating surfaces are `--surface` at 62% over the macOS vibrancy material, with a 0.5px inset `--hair` edge. They are `NSPanel`s that never activate the app (`modes.rs`). Changing mode folds the current surface to 0.96 scale and fades it out over 200ms, and the next one arrives over 280ms.

## The shape

The episode's loudness is stored as peaks in Rust and drawn on a canvas that reads its colours from the tokens (`resolveColors` in `signal.js`). There are three ways to see it, chosen under the shape:

- **Field.** Thin vertical bars across the full width, played bars in accent, a baseline, an accent line up to the head, and a 3.6px accent dot. Bars near the head wake up with the live audio.
- **Thread.** Three strands around one line. The accent strand leads, and the two ink strands trail at 28% and 12%.
- **Orbit.** The same loudness bent into a ring of 72 rays. The Mini and the Pill always use Orbit.

The panel uses a thin **tether** line. Chapters sit on the shape. Light flavors draw ink at 1.6× strength so the shape holds up on a pale ground. Clicking the shape seeks.

## Motion

| Token | Value | Use |
| --- | --- | --- |
| ease-out | `cubic-bezier(.23, 1, .32, 1)` | Default |
| ease-in-out | `cubic-bezier(.77, 0, .175, 1)` | Moving on screen |
| hover | 140ms, ease | Colour changes |
| pane | 200ms | Notes and new rows arriving, fold-away |
| arrive | 280ms | Next surface after a mode change |
| light | 400ms | Cast light on or off (ambient) |

Only colour, opacity and transform animate. `prefers-reduced-motion` removes transitions and animations entirely. The app uses no animation library.

## Drift to clean up

These are known gaps between the system and the code. None of them is visible to someone using the app.

- `tokens.css` still carries the full 2050 system: five flavors, the contrast steps, agent hues, chart and dock tokens. Zenpod uses none of them.
- Components hard-code `0.14s ease` where `--dur-base` exists, and the `.ico` and `.play` rules are repeated in four files.
- The Night icon's accent is `#88a6e0` and the Night Blue is `#7fa6e6`. They are close, but they aren't the same value.

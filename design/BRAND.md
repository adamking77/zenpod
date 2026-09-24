# Zenpod brand and style guide

How Zenpod looks, sounds and names things. The interface system is in [DESIGN.md](DESIGN.md). `guide.html` shows both rendered.

## What Zenpod is

**A quiet podcast player for macOS.**

The long version: each episode appears as a single shape drawn from its loudness, so you can see where it gets busy before you play it. The player floats out of the way and never takes focus from what you're doing.

Zenpod is quiet, exact and unhurried. It doesn't try to keep you in the app. It has no badges, no streaks, no red dots, no recommendations and no social features. When there's nothing to do, it says so and waits.

## The name

- Write it **Zenpod**: one word, capital Z. Never ZenPod, Zen Pod or ZENPOD in running text.
- Internally the app is still called `listener`: the folders, the crate, the `listener://` scheme and the bundle id `me.adamking.listener`. That's deliberate, because changing the bundle id would move the library. None of it should ever appear in front of a user.

## The mark

The icon is an **Orbit**, the same instrument the Mini and the Pill show. It's a ring of rays whose length follows an episode's loudness. The played third, from the top right round to the bottom, is in the accent, with a solid head dot where playback is. The rest of the ring is in ink that fades as it goes, so the brightest point is always the one just played.

| Variant | Ground | Accent | Ink |
| --- | --- | --- | --- |
| Light | `#f4f2ef` | `#355aa0` | warm greys, fading lighter |
| Dark | `#1d1c27` | `#88a6e0` | cool greys, fading darker |
| Tinted / Clear | system | the Dark accent layer | system tint |

The source is `src-tauri/icons/Zenpod.icon` (Icon Composer, two layers: `accent` and `ink`). The compiled light and dark icon is `Assets.car`, built by the `Icon` GitHub workflow.

**Do**

- Keep the ring's opening, the head dot and the played third where they are. The asymmetry is the point: it's a reading, not a logo shape.
- Give it clear space of at least a quarter of its width.
- Use the light mark on light grounds and the dark mark on dark grounds.

**Don't**

- Rotate it, fill the ring, close the gap or make the rays even.
- Recolour the played third anything but the accent, or recolour the ink.
- Add shadows, glass, gradients or a wordmark inside the tile.

## The wordmark

The app has no custom logotype yet. `guide.html` sets six calm, futuristic candidates beside the mark so they can be compared. The recommendation there is a proposal, not a decision, until Adam chooses one.

Whichever is chosen:

- Lowercase `zenpod` or sentence case `Zenpod`, set light (200–300) with open tracking. Never bold and never in capitals.
- Set it in ink, never in the accent. The accent belongs to the mark's played third.
- The wordmark is only for the brand: the README, the website, the DMG window and the About panel. Inside the app, the interface stays in Hanken Grotesk.

## Colour

The brand colours are the app's colours. There is no separate marketing palette.

| Name | Day | Night | Role |
| --- | --- | --- | --- |
| Ground | `#fbfaf8` | `#1b1b29` | Every background |
| Ink | `#3f3f48` | `#dfe4f0` | Words and the mark's ink |
| Zenpod Blue | `#355aa0` | `#7fa6e6` | The default accent and the mark |
| Cast | from the artwork | from the artwork | The window's light |

The other thirteen accents belong to the person, not the brand. Use Blue in any brand material.

## Type

- **Hanken Grotesk** for everything: light weights for large text, regular for body.
- **Fragment Mono** for numbers and times only.
- For the wordmark, see above.

## Voice

Zenpod talks like a calm person in the room with you. It's short, plain and specific. It says what happened, then what you can do.

### Rules

1. **Say it like a person, not a system.** An empty player is "The room is quiet.", not "No episode playing."
2. **Use the verbs people use.** Bring your shows, Keep offline, Stop following, let it go. Don't say import, download, unsubscribe or delete.
3. **Say what happened, then what to do.** "No public feed was found for this show, so it can't be played here. If it has one, paste its address below."
4. **Be exact.** "3 new shows, 2 you already had." "12 min left." Don't write "some" or "a few".
5. **Never blame, never apologise.** "Couldn't reach these 2. Their feeds may be private or gone." Don't write "Oops", "Sorry" or "Something went wrong".
6. **Name the machinery only when the person has to find it**, and say where it comes from: "An OPML file, from Overcast, Pocket Casts or most other podcast apps."
7. **Confirm in words, not dialogs.** "Press again to stop following."

### Mechanics

- Sentence case for everything, including buttons, tabs and headings.
- Full sentences end with a full stop. Labels and buttons don't.
- No exclamation marks and no emoji.
- British spelling: colour, notarised, favourite.
- Numbers as digits. Durations are "12 min left" in the window and "−12 min" in the Mini.
- Times use a middle dot as a separator: `38:51 · 12 min left`.
- "←" for back links: "← Following".

### Words

| Say | Not |
| --- | --- |
| show | podcast, channel |
| episode | track, item |
| Following | Subscriptions, Library |
| New | Inbox, Feed, Up next |
| Keep offline | Download |
| Stop following | Unsubscribe, Delete |
| Bring your shows | Import |
| feed address | RSS URL |
| the Mini, the Pill | mini player widget, compact mode |
| The room is quiet. | Nothing playing |

## Imagery

- Show the app with real shows and real artwork. Never use lorem ipsum or grey boxes.
- Show both Day and Night, with the cast light on.
- Crop to the window. Use the macOS shadow and no device frames.
- Screenshots live in `docs/screenshots/` and are named for what they show (`main.png`, `mini.png`, `orbit-day.png`).

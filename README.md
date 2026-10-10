# Zenpod

A quiet podcast player for macOS.

![Zenpod, playing an episode](docs/screenshots/main.png)

Each episode appears as a single shape drawn from its loudness, so you can see where it gets busy before you play it. Click the shape to jump to that point.

## Three ways to see an episode

| Orbit | Thread |
| --- | --- |
| ![Orbit, in Day](docs/screenshots/orbit-day.png) | ![Thread, in Night](docs/screenshots/thread-night.png) |

Field is the third, shown at the top. Chapters sit on the shape, and the played part takes your accent colour.

## Out of the way

The Mini and the Pill float above other windows. They never take focus from the app you're working in. Drag them from anywhere that isn't a button.

<table>
  <tr>
    <td><img src="docs/screenshots/mini.png" width="260" alt="The Mini"></td>
    <td>
      <img src="docs/screenshots/pill.png" width="406" alt="The Pill"><br><br>
      <img src="docs/screenshots/pill-card.png" width="406" alt="The Pill with its card open">
    </td>
  </tr>
</table>

## What it does

- Follows shows by feed address or by name, and checks them hourly.
- Imports your shows from an OPML file (Overcast, Pocket Casts and most other apps) or from your Spotify data export.
- Press M to keep the last 30 seconds as a note, and write beside it on the shape. Notes are listed per episode and in a Notes tab, and can be written to a folder as Markdown, where each note's time opens Zenpod at that moment. Notes can be written in the Mini too.
- Read to me makes your own shows from what you read. Paste a link or text, drop a file (PDF, Markdown, Word, text, HTML), or turn a blog's feed or a folder into a show that keeps getting new episodes. Each becomes an episode with its shape, a transcript and chapters, read by a voice on your Mac, a model you run yourself, or a service with your own key (OpenAI, Gemini, Grok, MiniMax, Inworld, ElevenLabs). Keys stay in the Keychain.
- Opens an episode's show notes beside the player, with links you can follow.
- Shows chapters and transcripts when the feed has them.
- Keeps episodes for offline listening.
- Day, Night or follow the system, with fourteen accent colours. The window light can take its colour from the show's artwork.

| Following | An episode's show notes |
| --- | --- |
| ![Following](docs/screenshots/following.png) | ![An episode's show notes](docs/screenshots/episode.png) |

## Build

Needs macOS 13 or later, Rust, Node and pnpm.

```bash
pnpm install
pnpm tauri build
```

The app and DMG land in `src-tauri/target/release/bundle/`. The build is signed ad hoc, not notarised, so the first time you open it, right-click the app and choose Open.

For development:

```bash
pnpm tauri dev
```

Built with Tauri 2 and SvelteKit.

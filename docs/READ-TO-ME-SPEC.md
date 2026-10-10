# Zenpod: Read to me

Spec · 10 October 2026 · approved for build

Read to me lets you make your own shows from what you read. An article, a file or pasted text becomes an episode. A blog's feed or a folder becomes a show that keeps getting new episodes. Zenpod reads everything aloud with a voice you choose: one already on your Mac, a model you run yourself, or a service you pay for with your own key. Every result is an ordinary episode, with a shape, a transcript, chapters and Notes.

- **Prototype:** `mockups/read-to-me/index.html`, approved 10 October 2026. It's the reference for layout, wording and behaviour. Where this spec and the prototype disagree, this spec wins.
- **Research:** the layout was checked against Spotify's Your Library, Readwise Reader, ElevenReader, Apple Podcasts and Music on Mac, Linear and Arc. Choose folder and the R key came from that review.
- **Background:** idea 4 in `FUTURE-ENHANCEMENTS-CLAUDE.md` ("Anything becomes an episode").
- **Provider facts:** checked against each provider's API reference on 9 October 2026 (section 7). Model and voice names change often, so Zenpod never hardcodes a list of them.

## 0 · Naming

- **Read to me** is the feature, its icon, its pane and the default show for things you add one at a time. It says what happens in the words you'd use to ask for it.
- **Your show** is what Zenpod calls a show you made, wherever it appears beside podcasts.
- **Voice** is the setup in Settings.
- **In code** the feature is `read` (`read.rs`, `voice.rs`, the `reads` table). Nothing visible says "read aloud", "TTS" or "text to speech".

## 1 · Decisions

| Question | Decision |
| --- | --- |
| Where it lives | An icon at the top right of the sidebar, left of Mini, Pill and Settings. It opens the Read to me pane, the way the gear opens Settings. |
| The sidebar header | Two rows: the icons stay at the top right; New · Following · Notes move to their own row below. |
| What you can give it | A link, a file, pasted text, a blog's feed, or a folder. |
| One-off items | Go into a show you pick: Read to me (the default), any show you made, or a new one named on the spot. |
| Feeds and folders | Each becomes a show of its own. New posts or files either wait for you or are read automatically, your choice per show. The default is to wait. |
| A podcast feed pasted here | Zenpod says it already has audio and offers Follow it. |
| Where your shows appear | In Following, among your podcasts, in the same sort order, with "Your show" under the name. Their episodes appear in New like any others. |
| Voice | One default in Settings › Voice. Each show can override it with its own voice. |
| Voice setup | This Mac (free, offline) or A service: Service, API key, Model, Voice, and Address for OpenAI-compatible. Model and Voice are plain text fields with working defaults. |
| Local models | Through the OpenAI-compatible service at an address such as `http://localhost:8880/v1`. Zenpod doesn't download or run models itself. |
| API keys | In the macOS Keychain, one per service. Never in the database or the settings file. |
| First use | Works with no setup, using the Mac's default voice. |
| Reading order | One episode at a time, in the order asked for. The rest say "Next to read". |
| Listening while it reads | Not in this version. An episode appears when it's finished; progress shows while it's made. |
| Prices | Not shown. They change too often to keep right. The character count lets you work it out. |

## 2 · Where it lives

### The sidebar header

The library header becomes two rows. This applies everywhere, not only to Read to me.

```
                          [Read to me] [Mini] [Pill] [Settings]
New   Following   Notes
```

- **Icons:** Mini, Pill and Settings keep their exact places. The Read to me icon is added to their left.
- **Tabs:** New · Following · Notes sit on their own row beneath, left-aligned with the list.
- **Settings and Read to me** are panes, not tabs: opening one leaves no tab underlined, as Settings does today. Clicking the same icon again closes the pane.

### The icon

- **Glyph:** a page whose last lines have become sound bars. Tooltip "Read to me (R)".
- **R** opens the pane with the field focused, from anywhere in the main window when no text field has focus. It pairs with M for notes.
- **The dot:** a small accent dot at its top right while something is being read or a new episode of yours hasn't been played. That's the only signal; there are no notifications or pop-ups.
- **On:** it takes `--text-dim` while its pane is open, like the gear.

### The pane

```
Read to me
Turn articles, feeds, files and folders into shows of your own.

[ Paste a link, a feed or text, or drop a file ]   Choose file   Choose folder
(the check card replaces the field while something is waiting to be read)

Reading
  ▤ The Case for Slow Listening     Research · Reading 2 of 8   ━━──
  ▤ Why Transcripts Matter          Read to me · Next to read

Your shows
  [art] Read to me      Things you add one at a time
  [art] Morning brief   1 new · ~/vault/briefs · read automatically
  [art] Stratechery     2 ready to read · stratechery.com/feed · new posts wait for you
```

- **Field:** focused when the pane opens. Enter takes what's in it.
- **Choose file** and **Choose folder** sit beside it, so every way in can be found without knowing to drag or paste.
- **Reading:** appears only while something is being read, is waiting its turn, or has failed.
- **Your shows:** every show you've made. Read to me is always first; the rest follow in A–Z order. Clicking one opens its page.

## 3 · Your shows

### Kinds of show

| Kind | Made by | New episodes come from |
| --- | --- | --- |
| **Read to me** | Always there | Things you add one at a time |
| **A show of things you add** | Choosing "New show…" when adding something | Things you add and send to it |
| **A feed show** | Making a show from a blog's feed | New posts in the feed |
| **A folder show** | Making a show from a folder | New files in the folder |

### Its page

Opened from the pane or from Following. "← Read to me" or "← Following" takes you back to wherever you came from.

```
← Read to me
[art]
Stratechery
New posts from stratechery.com/feed

New posts   Wait for me   Read them automatically
Voice       [ Your default (Samantha)   v ]

▤ An interview about chips and power   Ready to read · 15 min  Read
▤ Apple's quiet platform shift         New · 13 min
▤ The AI margin squeeze                11 min

Remove this show
```

- **New posts / New files:** feed and folder shows only. Switching to automatic queues any waiting episodes.
- **Voice:** "Your default (…)" plus this Mac's voices, and the service's voice when a service is set. The show's episodes are read with it from then on; finished episodes aren't re-read.
- **Episodes:** newest first. Waiting ones say "Ready to read · 15 min" with **Read**. Finished ones play when clicked, show **· Delete** on hover, and dim once played.
- **Remove this show:** press twice ("Press again to remove this show and its episodes"), the same way Stop following works. Not shown for Read to me.

### Art

- **Read to me:** the Orbit drawn for small sizes: a ring of rays, the first third of the ring and its rays in the accent, and the playhead dot.
- **Every other show of yours:** its name in light type on the page colour, over a short line of sound bars whose first third is in the accent. Generated from the name, so it needs no image and matches any accent, Day or Night.
- **Episodes:** a page cover with the episode's title, as in the demo film, unless the article has a preview image.

### In Following and New

- **Following:** your shows are mixed with podcasts and sorted with them. The line under the name reads "Your show", with "2 new" before it when there are any. Read to me appears only once it has an episode.
- **New:** finished episodes of yours appear by day, with the show name where a podcast's name would be.
- **Main window:** title, cover and shape exactly like a podcast episode; the line under the title shows the show name.
- **Episode pane:** Show notes holds "From {source}" with **Open the original**, then the first paragraph. Transcript and Notes work as usual.
- **Mini and Pill:** unchanged.

## 4 · Bringing things in

### Ways in

1. **R**, or the icon, then the field: paste or type, then Enter.
2. **Choose file** or **Choose folder** beside it.
3. **⌘V** anywhere in the main window when no text field has focus. A link or text on the clipboard opens the pane with the check card.
4. **Drop** a file, a folder or a link anywhere on the main window. While you drag, the library gets a 2 px accent edge, its list fades back, and "Drop to hear it" / "It goes to Read to me" shows in the middle.
5. **Drop on Zenpod's Dock icon**, or Open With › Zenpod, for PDF, Markdown, text, Word, RTF and HTML files (Tauri file associations).
6. **`zenpod://read?url=…`**, optionally `&show=…`, using the deep-link handler Notes already registered. This lets Shortcuts, Raycast, a browser bookmark or an agent send things in.

### What Zenpod does with each

| You give it | Zenpod checks | The card offers |
| --- | --- | --- |
| A link to a page | Fetches it. RSS/Atom → feed; audio enclosures → podcast; otherwise an article. | Article card |
| A feed with audio | Its items have audio enclosures | Podcast card |
| A feed of articles | RSS or Atom without audio | Feed card |
| A folder | It's a directory | Folder card |
| A file | PDF, Markdown, text, Word, RTF or HTML | Article card |
| Text | First line becomes the title | Article card |

### The check card

**Article card** (link, file or text):

```
READY TO READ
The Case For Slow Listening            ← editable
theatlantic.com · article
23,165 characters · about 26 min
Goes into  [ Read to me      v ]       ← your shows, then "New show…"
Read by Samantha · Change
Read     Cancel
```

- **New show…** reveals a name field under the menu. Read makes the show and sends the episode into it.
- **Read by** shows the chosen show's voice. **Change** opens Settings › Voice.
- Nothing is sent to a service until you press Read. Esc or Cancel puts the field back.

**Feed card** and **Folder card**:

```
A FEED OF ARTICLES                     (or A FOLDER)
Stratechery                            ← editable show name
stratechery.com/feed/
3 recent posts                         (or 3 recent files)
· The AI margin squeeze
· Apple's quiet platform shift
· An interview about chips and power
New posts   Wait for me   Read them automatically
Read by Samantha · Change
Make it a show     Cancel
```

- **Make it a show** creates the show and opens its page. Its 3 latest posts or files arrive as episodes ready to read, or queued if automatic is chosen. Older posts are left alone.

**Podcast card:**

```
A PODCAST
Hard Fork
feeds.megaphone.fm/hardfork
This feed already has audio, so there's nothing to read. Follow it like any podcast.
Follow it     Cancel
```

- **Follow it** runs the existing add-show path and opens Following, as adding from Settings does.

## 5 · Getting the text

- **Link:** Rust fetches the page; the webview runs Mozilla Readability on it for the title, byline, main text, headings and preview image.
- **PDF:** pdf.js in the webview. A scanned PDF with no text says so and stops.
- **Word, RTF, HTML:** macOS `textutil -convert txt`.
- **Markdown and text:** read directly. Markdown syntax is stripped; `#` headings are kept for chapters.
- **Feed posts:** the post's full content when the feed carries it. When it carries only a summary, Zenpod fetches the post's link and runs Readability. If that still yields only a teaser (under 1,500 characters and the page asks to subscribe or sign in), the episode is read anyway and its Show notes say "Only the preview was available".

## 6 · Settings: Voice

The first section in Settings, above Notes folder.

```
Voice           This Mac   A service

This Mac:
  Voice         [ Samantha              v ]
  Free, and works without a connection. Better voices appear here once you
  download them in System Settings › Accessibility › Spoken Content.

A service:
  Service       [ OpenAI                v ]
  Address       [ http://localhost:8880/v1 ]    (OpenAI-compatible only)
  API key       [ ••••••••••••          ]
                Saved in your Keychain
  Model         [ gpt-4o-mini-tts       ]
  Voice         [ marin                 ]

Play a sample
Every show reads with this voice unless you give it its own.
```

- **Service list:** OpenAI, Google Gemini, xAI Grok, MiniMax, Inworld, ElevenLabs, OpenAI-compatible.
- **Switching service** fills Model and Voice with that service's defaults unless you've typed your own. Each service remembers its own key, model and voice.
- **Model** is disabled for Grok, with the placeholder "xAI Grok has no model choice".
- **API key** is optional for OpenAI-compatible; its placeholder says "Only if your server asks for one".
- **This Mac's voices** come from `say -v '?'`, English first.
- **Play a sample** reads "This is how Zenpod will read to you." with the current settings. Errors appear beside it in plain words (section 10).

## 7 · Services

Each service is one small adapter: build the request, send it, return audio bytes. Zenpod splits text into pieces of at most **4,000 characters** (or the service's limit if lower), so progress moves steadily and a failure costs one piece, not the whole article.

| Service | Request | Key goes in | Default model · voice | Limit per request | Audio back |
| --- | --- | --- | --- | --- | --- |
| OpenAI | `POST api.openai.com/v1/audio/speech` `{model, input, voice, response_format:"mp3"}` | `Authorization: Bearer` | `gpt-4o-mini-tts` · `marin` | 4,096 | MP3 bytes |
| Google Gemini | `POST generativelanguage.googleapis.com/v1beta/interactions` with `speech_config:[{voice}]`, audio `audio/l16` 24 kHz | `x-goog-api-key` | `gemini-3.8-flash-tts` · `Kore` | 4,000 (Zenpod's cap) | WAV, base64 at `steps[].content[].data` |
| xAI Grok | `POST api.x.ai/v1/tts` `{text, voice_id, language:"auto"}` | `Authorization: Bearer` | none · `eve` | 60,000 | MP3 bytes |
| MiniMax | `POST api.minimax.io/v1/t2a_v2` `{model, text, voice_setting:{voice_id}, audio_setting:{format:"mp3", sample_rate:32000}}` | `Authorization: Bearer` | `speech-2.8-hd` · `English_expressive_narrator` | 10,000 | MP3, hex at `data.audio`; `base_resp.status_code` 0 is success |
| Inworld | `POST api.inworld.ai/tts/v1/voice` `{text, voiceId, modelId, audioConfig:{audioEncoding:"MP3"}}` | `Authorization: Basic` (the key as given) | `inworld-tts-2` · `Ashley` | 2,000 | MP3, base64 at `audioContent` |
| ElevenLabs | `POST api.elevenlabs.io/v1/text-to-speech/{voice}?output_format=mp3_44100_128` `{text, model_id}` | `xi-api-key` | `eleven_multilingual_v2` · `21m00Tcm4TlvDq8ikWAM` | 10,000 | MP3 bytes |
| OpenAI-compatible | `POST {address}/audio/speech`, same body as OpenAI | `Authorization: Bearer`, if a key is set | `kokoro` · `af_heart` | 4,000 (Zenpod's cap) | MP3 bytes |
| This Mac | `say -v {voice} -f {text file} -o {piece}.m4a --data-format=aac` (checked on this Mac) | none | the system voice · none | none | AAC file |

- **Defaults to confirm at build:** the Inworld and ElevenLabs default voices, and the Gemini model name. The first sample against each service settles them; if a default fails, change it here and in code together.
- **Gemini's free tier** depends on Google's current terms for the chosen model. Zenpod treats Gemini like any other keyed service.

## 8 · Making the audio

1. **Queue.** Reading runs one episode at a time, in the order asked for. Waiting episodes say "Next to read" with **· Stop** on hover.
2. **Split** at headings first, then paragraphs, then sentences, so each piece fits the limit. A piece never cuts a sentence.
3. **Send** pieces in order. Each finished piece is saved to `read/{read id}/pieces/` before the next starts. The row says "Reading 3 of 12" over a 1 px accent progress line.
4. **Retry** a failed piece once after 2 seconds. If it fails again, that episode stops with its error and **Try again**, which carries on from the failed piece. The queue moves on to the next episode.
5. **Stop** cancels an episode in progress or waiting. It goes back to "Ready to read"; finished pieces are kept for when you press Read again.
6. **Join.** Pieces are decoded to PCM with symphonia (already used for shapes), joined with 350 ms of silence, written as WAV and converted with `afconvert -f m4af -d aac` to `read/{read id}.m4a`. The pieces folder is deleted.
7. **Finish.** The episode row is created with its duration and shape (the existing peaks code). It appears in New and its show with "New", and the icon's dot comes on.

### Transcript and chapters

- **Transcript:** each piece's exact decoded length is shared among its sentences by character count and written as WebVTT. The Transcript tab and quotes in Notes work as for any episode.
- **Chapters:** each heading becomes a chapter at the start of its piece. A text with no headings has none.
- Both are written to the cache files `kept_text` already reads first (`transcripts/{episode id}.txt`, `chapters/{episode id}.txt`).

## 9 · Feed and folder shows

- **When they're checked:** whenever Zenpod refreshes podcasts (at launch and on the existing schedule), with the same bounded concurrency.
- **Feeds:** items are matched by GUID, or by link when there's no GUID. Each new one becomes a waiting read, or a queued one if the show reads automatically.
- **Folders:** new files of a readable type (section 4) become reads. Files are matched by path and modification time. Zenpod watches nothing in between refreshes. Files already there when the show was made, beyond the 3 newest, are ignored. A file that changes after it was read is not re-read.
- **Removed sources:** a feed that stops answering keeps its show and episodes. Its page shows "Couldn't reach this feed last time". A missing folder says "This folder isn't there any more".

## 10 · Data

### Shows

`shows` gains four columns. Podcasts leave them null.

| Column | Use |
| --- | --- |
| `kind text` | `null` for podcasts; `items`, `feed` or `folder` for yours |
| `source text` | Feed address or folder path |
| `auto integer` | 1 when new posts or files are read automatically |
| `voice text` | The show's own voice, or null for the default |

- **Feed addresses** of your shows are kept unique as `zenpod:show:{uuid}`, so `store::feeds()` skips them with `where kind is null and spotify_only = 0`. Read to me is `zenpod:show:read-to-me`, created on first use.

### Reads

A new table holds everything on its way to becoming an episode, so podcast episodes keep their current shape and code.

```sql
create table if not exists reads(
  id integer primary key,
  show_id integer not null references shows(id) on delete cascade,
  episode_id integer references episodes(id) on delete set null,
  title text not null, source text not null, source_key text,
  chars integer not null, state text not null,          -- waiting, queued, reading, failed, done
  pieces integer not null default 0, done integer not null default 0,
  error text, created integer not null default (unixepoch()));
create index if not exists reads_show on reads(show_id, created);
create unique index if not exists reads_source on reads(show_id, source_key);
```

- **Text** for each read is stored at `read/{id}.txt` until its episode is made, then deleted.
- **When done:** an `episodes` row is inserted with `local_path` set to the m4a, `kept = 1`, `guid = 'read:{id}'`, and `description` holding the source and first paragraph. `reads.episode_id` points to it.
- **Deleting** an episode of yours deletes its m4a, transcript and chapters files and its read. Its Notes go with it, as for any episode. Removing a show deletes all of these for every episode.

### Settings and Keychain

- **Settings table:** `voice_from`, `voice_service`, `voice_mac`, `voice_compat_address`, and per service `voice_{service}_model` and `voice_{service}_voice`.
- **Keychain:** the `keyring` crate, service `me.adamking.listener`, account `voice-{service}`.

## 11 · What you see when something goes wrong

One line: what happened, and what to do.

| Cause | Message |
| --- | --- |
| No key set | "{Service} needs a key. Paste it in Settings › Voice." |
| 401 or 403 | "{Service} didn't accept the key. Check it in Settings › Voice." |
| 404, or the service names the model | "{Service} doesn't know the model "{model}"." |
| The service names the voice | "{Service} doesn't know the voice "{voice}"." |
| 429 or out of credit | "{Service} says you've hit your limit. Try again later or check your account." |
| No connection, or local server not running | "Couldn't reach {address}. Is it running?" |
| Page with no readable text | "There's no article text on that page." |
| Scanned PDF | "This PDF has no text in it, only images of pages." |
| Anything else | The service's own message, shortened to one line. |

"Settings › Voice" never breaks across a line.

## 12 · Build

**Borrowed:** `@mozilla/readability` (page text), `pdfjs-dist` (PDF text), the `keyring` crate (Keychain). macOS `say`, `textutil` and `afconvert` are built in. Feed parsing reuses Zenpod's existing feed code.

| File | Change |
| --- | --- |
| `src-tauri/src/voice.rs` (new) | The adapters, `sample()`, error mapping, Keychain |
| `src-tauri/src/read.rs` (new) | The queue, splitting, pieces, retry and resume, join and `afconvert`, WebVTT and chapters, feed and folder checks |
| `src-tauri/src/store.rs` | `shows` columns, `reads` table and queries, `feeds()` filter, delete cleanup |
| `src-tauri/src/lib.rs` | Commands (`voice_settings`, `set_voice`, `set_voice_key`, `voice_sample`, `mac_voices`, `inspect`, `add_read`, `make_show`, `read_now`, `stop_read`, `retry_read`, `set_show`, `remove_show`, `reads`); `read` progress event; `zenpod://read` deep link; refresh calls the source check |
| `src-tauri/tauri.conf.json` | File associations for the readable types |
| `src/lib/extract.ts` (new) | Readability, pdf.js, Markdown stripping, title and headings |
| `src/lib/ReadToMe.svelte` (new) | The pane: field, check cards, Reading, Your shows |
| `src/lib/YourShow.svelte` (new) | A show's page |
| `src/lib/Library.svelte` | Two-row header, the icon and its dot, panes, your shows in Following and New, the drop state |
| `src/routes/+page.svelte` | Window-wide drop and ⌘V |
| `src/lib/Settings.svelte` | Voice section |
| `src/lib/icons.ts` | The Read to me glyph |
| `design/DESIGN.md`, `README.md` | The header, Read to me, your shows, Voice |

**Time, Claude building:** about 4 days, in two milestones that each ship on their own.

| Milestone | What | Estimate |
| --- | --- | --- |
| 1 | Two-row header, icon and pane, Voice settings and all adapters, one-off items into Read to me or a new show, every way in, the queue, transcript and chapters, show pages, Following and New | 2½ days |
| 2 | Feed and folder shows: cards, Make it a show, Choose folder, checks on refresh, wait or automatic, preview-only notice, the icon's dot for new arrivals | 1½ days |

## 13 · Acceptance

1. The header has two rows at every window size. R opens Read to me with the field focused, and does nothing while typing in a field. Mini, Pill and Settings are where they were; the Read to me icon is to their left; the tabs sit below.
2. With **This Mac**, pasted text becomes a playable episode with no network.
3. With a running local Kokoro server and **OpenAI-compatible**, a link becomes an episode.
4. **Play a sample** works for every service you have a key for, and each error in section 11 appears for its cause.
5. A 20,000-character article read through OpenAI becomes one episode whose length matches the sum of its pieces within 1 second.
6. The transcript follows the voice: at five random points, the highlighted sentence is the one being read, give or take one.
7. Headings in the source appear as chapters at the right places.
8. Three items added at once read one at a time, in order. Stop and Try again behave as in section 8, and Try again never re-sends a finished piece.
9. Pasting a podcast feed offers Follow it; a blog feed offers Make it a show; a folder dropped on the window offers Make it a show.
10. A feed show set to wait shows new posts as "Ready to read" after a refresh; set to automatic, it reads them. Nothing is sent to a service for a waiting post.
11. Your shows appear in Following in sort order with "Your show", and their finished episodes in New.
12. A show's own voice is used for its new episodes; others use the default.
13. Notes, the shape, the Mini and the Pill work on your episodes as on any other.
14. Podcast refresh never treats your shows as podcast feeds. Removing a show deletes its files.
15. Keys appear in Keychain Access under `me.adamking.listener` and nowhere in the database or app data folder.
16. Day and Night, all fourteen accents, and reduced motion all work in the header, the pane, show pages, cards and Voice settings.
17. No visible text says "read aloud", "TTS" or "text to speech".

## 14 · Not in this version

- Listening while an episode is still being made.
- Newsletters by email, saved-article services, and a Safari share extension (the `zenpod://read` link covers sending from other apps).
- Summaries, rewrites or anything else that changes the text before it's read.
- Two voices, dialogue, or a conversation made from an article.
- Voice cloning.
- Downloading or running voice models inside Zenpod.
- Prices and usage tracking.
- Re-reading a file or post that changed after it was read.
- Sharing your shows as a public feed.

# Zenpod: Notes

Spec · 9 October 2026 · approved for build

Notes let you keep the part of an episode you want to come back to. Press M and Zenpod keeps the last 30 seconds as a note. If you want, write your thoughts right there beside it, without leaving the episode.

- **Design source:** the "Marks" beat in the demo film (`mockups/demo-video/copy/film-stills/08-mark.jpg`), built on in the prototype.
- **Prototype:** `mockups/marks/index.html`. It's the reference for behaviour, type and motion. Where this spec and the prototype disagree, this spec wins.
- **Background:** the two proposals in this folder, `FUTURE-ENHANCEMENTS-CLAUDE.md` and `FUTURE-ENHANCEMENTS-KEEL.md`. They call the feature Marks.

## 0 · Naming

The feature is called **Notes** everywhere the listener can see it. "Mark" means nothing to someone using a podcast player.

- **The show's notes from the feed** are renamed **Show notes**, the standard podcast term, which the app already uses in the episode title's tooltip. The episode pane reads **Show notes · Transcript · Notes**.
- **In code**, the feature keeps the name `marks` (table, commands, functions). `episode_notes` already means the show's notes, and using "notes" for both would cause bugs. Nothing in the interface says "mark".

## 1 · Decisions

| Question | Decision |
| --- | --- |
| Which key | **M**, inside Zenpod only. There's no global shortcut. |
| What a note keeps | The 30 seconds that end at the playhead. The length is fixed and you can't drag the ends. |
| Two ways to take a note | M takes it and keeps playing. The note button pauses first, then opens the field. |
| While you type | Playback pauses on the first key and resumes when you finish, but only if Zenpod paused it. |
| Where you write | On the main screen, at the note on the shape. Never in the side pane. |
| Enter and Esc | Both save what you typed. Esc on an empty field closes it and keeps the note with no words. |
| A note you don't write in | The field fades after 5 seconds. The note stays. |
| Where you review notes | In two places: the episode pane (Show notes · Transcript · Notes) and a Notes tab in the library. |
| Notes folder | A simple choice in Settings. Without one, notes stay in Zenpod. |
| Mini and Pill | M takes a note there without words. You add words later from the main window. |
| The shape | It never moves, shrinks or fades as a whole to make room for writing. |

## 2 · How it works

### Taking a note

- **M** (in any Zenpod window, unless you're typing in a field) takes a note from `max(0, playhead − 30 s)` to the playhead.
- **The note button** in the transport does the same, then pauses and opens the field. If you click it while paused, nothing extra pauses.
- The note appears on the shape immediately. It's saved as soon as it's taken, before you write anything.

### Writing, at the note

The plate opens beside the note on the shape. It has four states:

| State | What you see | What it does |
| --- | --- | --- |
| Just taken (by M) | Label, quote, an empty field reading "Add a note", and "Undo" | The field has focus. Playback continues. Space and the arrow keys still control playback while the field is empty. Untouched for 5 s, it fades. |
| Typing | Your words as you type them. "↵" appears once there's text | Playback pauses on the first key. Long notes wrap to three lines, then scroll inside the field. |
| Saved | Your words, with "Saved to your notes" or "Saved in Zenpod" rising in beneath them | Playback resumes if Zenpod paused it. Fades after 1.6 s. |
| Undo | | Removes the note, closes the plate, and resumes if Zenpod paused. |

- **Enter** or **Esc** saves. **Esc** on an empty field closes the plate and keeps the note with no words.
- Clicking away from typed words saves them.
- Pressing M again while a plate is open saves what's there first, then takes the new note.

### Hover and click

- **Hovering over a note** on the shape shows a read-only plate: the label, then your words (or the quote if there are no words). It has no field and no Undo.
- **Clicking a note** plays from its start.
- **Clicking anywhere else on the shape** still moves the playhead, as it does today.

### Mini and Pill

- M works there (all windows share one key handler). The note shows on their Orbit. No plate opens.

## 3 · How it looks

### The note on the shape

It looks like a chapter dot you made yourself, drawn in the accent colour because it's something you touched (`DESIGN.md`, "Colour means something").

- **Span:** a 2.2px accent line along the centre line, from the start to the end of the note, at 85% opacity. On the Orbit it's an arc on the chapter ring (radius `0.165 × side`).
- **Ring:** at the end of the span. 6.5px radius, filled with `--base`, with a 1.2px accent stroke and a 2.6px accent dot in the centre.
- **Lit:** while its plate is open, the span is 2.6px at full opacity.
- **Order:** notes are drawn after the shape and the chapters, so nothing covers them.

### The plate

The film's plate: no box, no background, no shadow. Just a leader line and words.

| Part | Style |
| --- | --- |
| Leader | 1px accent line at 80% opacity |
| Label | `NOTE · 4:42–5:12` in Fragment Mono, 10.5px, 0.1em tracking, `--text-faint` |
| Quote | Only when the episode has a transcript. 15px/1.45, `--text-mid`, curly quotes, at most two lines |
| Field | 15px/1.45 in `--text`, accent caret. Placeholder "Add a note" in `--text-faint`. A 1px accent line underneath. "↵" in mono 12px, `--text-faint` |
| Under the field | "Undo" (12.5px, `--text-faint`, turns `--red` on hover). After saving: "Saved to your notes" or "Saved in Zenpod" (12.5px, `--text-dim`) |
| Width | Up to 380px. It narrows to 200px before it would leave the listening side |

### Placement

Field and Thread:

1. **Horizontal.** The leader rises straight up from the ring (starting 8px above its centre) to the label's centre line, then turns 14px toward the plate. The plate starts 8px after the turn. It opens to the right unless the left has more room. It's always clamped inside the listening side.
2. **Vertical.** The plate is centred in the open space between the title block and the line: `top = 12 + max(0, (lineY − 22 − 12 − plateHeight) / 2)`, measured in the stage. It's never closer than 12px to the title block or 22px to the line.
3. **Clearing.** The bars directly behind the plate fade so the words read: a soft cut-out 10px wider than the plate on each side, 94% strength, 8px blur, fading in and out with the plate. The centre line, chapters and notes are drawn after the cut-out and stay untouched. Nothing else about the shape changes.

Orbit:

1. The plate sits beside the ring, on the note's side, 12px outside the rays (at `centre ± (0.49 × side + 12)`).
2. The label's centre line is level with the note where possible, and kept inside the stage.
3. The leader runs flat from the ring (8px off the note) to the plate's edge, with one vertical step if the plate had to move.
4. The label wraps rather than cut off its words, and the quote gets two lines.

### Motion

The house defaults: GSAP, power3. Reduced motion makes every step instant.

| Moment | Motion |
| --- | --- |
| Note appears | The span sweeps back from the playhead and the ring grows: 300ms, ease-out cubic |
| Plate opens | The leader draws out of the ring (stroke-dashoffset 1 → 0, 450ms, power3.inOut). The plate fades in over 300ms, starting at 350ms |
| Hover plate | The leader takes 300ms; the plate starts at 150ms |
| Saved | The saved line rises 4px and fades in over 350ms |
| Plate closes | The plate fades over 250ms and the leader retracts over 350ms |

## 4 · Reviewing notes

### The episode pane

The reading words become **Show notes · Transcript · Notes**, with a mono count after Notes when there are any. In the Notes list, each note shows:

- its time (`4:42–5:12`, mono) and its chapter (faint);
- the quote, up to three lines, in `--text-mid`;
- your words in `--text`, or the words "Add a note", which open the field in place;
- on hover: **Play · Edit · Remove**. Remove follows the app's pattern: "Press again to remove", in red.

The note the playhead is inside takes the accent on its time.

Below the list it says either "Written to ~/…/file.md" or "Kept in Zenpod. Choose a notes folder in Settings to write these out as Markdown."

With no notes yet, it reads: "No notes in this episode yet. Press M while it plays to keep the last 30 seconds, or use the note button to stop and write."

### The library's Notes tab

Notes sits beside New and Following. It lists every note, newest first, grouped by day the same way New is. Each row shows:

- the cover;
- as its title, your words, or else the quote's first sentence, or else "Note at 4:42";
- `Show · 4:42 · Chapter` underneath.

Choosing a row opens that episode's Notes and plays from the note.

When there are no notes: "No notes yet. Press M while you listen to keep the last 30 seconds."

## 5 · Notes folder and Markdown

### Settings

A **Notes folder** section, in the existing section style:

- **Not set:** "Notes stay in Zenpod. Choose a folder and each episode's notes are also written there as a Markdown file." The word **Choose a folder…** opens the native folder picker.
- **Set:** the path in mono, then "Each episode you take notes in gets one Markdown file here, rewritten whenever its notes change." Then the words **Change** and **Stop writing** (Stop writing turns red on hover).
- **After choosing:** a quiet note, for example "2 episode files written." Every episode that already has notes is written at once.
- **After stopping:** "Notes stay in Zenpod. The files already written are left where they are."

### The file

- **One file per episode**, named `<show>-<episode>.md`. The name is lowercase ASCII with hyphens.
- **Clashes:** if two episodes would get the same name, the second has its episode id added.
- **When it's written:** in full each time any of the episode's notes is taken, edited or removed. It's written to a temporary file and renamed, so a reader never sees half a file.
- **If the episode loses all its notes,** the file stays and reads "No notes yet."

```markdown
---
title: "We're Not Late (Or, Rethinking the Long-Term)"
show: What Works
published: 2025-06-04
---

# We're Not Late (Or, Rethinking the Long-Term)

What Works · 4 Jun

## 4:42–5:12 · Time Flies

> The quote, when the episode has a transcript.

Temporal bandwidth, for the workshop
```

## 6 · Data

The table keeps the internal name `marks` (see section 0). It's created in `store::open` like the others:

```sql
create table if not exists marks(
  id integer primary key,
  episode_id integer not null references episodes(id) on delete cascade,
  start real not null,
  end real not null,
  note text not null default '',
  quote text,
  created integer not null default (unixepoch()));
create index if not exists marks_episode on marks(episode_id, start);
```

- **The quote** is saved when the note is taken, from the feed's transcript (up to the last three cues inside the span). It's saved rather than worked out later, because feeds and their transcripts change.
- **The notes folder** is the `notes_folder` key in the existing `settings` table.

New Tauri commands:

| Command | Returns |
| --- | --- |
| `add_mark(episode_id, start, end, quote)` | The new note |
| `set_mark_note(id, note)` | Nothing |
| `remove_mark(id)` | Nothing |
| `marks(episode_id)` | That episode's notes, by start |
| `all_marks()` | Every note with its episode and show, newest first |
| `set_notes_folder(path or null)` | How many files were written |

The add, set and remove commands each rewrite that episode's file when a folder is set. They emit a `marks` event, so every window can update.

## 7 · Build

**Borrowed:** the official `tauri-plugin-dialog`, for the folder picker. Add `dialog:allow-open` to the capability. Everything else is built in Zenpod's own code.

| File | Change |
| --- | --- |
| `src-tauri/src/store.rs` | `marks` table and queries |
| `src-tauri/src/marks.rs` (new) | Markdown writer, file names, atomic write |
| `src-tauri/src/lib.rs` | Commands, the dialog plugin, the `marks` event |
| `src/lib/now.svelte.ts` | M in `keys()`; the current episode's notes |
| `src/lib/signal.js` | `drawMarks` (span, ring, dot) for line and ring |
| `src/lib/Signal.svelte` | Draw notes after chapters, the cut-out behind the plate, hit-testing, hover and click |
| `src/lib/Plate.svelte` (new) | Plate, leader, states, placement, motion |
| `src/lib/Listen.svelte` | Note button in the transport; holds the plate in the stage |
| `src/lib/Library.svelte`, `ui.svelte.ts` | "Show notes" rename; Notes in the reading words; the Notes tab |
| `src/lib/Settings.svelte` | Notes folder section |
| `design/DESIGN.md`, `README.md` | "Show notes" wording; add the note, the plate and the Notes list |

**Time, Claude building:** about 2 days.

| Part | Estimate |
| --- | --- |
| Data and commands | 2 h |
| Shape drawing and plate | 5 h |
| Lists, tab and the rename | 3 h |
| Settings and Markdown | 2 h |
| Checks | 2 h |

## 8 · Acceptance

1. M takes a note in the main window, the Mini and the Pill. It does nothing while typing in a field.
2. The note button pauses, takes a note and puts focus in the field.
3. Typing pauses playback. Enter and Esc save. Playback resumes only if Zenpod paused it.
4. An untouched plate fades after 5 s and leaves a note with no words. Undo removes the note.
5. **Layout check:** port `auditCaption()` from the prototype as a check script. It covers Field, Thread and Orbit, an episode with and without a transcript, 21 positions, and empty words, long words and a hover. That's 378 cases, all passing, at the default window size and at the smallest window the app allows. It must also fail when the plate is deliberately moved into the shape or the title.
6. The shape's drawing is identical with a plate open and closed, apart from the cut-out behind the plate.
7. Notes survive a restart. Deleting an episode deletes its notes.
8. With a folder set, every change rewrites that episode's file. With no folder, nothing is written.
9. The episode pane and the Notes tab show the same notes, and the Markdown matches them.
10. No visible text says "mark" anywhere. The show's own notes say "Show notes".
11. Day and Night, all fourteen accents, and reduced motion all work.

## 9 · Not in this version

These were considered and left out. Each is a separate decision for later.

- Dragging a note's ends to change its length.
- Clickable times in the Markdown that open Zenpod at that moment (they need the deep-link plugin).
- A writing field in the Mini or Pill.
- Voice notes, saving the audio itself, and a global shortcut that works from other apps.
- Transcripts made by Zenpod. Quotes come only from the feed's own transcript.

# Zenpod: future enhancements

Keel · 27 September 2026

An exploration of what Zenpod could become, written at Adam's request. These are proposals, not an approved build plan. The current product and design contracts remain in force until a specific change is accepted.

## The direction

A private listening companion that helps you keep the ideas that matter.

Zenpod already has a distinctive foundation: the shape of an episode, quiet playback, artwork light, and floating surfaces that stay out of the way. More capability should preserve that experience. A person who only wants to listen should still open the app and press play.

The opportunity is to connect listening with remembering: catch a passage, attach the thought it prompted, and find both again when they become useful.

## Ownership and showing the experience

Adam's response to this exploration adds two important constraints: avoid another compulsory subscription, and show the app actually being used.

- Keep the core experience local and account-free. Saved moments, annotations, and exports should remain usable without a hosted service.
- If Zenpod becomes a commercial product, investigate a one-time purchase and optional paid major upgrades. This is a proposed model, not a pricing decision.
- Prefer on-device processing where its quality and performance are sufficient. Model downloads, processing time, storage, and battery use are real costs that need testing.
- Any optional cloud processing should disclose what leaves the device and what it costs. It must not become a hidden dependency of ordinary playback or access to saved work.
- Demonstrate a complete interaction before asking someone to buy: hear a passage, save it, add a thought, close the app, then find and replay it later. Use real app recordings and a usable trial if the app is distributed.

Adam's dislike of subscription accumulation is product direction for this exploration, not evidence that every potential customer shares it. Broader demand remains untested.

## 1. Keep that thought

**The moment:** something clicks while you are listening, but stopping to take notes would interrupt it.

A global shortcut saves a short passage ending at the current position. A small mark appears on the episode shape, and playback continues. The saved moment retains its episode, timestamp, source reference, and transcript where available. Its boundaries can be adjusted afterward.

A deliberate second action records a spoken annotation: “This connects to what we discussed about trust yesterday.” The listener's thought stays separate from the speaker's words, but attached to the same moment.

**Smallest useful version:** timestamp bookmarks, optional typed notes, replay, and Markdown export. No transcription or AI is required to prove whether saving moments is useful.

**Further possibility:** retain local excerpts when appropriate, add voice annotations, and suggest sentence boundaries when a transcript exists. A timestamp alone cannot guarantee future replay if the original source disappears; the interface should distinguish saved references from retained audio.

## 2. Find it by what you remember

**The moment:** “Who was talking about why people stay in failing organisations?”

Search across available transcripts and personal notes, returning a few playable passages with episode, date, speaker when known, and surrounding context. Start with ordinary text search. Explore semantic search only if it solves retrieval failures that matter in use.

The ambitious extension is comparison: “Where do these two guests disagree?” Let the listener hear the relevant arguments beside one another. Label generated interpretation and link it to the passages that support it.

**Dependency:** transcript coverage. Publisher transcripts can be used when available; optional local transcription would need its own quality and performance trial. Generated transcripts must be labelled and correctable. Missing coverage must be visible in search results.

## 3. Talk back to the episode

**The moment:** pause an interview and ask, “Explain that term, then replay the part where she used it.”

A short answer appears in the existing reading pane, with playable references. Questions about the episode stay grounded in its content. General explanation is distinguished from what the speaker said; external research is a separate action.

The simpler version may be more valuable first: record your own response and resume. An episode can hold the conversation and your evolving thoughts around it without needing an AI conversation at all.

**Decision to earn:** whether contextual answers save enough effort to justify their latency, processing requirements, and possible errors. An unsupported answer must not be presented as an episode fact.

## 4. Listening that fits the time you have

**The moment:** “I have twenty minutes and want something absorbing.”

Build a bounded session from the person's own library: a short episode, a complete chapter, or the next section of something already started. Account for playback speed and stop at a sensible boundary. Where chapters are unavailable, offer an explicit timer rather than promise a natural stopping point.

Optional comfort controls could provide steadier volume, slower playback, reduced visual movement, and a short replay when returning after a long pause. The person chooses the experience; the app does not infer a diagnosis or emotional state.

**Smallest useful version:** duration filters, an explicit short queue, and “stop after this episode or chapter.” More ambitious session assembly would be a deliberate extension of the current no-recommendations design principle.

## 5. Bring more listening into the room

**The moment:** a recorded talk or a voice memo deserves the same thoughtful player as a podcast.

Start with imported local audio files. Give them position memory, the familiar episode shape, and saved moments. Later, investigate audiobook chapter support and articles read aloud, with navigation back to the original text.

This opens up several uses:

- Study a lecture with transcript navigation and repeatable passages.
- Hear an essay while cooking, then return to the paragraph on the Mac.
- Revisit voice notes from a walk and retain the useful ideas.
- Prepare for a conversation by listening to a selected set of talks.

**Scope boundary:** local audio is the first experiment. Web extraction, text-to-speech, and protected media each introduce separate product and technical questions. Supporting “anything” is not a useful initial requirement.

## 6. Mixtapes of ideas

**The moment:** “Five conversations that changed how I think about trust.”

Arrange saved passages from several episodes into a listening sequence, optionally adding personal spoken introductions. The existing Thread could show the sequence, revealing each source as playback reaches it.

A mixtape could be a study collection, preparation for a discussion, or a personal retrospective. Later, a deliberately shared collection could become a thoughtful gift.

**Smallest useful version:** a private sequence of source references and timestamps. Sharing and copying audio are separate capabilities; distributing excerpts requires the appropriate rights. This proposal does not introduce a public feed, follower system, or automatic publishing.

## 7. Take it on a walk

**The moment:** leave the Mac, continue on the phone, capture an idea, and find it waiting when you return.

A focused iPhone companion could carry a chosen listening session, save moments, and return playback position and annotations to the desktop. A Watch interaction could follow if it makes capture easier.

**Scope boundary:** this is a substantial expansion from today's local-only Mac app. Optional private sync needs an explicit product decision and investigation of offline behavior, conflicting positions, and recovery. The desktop should remain independently useful.

## The first bet

Build saved moments with optional notes, replay, and export. Try them during ordinary listening before adding an AI layer.

The hypothesis is that returning to a personally meaningful passage is more valuable than accumulating episode summaries. Confidence is moderate as a design judgment and low as a claim about wider demand: we have not tested it.

The competing hypothesis is that Zenpod's greatest value is uninterrupted playback, and capture features would mostly add unused controls.

**Decision threshold:** if Adam repeatedly saves and revisits moments across real listening sessions, extend capture into voice annotations and retrieval. If saved moments go unused or feel intrusive, prioritise listening comfort and continuity. If capture is useful but retrieval fails, improve search before adding conversational answers.

The larger concepts remain available without becoming simultaneous commitments. No delivery estimates are assigned before technical investigation and a bounded feature choice.

## Context and provenance

These ideas were first proposed in conversation before Keel read Claude's `NEXT.md` in the parent workspace. That document was read afterward for comparison. The overlap is around search, transcription, and mobile continuity; this document records Keel's separate exploration rather than replacing Claude's operational next steps.

Grounding came from the current source code, README, design and brand documents, earlier scope and handoff documents, and saved app screenshots. This was concept exploration, not a live product verification or user-research study.

[Snipd's official site](https://www.snipd.com/) was consulted on 27 September 2026. It already describes highlights, transcript search, podcast chat, and note export. Those capabilities are precedents, not claims of invention or unique differentiation. Zenpod's proposed emphasis is quiet interaction, personal ownership, and keeping the listener's own thoughts connected to the audio.

Useful local references: [README](../README.md), [design system](../design/DESIGN.md), and [brand guide](../design/BRAND.md).

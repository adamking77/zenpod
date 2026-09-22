export type Cue = { start: number; text: string };

const clock = (s: string) => s.replace(',', '.').split(':').reduce((a, p) => a * 60 + parseFloat(p), 0);

/** WebVTT, SRT, Podcast Namespace JSON or plain text, as timed cues (plain text has no times: start is -1). */
export function parseTranscript(raw: string): Cue[] {
  const text = raw.trim();
  if (text.startsWith('{')) {
    try {
      const j = JSON.parse(text);
      return (j.segments ?? []).map((s: { startTime: number; body: string }) => ({ start: s.startTime, text: s.body }));
    } catch { /* fall through to text */ }
  }
  const timed = /^((?:\d+:)?\d{1,2}:\d{2}[.,]\d{1,3})\s*-->/;
  const cues: Cue[] = [];
  for (const block of text.split(/\r?\n\s*\r?\n/)) {
    const lines = block.split(/\r?\n/);
    const at = lines.findIndex((l) => timed.test(l));
    if (at < 0) continue;
    const words = lines.slice(at + 1).join(' ').replace(/<[^>]+>/g, '').trim();
    if (words) cues.push({ start: clock(timed.exec(lines[at])![1]), text: words });
  }
  if (cues.length) return cues;
  const plain = new DOMParser().parseFromString(text, 'text/html').body.textContent ?? '';
  return plain.split(/\n\s*\n/).map((t) => ({ start: -1, text: t.trim() })).filter((c) => c.text);
}

/** Cues gathered into paragraphs at the pauses, so the transcript reads like prose. */
export function paragraphs(cues: Cue[]): Cue[][] {
  const out: Cue[][] = [];
  cues.forEach((c, i) => {
    const prev = cues[i - 1];
    const pause = !prev || c.start < 0 || (/[.?!]["”']?$/.test(prev.text) && (out.at(-1)?.length ?? 0) >= 6);
    if (pause) out.push([]);
    out.at(-1)!.push(c);
  });
  return out;
}

// Run: bun src/lib/transcript.check.ts
import { parseTranscript } from './transcript';
const vtt = `WEBVTT\n\n00:00:04.679 --> 00:00:07.639\nYou're listening to a stage talk\n\n00:01:07.639 --> 00:01:10.439\n<v Peter>Missing Records.</v>\n`;
const c = parseTranscript(vtt);
if (c.length !== 2 || Math.abs(c[0].start - 4.679) > 1e-6 || c[1].start !== 67.639 || c[1].text !== 'Missing Records.') throw new Error(JSON.stringify(c));
const srt = `1\n00:00:01,500 --> 00:00:02,000\nHello\n\n2\n00:00:03,000 --> 00:00:04,000\nThere`;
const s = parseTranscript(srt);
if (s.length !== 2 || s[0].start !== 1.5) throw new Error(JSON.stringify(s));
const j = parseTranscript('{"version":"1.0.0","segments":[{"startTime":2.5,"body":"Hi"}]}');
if (j[0].start !== 2.5 || j[0].text !== 'Hi') throw new Error(JSON.stringify(j));
console.log('transcript parsing ok');

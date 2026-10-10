import { Readability } from '@mozilla/readability';

/** Text as the reader takes it: paragraphs separated by blank lines, "# " before a heading. */
export type Doc = { title: string; text: string };

const squash = (s: string | null | undefined) => (s ?? '').replace(/\s+/g, ' ').trim();

/** The article on a page, by Mozilla's Readability; null when the page has no article to speak of. */
export function fromHtml(html: string, url: string): Doc | null {
  const doc = new DOMParser().parseFromString(html, 'text/html');
  const base = doc.createElement('base');
  base.href = url;
  doc.head.prepend(base);
  const art = new Readability(doc, { charThreshold: 400 }).parse();
  if (!art?.content) return null;
  const body = new DOMParser().parseFromString(art.content, 'text/html').body;
  const out: string[] = [];
  const SKIP = /^(PRE|CODE|TABLE|FIGURE|SCRIPT|STYLE|NOSCRIPT|SVG|BUTTON|NAV|ASIDE|FORM)$/;
  const BLOCK = /^(P|LI|BLOCKQUOTE|DD|DT|H[1-6])$/;
  const walk = (el: Element) => {
    for (const c of el.children) {
      if (SKIP.test(c.tagName)) continue;
      if (/^H[1-4]$/.test(c.tagName)) { const h = squash(c.textContent); if (h) out.push(`# ${h}`); continue; }
      if (BLOCK.test(c.tagName) || !c.querySelector('p, li, h1, h2, h3, h4, blockquote, div')) {
        const t = squash(c.textContent);
        if (t) out.push(t);
        continue;
      }
      walk(c);
    }
  };
  walk(body);
  const title = squash(art.title) || new URL(url).hostname;
  // The article's own title heading would be read twice.
  if (out[0]?.startsWith('# ') && squash(out[0].slice(2)).toLowerCase() === title.toLowerCase()) out.shift();
  const text = out.join('\n\n');
  return text.length < 200 ? null : { title, text };
}

/** Pasted text: its paragraphs as they are, the first line as the title. */
export function fromText(raw: string): Doc {
  const paras = raw.replace(/\r\n/g, '\n').split(/\n\s*\n/).map(squash).filter(Boolean);
  const first = squash(raw.split('\n').find((l) => l.trim()) ?? '').replace(/^#+\s*/, '');
  return { title: first.length > 90 ? `${first.slice(0, 88).trimEnd()}…` : first, text: paras.join('\n\n') };
}

export const isLink = (s: string) => /^https?:\/\/\S+$/i.test(s.trim());

export const host = (u: string) => { try { return new URL(u).hostname.replace(/^www\./, ''); } catch { return u; } };

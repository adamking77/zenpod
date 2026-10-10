<script lang="ts" module>
  /** Read to me's art: the Orbit drawn for small sizes. A ring of rays, the first third and its rays in the accent, and the head. */
  const MARK = (() => {
    const P = (r: number, d: number) => [r * Math.cos((d * Math.PI) / 180), r * Math.sin((d * Math.PI) / 180)];
    let rays = '';
    [7, 10, 6, 12, 9, 5, 11, 8, 6, 10, 7, 9, 5, 8, 11, 6, 9, 7].forEach((len, n) => {
      const d = -90 + n * 20, [x1, y1] = P(23, d), [x2, y2] = P(23 + len, d), lit = n < 6;
      rays += `<line x1="${x1.toFixed(1)}" y1="${y1.toFixed(1)}" x2="${x2.toFixed(1)}" y2="${y2.toFixed(1)}" stroke-width="3" stroke-linecap="round" style="stroke:${lit ? 'var(--accent)' : 'var(--text)'};opacity:${lit ? 0.9 : 0.28}"/>`;
    });
    const [hx, hy] = P(17, 30);
    return `<svg viewBox="-40 -40 80 80" aria-hidden="true">${rays}<circle r="17" fill="none" stroke="var(--text)" stroke-width="2.4" opacity="0.22"/><circle r="17" fill="none" stroke="var(--accent)" stroke-width="3.4" stroke-linecap="round" transform="rotate(-90)" pathLength="1" stroke-dasharray="0.333 1"/><circle cx="${hx.toFixed(1)}" cy="${hy.toFixed(1)}" r="4.2" fill="var(--accent)"/></svg>`;
  })();

  const hash = (s: string) => [...s].reduce((h, c) => (h * 31 + c.charCodeAt(0)) % 2147483646, 7) + 1;

  /** A short line of sound bars, the same for the same name, its first third in the accent. */
  function sound(name: string) {
    let s = hash(name), out = '';
    const rnd = () => (s = (s * 16807) % 2147483647) / 2147483647;
    for (let i = 0; i < 22; i++) {
      const h = 4 + rnd() * 16, x = 2 + i * 4.5;
      out += `<line x1="${x}" y1="${(12 - h / 2).toFixed(1)}" x2="${x}" y2="${(12 + h / 2).toFixed(1)}" stroke-width="1.6" stroke-linecap="round" style="stroke:${i < 7 ? 'var(--accent)' : 'var(--text)'};opacity:${i < 7 ? 0.9 : 0.3}"/>`;
    }
    return `<svg viewBox="0 0 100 24" preserveAspectRatio="none" aria-hidden="true">${out}</svg>`;
  }
</script>

<script lang="ts">
  // Artwork for anything in the library. A podcast shows its own image. A show you made has none, so it's drawn:
  // Read to me gets the Orbit; the rest their name over a line of sound. An episode you made is a page with its title.
  let { id, kind = null, name = '', page = null, src = null, size = null, round = false }: {
    id: number; kind?: string | null; name?: string; page?: string | null; src?: string | null; size?: number | null; round?: boolean;
  } = $props();
</script>

<span class="art" class:round class:drawn={kind != null} style:width={size ? `${size}px` : null}>
  {#if kind == null}
    <img src={src ?? `listener://localhost/art/${id}`} alt="" loading="lazy" />
  {:else if page != null}
    <span class="page"><b>{page}</b>{#each [92, 100, 84, 96, 70, 88] as w}<i style:width="{w}%"></i>{/each}</span>
  {:else if kind === 'read-to-me'}
    <span class="mark">{@html MARK}</span>
  {:else}
    <span class="yours"><b>{name}</b>{@html sound(name)}</span>
  {/if}
</span>

<style>
  .art { display: block; aspect-ratio: 1; border-radius: 6px; overflow: hidden; background: var(--hair); container-type: inline-size; }
  .art.round { border-radius: 50%; }
  .art.drawn { background: var(--surface); box-shadow: inset 0 0 0 1px var(--line); }
  img { display: block; width: 100%; height: 100%; object-fit: cover; }
  .mark { display: grid; place-items: center; width: 100%; height: 100%; }
  .mark :global(svg) { width: 84%; height: 84%; }
  .yours, .page { position: relative; display: block; width: 100%; height: 100%; }
  .yours b { position: absolute; left: 11%; right: 9%; top: 11%; font-weight: 300; font-size: 15cqi; line-height: 1.12; color: var(--text);
    overflow: hidden; display: -webkit-box; -webkit-line-clamp: 3; line-clamp: 3; -webkit-box-orient: vertical; overflow-wrap: anywhere; }
  .yours :global(svg) { position: absolute; left: 11%; bottom: 13%; width: 78%; height: 22%; }
  /* an episode you made: a page, its title, and lines of text */
  .page { padding: 15% 12%; box-sizing: border-box; display: grid; align-content: start; gap: 7%; }
  .page b { font-weight: 400; font-size: 9cqi; line-height: 1.2; color: var(--text); overflow: hidden; display: -webkit-box; -webkit-line-clamp: 3; line-clamp: 3; -webkit-box-orient: vertical; overflow-wrap: anywhere; }
  .page i { display: block; height: max(1px, 2.4cqi); border-radius: 2px; background: color-mix(in srgb, var(--text-muted) 38%, transparent); }
  .round .yours b, .round .page b { display: none; }
  .round .yours :global(svg) { left: 18%; width: 64%; bottom: 39%; }
</style>

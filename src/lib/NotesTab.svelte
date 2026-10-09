<script lang="ts">
  // Every note, newest first, grouped by day as New is. Choosing one opens its episode's notes and plays from it.
  import { onMount } from 'svelte';
  import { fmt } from '$lib/api';
  import { now, player } from '$lib/now.svelte';
  import { allMarks, changes, type Mark } from '$lib/marks.svelte';

  let { onopen }: { onopen: (episodeId: number) => void } = $props();
  let list = $state<Mark[]>([]);
  onMount(() => { allMarks().then((l) => (list = l)); });
  $effect(() => { void changes.n; allMarks().then((l) => (list = l)); });

  const DAY = 86400;
  function bucket(t: number) {
    const d = new Date(), start = new Date(d.getFullYear(), d.getMonth(), d.getDate()).getTime() / 1000;
    return t >= start ? 'Today' : t >= start - 6 * DAY ? 'This week' : 'Earlier';
  }
  const grouped = $derived(
    list.reduce<{ label: string; notes: Mark[] }[]>((g, m) => {
      const label = bucket(m.created);
      if (g.at(-1)?.label !== label) g.push({ label, notes: [] });
      g.at(-1)!.notes.push(m);
      return g;
    }, []),
  );
  const title = (m: Mark) => m.note || (m.quote ? `“${m.quote.split(/(?<=[.?!])\s/)[0]}”` : `Note at ${fmt(m.start)}`);

  async function open(m: Mark) {
    onopen(m.episode_id);
    if (now.episode?.id === m.episode_id) { await player.seek(m.start); if (!now.playing) player.toggle(); }
    else await player.choose(m.episode_id, undefined, m.start);
  }
</script>

{#each grouped as g (g.label)}
  <div class="day">{g.label}</div>
  {#each g.notes as m (m.id)}
    <button class="row" onclick={() => open(m)}>
      <span class="cover">{#if m.image_url}<img src={m.image_url} alt="" loading="lazy" />{/if}</span>
      <span class="txt"><span class="t">{title(m)}</span><span class="m sub">{m.show_title} · <span class="num">{fmt(m.start)}</span>{#if m.chapter} · {m.chapter}{/if}</span></span>
    </button>
  {/each}
{:else}
  <p class="quiet">No notes yet. Press M while you listen to keep the last 30 seconds.</p>
{/each}

<style>
  .day { font-size: 12px; color: var(--text-faint); margin: 22px 0 2px; }
  .day:first-child { margin-top: 4px; }
  .row { display: grid; grid-template-columns: 34px minmax(0, 1fr); gap: 14px; align-items: center; width: 100%; text-align: left; padding: 9px 0; }
  .cover { display: block; width: 34px; aspect-ratio: 1; border-radius: 6px; overflow: hidden; background: var(--hair); }
  .cover img { display: block; width: 100%; height: 100%; object-fit: cover; }
  .txt { min-width: 0; }
  .t { display: block; font-size: 14.5px; line-height: 1.35; color: var(--text); white-space: nowrap; overflow: hidden; text-overflow: ellipsis; transition: color var(--dur-base) var(--ease-hover); }
  .m { display: block; margin-top: 1px; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .row:hover .t { color: var(--accent); }
  .quiet { color: var(--text-dim); font-size: 13.5px; max-width: 32ch; margin-top: 8px; }
</style>

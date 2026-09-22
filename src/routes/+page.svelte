<script lang="ts">
  import { onMount } from 'svelte';
  import { savedLook, setLook, type Look } from '$lib/look';

  let tab = $state<'new' | 'following' | 'settings'>('new');
  let look = $state<Look>(savedLook());

  onMount(() => { setLook(look); });

  function pick(v: Look) { look = v; setLook(v); }
</script>

<main class="win">
  <div class="drag" data-tauri-drag-region></div>
  <div class="panes">
    <section class="listen" aria-label="Listening"></section>
    <section class="lib" aria-label="Library">
      <div class="lib-h" data-tauri-drag-region>
        <span class="tabs" role="group" aria-label="Library">
          <button aria-pressed={tab === 'new'} onclick={() => (tab = 'new')}>New</button>
          <button aria-pressed={tab === 'following'} onclick={() => (tab = 'following')}>Following</button>
        </span>
        <span class="modes">
          <button aria-label="Settings" aria-pressed={tab === 'settings'} onclick={() => (tab = 'settings')}>
            <svg width="15" height="15" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.1"><circle cx="8" cy="8" r="2.2"/><path d="M8 1.5v2M8 12.5v2M1.5 8h2M12.5 8h2M3.4 3.4l1.4 1.4M11.2 11.2l1.4 1.4M3.4 12.6l1.4-1.4M11.2 4.8l1.4-1.4"/></svg>
          </button>
        </span>
      </div>
      <div class="list">
        {#if tab === 'settings'}
          <div class="set">
            <h2>Settings</h2>
            <section>
              <div class="lbl">Appearance</div>
              <div class="words" role="group" aria-label="Appearance">
                <button aria-pressed={look === ''} onclick={() => pick('')}>Follow the system</button>
                <button aria-pressed={look === 'day'} onclick={() => pick('day')}>Day</button>
                <button aria-pressed={look === 'night'} onclick={() => pick('night')}>Night</button>
              </div>
            </section>
          </div>
        {/if}
      </div>
    </section>
  </div>
</main>

<style>
  .win { position: relative; height: 100vh; overflow: hidden; }
  /* the light: cast from the cover's centre (48 + 136/2, 64 + 136/2) in the show's colour */
  .win::before {
    content: ""; position: absolute; inset: 0; pointer-events: none;
    background: radial-gradient(var(--glow) calc(var(--glow) * 0.8) at 116px 132px,
      color-mix(in srgb, var(--cast) var(--cast-pct), transparent), transparent 62%);
  }
  .drag { position: absolute; inset: 0 0 auto 0; height: 44px; z-index: 1; }
  .panes { position: relative; display: grid; grid-template-columns: 1fr 400px; height: 100%; }
  .listen { padding: 64px 48px 44px; }
  .lib { border-left: 1px solid var(--hair); padding: 12px 22px 20px 32px; display: grid; grid-template-rows: 36px 1fr; min-height: 0; position: relative; z-index: 2; }
  .lib-h { display: flex; justify-content: space-between; align-items: center; }
  .tabs { display: flex; gap: 20px; align-items: baseline; }
  .tabs button { font-size: 14.5px; color: var(--text-faint); transition: color 0.14s ease; }
  .tabs button[aria-pressed="true"] { color: var(--text); }
  .modes button { width: 28px; height: 24px; display: grid; place-items: center; color: var(--text-faint); border-radius: 6px; transition: color 0.14s ease; }
  .modes button:hover { color: var(--text); }
  .modes button[aria-pressed="true"] { color: var(--text-dim); }
  .list { overflow: auto; scrollbar-width: none; min-height: 0; padding: 14px 10px 0 0; }
  .set h2 { font-weight: 250; font-size: 24px; line-height: 1.2; margin: 4px 0 26px; }
  .set .lbl { font-size: 12px; color: var(--text-faint); margin-bottom: 12px; }
  .words { display: flex; gap: 18px; flex-wrap: wrap; }
  .words button { font-size: 14px; color: var(--text-faint); transition: color 0.14s ease; }
  .words button:hover { color: var(--text-dim); }
  .words button[aria-pressed="true"] { color: var(--text); }
</style>

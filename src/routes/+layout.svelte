<script lang="ts">
  import '@fontsource-variable/hanken-grotesk';
  import '@fontsource/fragment-mono';
  import '$lib/tokens.css';
  import '$lib/app.css';
  let { children } = $props();
</script>

{@render children()}

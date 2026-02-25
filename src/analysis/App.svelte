<script lang="ts">
  import type { StructuredGameLog } from '../gameLog';
  import AnalysisDashboard from './AnalysisDashboard.svelte';

  let logs: StructuredGameLog[] = $state([]);
  let loading = $state(false);
  let error: string | null = $state(null);

  async function handleFolderSelect(event: Event) {
    const input = event.target as HTMLInputElement;
    const files = input.files;
    if (!files) return;

    loading = true;
    error = null;
    const parsed: StructuredGameLog[] = [];

    for (const file of files) {
      if (!file.name.endsWith('.json')) continue;
      try {
        const text = await file.text();
        const log = JSON.parse(text) as StructuredGameLog;
        if (log.version === 1) {
          parsed.push(log);
        }
      } catch (e) {
        // skip invalid files
      }
    }

    if (parsed.length === 0) {
      error = 'No valid game logs found in the selected folder.';
    }

    logs = parsed;
    loading = false;
  }
</script>

<main>
  <h1>Colori Game Analysis</h1>
  <div class="folder-picker">
    <label>
      Select game logs folder:
      <input type="file" onchange={handleFolderSelect} webkitdirectory />
    </label>
  </div>

  {#if loading}
    <p>Loading...</p>
  {:else if error}
    <p class="error">{error}</p>
  {:else if logs.length > 0}
    <p>{logs.length} games loaded</p>
    <AnalysisDashboard {logs} />
  {/if}
</main>

<style>
  main {
    max-width: 1200px;
    margin: 0 auto;
    padding: 2rem;
    font-family: system-ui, sans-serif;
  }
  .folder-picker {
    margin: 1rem 0;
  }
  .error {
    color: red;
  }
</style>

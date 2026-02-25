<script lang="ts">
  import type { StructuredGameLog } from '../gameLog';
  import AnalysisDashboard from './AnalysisDashboard.svelte';

  interface TaggedGameLog {
    log: StructuredGameLog;
    batchId: string;
  }

  let taggedLogs: TaggedGameLog[] = $state([]);
  let loading = $state(false);
  let error: string | null = $state(null);
  let selectedBatch: string = $state("all");

  let availableBatches = $derived(
    [...new Set(taggedLogs.map(t => t.batchId))].sort()
  );

  let filteredLogs = $derived(
    selectedBatch === "all"
      ? taggedLogs.map(t => t.log)
      : taggedLogs.filter(t => t.batchId === selectedBatch).map(t => t.log)
  );

  function extractBatchId(filename: string): string {
    const match = filename.match(/^game-\d+-([a-z0-9]{6})\.json$/);
    return match ? match[1] : "Unknown";
  }

  async function handleFolderSelect(event: Event) {
    const input = event.target as HTMLInputElement;
    const files = input.files;
    if (!files) return;

    loading = true;
    error = null;
    selectedBatch = "all";
    const parsed: TaggedGameLog[] = [];

    for (const file of files) {
      if (!file.name.endsWith('.json')) continue;
      try {
        const text = await file.text();
        const log = JSON.parse(text) as StructuredGameLog;
        if (log.version === 1) {
          const batchId = extractBatchId(file.name);
          parsed.push({ log, batchId });
        }
      } catch (e) {
        // skip invalid files
      }
    }

    if (parsed.length === 0) {
      error = 'No valid game logs found in the selected folder.';
    }

    taggedLogs = parsed;
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
  {:else if taggedLogs.length > 0}
    {#if selectedBatch === "all"}
      <p>{taggedLogs.length} games loaded</p>
    {:else}
      <p>{filteredLogs.length} of {taggedLogs.length} games shown</p>
    {/if}

    {#if availableBatches.length > 1}
      <div class="batch-filter">
        <label>
          Batch:
          <select bind:value={selectedBatch}>
            <option value="all">All batches</option>
            {#each availableBatches as batch}
              <option value={batch}>{batch} ({taggedLogs.filter(t => t.batchId === batch).length} games)</option>
            {/each}
          </select>
        </label>
      </div>
    {/if}

    <AnalysisDashboard logs={filteredLogs} />
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
  .batch-filter {
    margin: 0.5rem 0 1rem;
  }
  .error {
    color: red;
  }
</style>

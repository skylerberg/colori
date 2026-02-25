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

  interface BatchInfo {
    count: number;
    iterations?: number;
    note?: string;
  }

  let batchInfo = $derived(() => {
    const map = new Map<string, BatchInfo>();
    for (const t of taggedLogs) {
      if (!map.has(t.batchId)) {
        map.set(t.batchId, {
          count: 0,
          iterations: t.log.iterations,
          note: t.log.note,
        });
      }
      map.get(t.batchId)!.count++;
    }
    return map;
  });

  function batchLabel(batchId: string): string {
    const info = batchInfo().get(batchId);
    if (!info) return batchId;
    let label = batchId;
    if (info.iterations != null) label += ` (iters: ${info.iterations})`;
    if (info.note) label += ` - ${info.note}`;
    label += ` (${info.count} games)`;
    return label;
  }

  let selectedBatchInfo = $derived(
    selectedBatch !== "all" ? batchInfo().get(selectedBatch) ?? null : null
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
      {#if selectedBatchInfo}
        <div class="batch-meta">
          {#if selectedBatchInfo.iterations != null}
            <span class="meta-tag">Iterations: {selectedBatchInfo.iterations}</span>
          {/if}
          {#if selectedBatchInfo.note}
            <span class="meta-tag">Note: {selectedBatchInfo.note}</span>
          {/if}
        </div>
      {/if}
    {/if}

    {#if availableBatches.length > 1}
      <div class="batch-filter">
        <label>
          Batch:
          <select bind:value={selectedBatch}>
            <option value="all">All batches</option>
            {#each availableBatches as batch}
              <option value={batch}>{batchLabel(batch)}</option>
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
  .batch-meta {
    display: flex;
    gap: 1rem;
    margin: 0.5rem 0;
  }
  .meta-tag {
    background: #e8f0fe;
    padding: 0.25rem 0.75rem;
    border-radius: 4px;
    font-size: 0.9rem;
  }
  .error {
    color: red;
  }
</style>

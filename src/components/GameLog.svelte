<script lang="ts">
  let { entries }: {
    entries: string[];
  } = $props();

  let expanded = $state(false);
</script>

{#if entries.length > 0}
  <div class="game-log">
    <button class="log-header" onclick={() => expanded = !expanded}>
      Game Log ({entries.length}) {expanded ? '▲' : '▼'}
    </button>
    {#if expanded}
      <div class="log-entries">
        {#each [...entries].reverse() as entry}
          <div class="log-entry">{entry}</div>
        {/each}
      </div>
    {/if}
  </div>
{/if}

<style>
  .game-log {
    border: 1px solid #ddd;
    border-radius: 8px;
    background: #fff;
    overflow: hidden;
  }

  .log-header {
    width: 100%;
    padding: 8px 12px;
    font-size: 0.85rem;
    font-weight: 600;
    color: #4a3728;
    background: #faf8f5;
    border: none;
    cursor: pointer;
    text-align: left;
  }

  .log-header:hover {
    background: #f0ece6;
  }

  .log-entries {
    max-height: 200px;
    overflow-y: auto;
    border-top: 1px solid #eee;
  }

  .log-entry {
    padding: 4px 12px;
    font-size: 0.8rem;
    color: #555;
    border-bottom: 1px solid #f5f5f5;
  }

  .log-entry:last-child {
    border-bottom: none;
  }
</style>

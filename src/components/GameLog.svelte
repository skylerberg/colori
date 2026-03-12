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
    border: 1px solid var(--border-gold, rgba(201, 168, 76, 0.3));
    border-radius: 8px;
    background: var(--bg-panel, #ebe3d3);
    overflow: hidden;
  }

  .log-header {
    width: 100%;
    padding: 8px 10px;
    font-family: var(--font-display, 'Cinzel', serif);
    font-size: 0.75rem;
    font-weight: 600;
    color: var(--text-primary, #2c1e12);
    background: transparent;
    border: none;
    cursor: pointer;
    text-align: left;
    min-height: 44px;
    display: flex;
    align-items: center;
  }

  .log-header:hover {
    background: #e5daca;
  }

  .log-header:active {
    background: #e5daca;
  }

  .log-entries {
    max-height: 250px;
    overflow-y: auto;
    border-top: 1px solid var(--border-gold, rgba(201, 168, 76, 0.3));
    -webkit-overflow-scrolling: touch;
  }

  .log-entry {
    padding: 6px 12px;
    font-size: 0.875rem;
    color: var(--text-secondary, #6b5744);
    border-bottom: 1px solid rgba(201, 168, 76, 0.15);
  }

  .log-entry:last-child {
    border-bottom: none;
  }

  @media (min-width: 768px) {
    .log-header {
      padding: 5px 10px;
      min-height: auto;
    }

    .log-entries {
      max-height: 200px;
    }

    .log-entry {
      padding: 4px 12px;
      font-size: 0.8rem;
    }
  }
</style>

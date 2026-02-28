<script lang="ts">
  import type { StructuredGameLog, StructuredLogEntry } from '../gameLog';
  import { buildCardInstanceMap, buildBuyerInstanceMap, formatChoice } from './logAnalysis';
  import ColorWheelDisplay from '../components/ColorWheelDisplay.svelte';
  import CardList from '../components/CardList.svelte';

  let { log }: { log: StructuredGameLog } = $props();

  const PLAYER_COLORS = ['#e63946', '#3b82f6', '#2ecc71', '#f4a261', '#a855f7'];

  let selectedPlayer: number | null = $state(null);

  let cardMap = $derived(buildCardInstanceMap(log));
  let buyerMap = $derived(buildBuyerInstanceMap(log));

  let filteredEntries = $derived(
    selectedPlayer != null
      ? log.entries.filter(e => e.playerIndex === selectedPlayer)
      : log.entries
  );

  interface RoundGroup {
    round: number;
    phases: Map<string, StructuredLogEntry[]>;
  }

  let rounds = $derived(() => {
    const groups: RoundGroup[] = [];
    const roundMap = new Map<number, Map<string, StructuredLogEntry[]>>();

    for (const entry of filteredEntries) {
      if (!roundMap.has(entry.round)) {
        roundMap.set(entry.round, new Map());
      }
      const phases = roundMap.get(entry.round)!;
      if (!phases.has(entry.phase)) {
        phases.set(entry.phase, []);
      }
      phases.get(entry.phase)!.push(entry);
    }

    for (const [round, phases] of [...roundMap.entries()].sort((a, b) => a[0] - b[0])) {
      groups.push({ round, phases });
    }
    return groups;
  });

  let winner = $derived(() => {
    if (!log.finalScores) return null;
    let maxScore = -Infinity;
    for (const fs of log.finalScores) {
      if (fs.score > maxScore) maxScore = fs.score;
    }
    const winners = log.finalScores.filter(fs => fs.score === maxScore);
    return winners.map(w => w.name).join(', ');
  });

  let roundCount = $derived(() => {
    let max = 0;
    for (const entry of log.entries) {
      if (entry.round > max) max = entry.round;
    }
    return max;
  });

  function phaseName(phase: string): string {
    const names: Record<string, string> = {
      draw: 'Draw',
      draft: 'Draft',
      action: 'Action',
      cleanup: 'Cleanup',
    };
    return names[phase] ?? phase;
  }
</script>

<div class="game-viewer">
  <!-- Game Summary Header -->
  <div class="summary-bar">
    {#if log.finalScores}
      {#each log.finalScores as fs, i}
        <span class="player-score" style="border-color: {PLAYER_COLORS[i] ?? '#999'}">
          {fs.name}: {fs.score}pts
        </span>
      {/each}
      <span class="summary-detail">{roundCount()} rounds</span>
      <span class="summary-detail">Winner: {winner()}</span>
    {/if}
    {#if log.durationMs != null}
      <span class="summary-detail">{(log.durationMs / 1000).toFixed(1)}s</span>
    {/if}
  </div>

  <!-- Player Filter -->
  <div class="player-filter">
    <label>
      Player filter:
      <select bind:value={selectedPlayer}>
        <option value={null}>All players</option>
        {#each log.playerNames as name, i}
          <option value={i}>{name}</option>
        {/each}
      </select>
    </label>
  </div>

  <!-- Final State -->
  <details class="final-state">
    <summary>Final State</summary>
    {#if log.finalPlayerStats}
      <div class="final-players">
        {#each log.finalPlayerStats as ps, i}
          {#if selectedPlayer == null || selectedPlayer === i}
            <div class="final-player">
              <h4 style="color: {PLAYER_COLORS[i] ?? '#999'}">{ps.name}</h4>
              <div class="final-player-details">
                <ColorWheelDisplay wheel={ps.colorWheel} size={120} />
                <div class="final-stats">
                  <div>Ducats: {ps.ducats}</div>
                  <div>Deck size: {ps.deckSize}</div>
                  <div>
                    Materials:
                    {Object.entries(ps.materials).filter(([_, v]) => v > 0).map(([k, v]) => `${k}: ${v}`).join(', ') || 'none'}
                  </div>
                  {#if ps.completedBuyers.length > 0}
                    <div class="completed-buyers">
                      Completed buyers:
                      <CardList cards={ps.completedBuyers} />
                    </div>
                  {/if}
                </div>
              </div>
            </div>
          {/if}
        {/each}
      </div>
    {/if}
  </details>

  <!-- Round-by-Round Timeline -->
  <div class="timeline">
    {#each rounds() as group}
      <details class="round-group" open>
        <summary class="round-header">Round {group.round}</summary>
        {#each [...group.phases.entries()] as [phase, entries]}
          <div class="phase-section">
            <h4 class="phase-name">{phaseName(phase)}</h4>
            {#each entries as entry}
              <div class="timeline-entry">
                {#if selectedPlayer == null}
                  <span class="player-dot" style="background: {PLAYER_COLORS[entry.playerIndex] ?? '#999'}">
                    {log.playerNames[entry.playerIndex]}
                  </span>
                {/if}
                <span class="choice-text">{formatChoice(entry.choice, cardMap, buyerMap)}</span>
              </div>
            {/each}
          </div>
        {/each}
      </details>
    {/each}
  </div>
</div>

<style>
  .game-viewer {
    display: flex;
    flex-direction: column;
    gap: 1rem;
  }

  .summary-bar {
    display: flex;
    flex-wrap: wrap;
    gap: 0.75rem;
    align-items: center;
    padding: 0.75rem;
    background: #f8f9fa;
    border-radius: 6px;
  }

  .player-score {
    border-left: 3px solid;
    padding-left: 0.5rem;
    font-weight: 600;
  }

  .summary-detail {
    color: #666;
    font-size: 0.9rem;
  }

  .player-filter {
    margin: 0.25rem 0;
  }

  .final-state {
    border: 1px solid #ddd;
    border-radius: 6px;
    padding: 0.5rem 0.75rem;
  }

  .final-state summary {
    cursor: pointer;
    font-weight: 600;
  }

  .final-players {
    display: flex;
    flex-wrap: wrap;
    gap: 1.5rem;
    margin-top: 0.75rem;
  }

  .final-player {
    border: 1px solid #eee;
    border-radius: 6px;
    padding: 0.75rem;
    min-width: 200px;
  }

  .final-player h4 {
    margin: 0 0 0.5rem 0;
  }

  .final-player-details {
    display: flex;
    gap: 1rem;
    align-items: flex-start;
  }

  .final-stats {
    font-size: 0.9rem;
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
  }

  .timeline {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
  }

  .round-group {
    border: 1px solid #ddd;
    border-radius: 6px;
    padding: 0.5rem 0.75rem;
  }

  .round-header {
    cursor: pointer;
    font-weight: 600;
    font-size: 1.05rem;
  }

  .phase-section {
    margin: 0.5rem 0 0.5rem 0.5rem;
  }

  .phase-name {
    margin: 0.5rem 0 0.25rem 0;
    font-size: 0.85rem;
    text-transform: uppercase;
    color: #888;
    letter-spacing: 0.05em;
  }

  .timeline-entry {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0.15rem 0;
    font-size: 0.9rem;
  }

  .player-dot {
    color: white;
    font-size: 0.75rem;
    padding: 0.1rem 0.4rem;
    border-radius: 3px;
    white-space: nowrap;
    min-width: 60px;
    text-align: center;
  }

  .choice-text {
    color: #333;
  }
</style>

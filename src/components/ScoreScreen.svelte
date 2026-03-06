<script lang="ts">
  import type { GameState, StructuredGameLog } from '../data/types';
  import { calculateScores, determineWinners } from '../engine/wasmEngine';
  import { downloadGameLog } from '../gameLog';
  import { formatTime } from '../gameUtils';
  import ColorWheelDisplay from './ColorWheelDisplay.svelte';
  import CardList from './CardList.svelte';

  let { gameState, gameStartTime, onPlayAgain, structuredLog }: {
    gameState: GameState;
    gameStartTime: number | null;
    onPlayAgain: () => void;
    structuredLog?: StructuredGameLog | null;
  } = $props();

  let finalTime = $derived(
    gameStartTime !== null ? formatTime(Math.floor((Date.now() - gameStartTime) / 1000)) : null
  );

  let rounds = $derived(gameState.round - 1);
  let scores = $derived(calculateScores(gameState.players, gameState.playerNames));
  let winners = $derived(determineWinners(gameState.players, gameState.playerNames));
  let winnerSet = $derived(new Set(winners));
  let sortedScores = $derived(() => {
    const playerMap = new Map(gameState.players.map((p, i) => [gameState.playerNames[i], p]));
    return [...scores].sort((a, b) => {
      if (a.score !== b.score) return b.score - a.score;
      const pa = playerMap.get(a.name)!;
      const pb = playerMap.get(b.name)!;
      const buyersA = pa.completedBuyers.length;
      const buyersB = pb.completedBuyers.length;
      if (buyersA !== buyersB) return buyersB - buyersA;
      const colorsA = Object.values(pa.colorWheel).reduce((sum, c) => sum + (c as number), 0);
      const colorsB = Object.values(pb.colorWheel).reduce((sum, c) => sum + (c as number), 0);
      return colorsB - colorsA;
    });
  });

  let expandedPlayers = $state(new Set<string>());

  function togglePlayer(name: string) {
    if (expandedPlayers.has(name)) {
      expandedPlayers.delete(name);
    } else {
      expandedPlayers.add(name);
    }
    expandedPlayers = new Set(expandedPlayers);
  }

  function getPlayer(name: string) {
    const idx = gameState.playerNames.indexOf(name);
    return gameState.players[idx];
  }
</script>

<div class="score-screen">
  <h2>Game Over!</h2>

  <div class="winner-banner">
    {#if winners.length > 1}
      It's a tie!
    {:else}
      {winners[0]} wins!
    {/if}
  </div>

  <div class="game-time">
    {#if finalTime}Game time: {finalTime} | {/if}Rounds: {rounds}
  </div>

  <div class="scores-list">
    {#each sortedScores() as entry, i}
      {@const player = getPlayer(entry.name)}
      {@const expanded = expandedPlayers.has(entry.name)}
      <div class="score-entry" class:winner={winnerSet.has(entry.name)}>
        <button class="score-row" onclick={() => togglePlayer(entry.name)}>
          <span class="rank">#{i + 1}</span>
          <span class="name">{entry.name}</span>
          <span class="score">{'*'.repeat(entry.score)} ({entry.score})</span>
          <span class="chevron" class:open={expanded}></span>
        </button>

        {#if expanded}
          <div class="player-details">
            <div class="compact-row">
              <div class="detail-section">
                <h4>Color Wheel</h4>
                <ColorWheelDisplay wheel={player.colorWheel} size={150} />
              </div>

              <div class="detail-section">
                <h4>Materials</h4>
                <div class="material-counts">
                  {#each Object.entries(player.materials) as [material, count]}
                    <span class="material-count">{material}: {count}</span>
                  {/each}
                </div>
                {#if player.ducats > 0}
                  <div class="ducats-count">Ducats: {player.ducats}</div>
                {/if}
              </div>
            </div>

            <div class="card-section">
              <h4>Full Deck</h4>
              <CardList cards={[...player.deck, ...player.discard, ...player.workshoppedCards]} />
            </div>

            {#if player.completedBuyers.length > 0}
              <div class="card-section">
                <h4>Completed Buyers</h4>
                <CardList cards={player.completedBuyers} />
              </div>
            {/if}
          </div>
        {/if}
      </div>
    {/each}
  </div>

  <div class="button-row">
    {#if structuredLog}
      <button class="download-btn" onclick={() => downloadGameLog(structuredLog!)}>Download Game Log</button>
    {/if}
    <button class="play-again-btn" onclick={onPlayAgain}>Play Again</button>
  </div>
</div>

<style>
  .score-screen {
    max-width: 700px;
    margin: 2rem auto;
    display: flex;
    flex-direction: column;
    gap: 1.5rem;
    align-items: center;
  }

  h2 {
    font-family: var(--font-display, 'Cinzel', serif);
    color: var(--text-primary, #2c1e12);
    font-size: 1.8rem;
  }

  .winner-banner {
    font-family: var(--font-display, 'Cinzel', serif);
    font-size: 1.4rem;
    font-weight: 700;
    color: var(--accent-gold, #c9a84c);
    padding: 12px 24px;
    border: 3px solid var(--accent-gold, #c9a84c);
    border-radius: 12px;
    background: rgba(201, 168, 76, 0.08);
    letter-spacing: 1px;
  }

  .game-time {
    font-size: 1rem;
    color: var(--text-secondary, #6b5744);
  }

  .scores-list {
    width: 100%;
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .score-entry {
    border: 2px solid var(--border-gold, rgba(201, 168, 76, 0.3));
    border-radius: 8px;
    background: var(--bg-panel, #ebe3d3);
    overflow: hidden;
  }

  .score-entry.winner {
    border-color: var(--accent-gold, #c9a84c);
    background: rgba(201, 168, 76, 0.1);
  }

  .score-row {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 10px 16px;
    width: 100%;
    background: none;
    border: none;
    cursor: pointer;
    font: inherit;
    text-align: left;
  }

  .score-row:hover {
    background: rgba(201, 168, 76, 0.06);
  }

  .rank {
    font-family: var(--font-display, 'Cinzel', serif);
    font-weight: 700;
    font-size: 1rem;
    color: var(--text-tertiary, #9a8775);
    min-width: 30px;
  }

  .winner .rank {
    color: var(--accent-gold, #c9a84c);
  }

  .name {
    font-family: var(--font-display, 'Cinzel', serif);
    font-weight: 600;
    flex: 1;
    text-align: left;
  }

  .score {
    font-family: var(--font-display, 'Cinzel', serif);
    color: var(--accent-gold, #c9a84c);
    font-weight: 600;
  }

  .chevron {
    width: 0;
    height: 0;
    border-left: 5px solid transparent;
    border-right: 5px solid transparent;
    border-top: 5px solid var(--text-tertiary, #9a8775);
    transition: transform 0.2s;
  }

  .chevron.open {
    transform: rotate(180deg);
  }

  .player-details {
    padding: 8px 16px 16px;
    display: flex;
    flex-direction: column;
    gap: 8px;
    border-top: 1px solid var(--border-gold, rgba(201, 168, 76, 0.3));
  }

  .compact-row {
    display: flex;
    gap: 1rem;
    flex-wrap: wrap;
  }

  .detail-section {
    display: flex;
    flex-direction: column;
    align-items: center;
  }

  h4 {
    font-family: var(--font-display, 'Cinzel', serif);
    font-size: 0.7rem;
    color: var(--text-primary, #2c1e12);
    text-transform: uppercase;
    letter-spacing: 0.5px;
    margin: 0 0 4px;
  }

  .material-counts {
    display: flex;
    flex-direction: column;
    gap: 2px;
    font-size: 0.75rem;
    color: var(--accent-gold, #c9a84c);
  }

  .material-count {
    font-weight: 600;
  }

  .ducats-count {
    font-size: 0.75rem;
    color: var(--accent-gold, #c9a84c);
    font-weight: 600;
    margin-top: 4px;
  }

  .card-section {
    text-align: left;
  }

  .card-section :global(.card-list) {
    min-height: auto;
  }

  .button-row {
    display: flex;
    gap: 12px;
    align-items: center;
    margin-top: 1rem;
  }

  .play-again-btn {
    padding: 12px 32px;
    font-family: var(--font-display, 'Cinzel', serif);
    font-size: 1.05rem;
    font-weight: 600;
    letter-spacing: 1.5px;
    background: var(--bg-deep, #2c1e12);
    color: var(--text-on-dark, #f5ede0);
    border: none;
    border-radius: 8px;
    transition: background 0.2s, transform 0.2s;
  }

  .play-again-btn:hover {
    background: #3a2a1e;
    transform: translateY(-2px);
  }

  .download-btn {
    padding: 12px 24px;
    font-family: var(--font-display, 'Cinzel', serif);
    font-size: 0.95rem;
    font-weight: 600;
    letter-spacing: 1px;
    background: var(--text-secondary, #6b5744);
    color: var(--text-on-dark, #f5ede0);
    border: none;
    border-radius: 8px;
    cursor: pointer;
    transition: background 0.2s;
  }

  .download-btn:hover {
    background: var(--text-primary, #2c1e12);
  }
</style>

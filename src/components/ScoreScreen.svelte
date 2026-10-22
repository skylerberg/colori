<script lang="ts">
  import type { GameState, StructuredGameLog } from '../data/types';
  import { calculateScores, determineWinners } from '../engine/wasmEngine';
  import { downloadGameLog } from '../gameLog';
  import { formatScore, formatTime } from '../gameUtils';
  import CardList from './CardList.svelte';
  import CharacterPortrait from './CharacterPortrait.svelte';
  import PlayerBoard from './PlayerBoard.svelte';

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
      const buyersA = pa.completedOrders.length;
      const buyersB = pb.completedOrders.length;
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

  function seatOf(name: string) {
    return gameState.playerNames.indexOf(name);
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
    {[finalTime && `Game time: ${finalTime}`, `Rounds: ${rounds}`].filter(Boolean).join(' · ')}
  </div>

  <div class="scores-list">
    {#each sortedScores() as entry, i}
      {@const seat = seatOf(entry.name)}
      {@const player = gameState.players[seat]}
      {@const expanded = expandedPlayers.has(entry.name)}
      <div class="score-entry" class:winner={winnerSet.has(entry.name)}>
        <button class="score-row" onclick={() => togglePlayer(entry.name)} aria-expanded={expanded}>
          <span class="rank">#{i + 1}</span>
          <CharacterPortrait {seat} size={36} />
          <span class="name">{entry.name}</span>
          <span class="score">{formatScore(entry.score)} points</span>
          <span class="chevron" class:open={expanded}></span>
        </button>

        {#if expanded}
          <div class="player-details">
            <div class="final-board">
              <PlayerBoard {player} {seat} round={gameState.maxRounds} withTuckedCards={false} />
            </div>

            <div class="card-section">
              <h4>Full Deck</h4>
              <CardList cards={[...player.deck.flat(), ...player.workshopCards, ...player.usedCards, ...player.topCards]} />
            </div>

            {#if player.completedOrders.length > 0}
              <div class="card-section">
                <h4>Completed Orders</h4>
                <CardList cards={player.completedOrders} />
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
    width: 100%;
    max-width: 700px;
    margin: 1.5rem auto;
    display: flex;
    flex-direction: column;
    gap: 1rem;
    align-items: center;
    padding: 0 0.75rem;
  }

  h2 {
    font-family: var(--font-display, 'Cinzel', serif);
    color: var(--text-primary, #2c1e12);
    font-size: clamp(1.4rem, 4vw, 1.8rem);
  }

  .winner-banner {
    font-family: var(--font-display, 'Cinzel', serif);
    font-size: clamp(1.1rem, 3.5vw, 1.4rem);
    font-weight: 700;
    color: var(--accent-gold, #c9a84c);
    padding: 10px 16px;
    border: 3px solid var(--accent-gold, #c9a84c);
    border-radius: 12px;
    background: rgba(201, 168, 76, 0.08);
    letter-spacing: 1px;
    text-align: center;
    max-width: 100%;
    word-break: break-word;
  }

  .game-time {
    font-size: clamp(0.875rem, 2.5vw, 1rem);
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
    gap: 8px;
    padding: 10px 12px;
    width: 100%;
    background: none;
    border: none;
    cursor: pointer;
    font: inherit;
    text-align: left;
    min-height: 48px;
  }

  .score-row:hover {
    background: rgba(201, 168, 76, 0.06);
  }

  .rank {
    font-family: var(--font-display, 'Cinzel', serif);
    font-weight: 700;
    font-size: 1rem;
    color: var(--text-tertiary, #9a8775);
    min-width: 28px;
    flex-shrink: 0;
  }

  .winner .rank {
    color: var(--accent-gold, #c9a84c);
  }

  .name {
    font-family: var(--font-display, 'Cinzel', serif);
    font-weight: 600;
    flex: 1;
    text-align: left;
    font-size: clamp(0.85rem, 2.5vw, 1rem);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    min-width: 0;
  }

  .score {
    font-family: var(--font-display, 'Cinzel', serif);
    color: var(--accent-gold, #c9a84c);
    font-weight: 600;
    font-size: clamp(0.8rem, 2.5vw, 1rem);
    flex-shrink: 0;
  }

  .chevron {
    width: 0;
    height: 0;
    border-left: 5px solid transparent;
    border-right: 5px solid transparent;
    border-top: 5px solid var(--text-tertiary, #9a8775);
    transition: transform 0.2s;
    flex-shrink: 0;
  }

  .chevron.open {
    transform: rotate(180deg);
  }

  .player-details {
    padding: 8px 12px 16px;
    display: flex;
    flex-direction: column;
    gap: 8px;
    border-top: 1px solid var(--border-gold, rgba(201, 168, 76, 0.3));
  }



  h4 {
    font-family: var(--font-display, 'Cinzel', serif);
    font-size: 0.7rem;
    color: var(--text-primary, #2c1e12);
    text-transform: uppercase;
    letter-spacing: 0.5px;
    margin: 0 0 4px;
  }




  .final-board {
    padding: 4px 0 8px;
  }

  .card-section {
    text-align: left;
    overflow-x: auto;
  }

  .card-section :global(.card-list) {
    min-height: auto;
  }

  .button-row {
    display: flex;
    flex-direction: column;
    gap: 12px;
    align-items: stretch;
    width: 100%;
    margin-top: 1rem;
  }

  .play-again-btn {
    padding: 14px 32px;
    font-family: var(--font-display, 'Cinzel', serif);
    font-size: 1.05rem;
    font-weight: 600;
    letter-spacing: 1.5px;
    background: var(--bg-deep, #2c1e12);
    color: var(--text-on-dark, #f5ede0);
    border: none;
    border-radius: 8px;
    min-height: 48px;
    transition: background 0.2s, transform 0.2s;
  }

  .play-again-btn:hover {
    background: #3a2a1e;
    transform: translateY(-2px);
  }

  .download-btn {
    padding: 14px 24px;
    font-family: var(--font-display, 'Cinzel', serif);
    font-size: 0.95rem;
    font-weight: 600;
    letter-spacing: 1px;
    background: var(--text-secondary, #6b5744);
    color: var(--text-on-dark, #f5ede0);
    border: none;
    border-radius: 8px;
    cursor: pointer;
    min-height: 48px;
    transition: background 0.2s;
  }

  .download-btn:hover {
    background: var(--text-primary, #2c1e12);
  }

  @media (min-width: 640px) {
    .button-row {
      flex-direction: row;
      align-items: center;
      justify-content: center;
      width: auto;
    }
  }

  @media (min-width: 768px) {
    .score-screen {
      margin: 2rem auto;
      gap: 1.5rem;
      padding: 0;
    }

    .winner-banner {
      padding: 12px 24px;
    }

    .score-row {
      gap: 12px;
      padding: 10px 16px;
    }

    .player-details {
      padding: 8px 16px 16px;
    }
  }
</style>

<script lang="ts">
  import type { GameState } from '../data/types';
  import { formatScore, isTakingActionTurn, playerScore } from '../gameUtils';
  import CharacterPortrait from './CharacterPortrait.svelte';

  let { gameState, selectedPlayerIndex, onSelect, actingPlayers, thinkingPlayers, viewerIndex }: {
    gameState: GameState;
    selectedPlayerIndex: number;
    onSelect: (index: number) => void;
    actingPlayers: number[];
    thinkingPlayers: number[];
    // The seat this screen plays, or -1.
    viewerIndex: number;
  } = $props();

  function status(i: number): { text: string; kind: string } | null {
    if (thinkingPlayers.includes(i)) return { text: 'Thinking', kind: 'thinking' };
    if (actingPlayers.includes(i)) return { text: i === viewerIndex ? 'Your move' : 'To play', kind: 'acting' };
    if (gameState.phase.type === 'action' && !isTakingActionTurn(gameState, i)) return { text: 'Done', kind: 'done' };
    return null;
  }
</script>

<div class="player-tabs" role="tablist" aria-label="Player boards">
  {#each gameState.players as player, i}
    {@const s = status(i)}
    <button
      class="player-tab"
      class:active={i === selectedPlayerIndex}
      role="tab"
      aria-selected={i === selectedPlayerIndex}
      onclick={() => onSelect(i)}
      title="Show {gameState.playerNames[i]}'s board"
    >
      <CharacterPortrait seat={i} size={34} />
      <span class="tab-text">
        <span class="tab-name">
          {gameState.playerNames[i]}
          {#if i === viewerIndex}<span class="badge you">You</span>{/if}
          {#if gameState.aiPlayers[i]}<span class="badge ai">AI</span>{/if}
        </span>
        <span class="tab-meta">
          <span class="score">{formatScore(playerScore(player))} pts</span>
          {#if s}
            <span class="status {s.kind}">
              {#if s.kind === 'thinking'}<span class="spinner"></span>{:else if s.kind === 'acting'}<span class="dot"></span>{/if}
              {s.text}
            </span>
          {/if}
        </span>
      </span>
    </button>
  {/each}
</div>

<style>
  .player-tabs {
    display: flex;
    gap: 6px;
    overflow-x: auto;
    scrollbar-width: none;
  }

  .player-tabs::-webkit-scrollbar {
    display: none;
  }

  .player-tab {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 4px 12px 4px 4px;
    min-height: 0;
    flex-shrink: 0;
    background: rgba(20, 15, 10, 0.6);
    border: 1px solid rgba(201, 168, 76, 0.25);
    border-radius: 999px;
    color: rgba(245, 237, 224, 0.65);
    font-family: 'Cinzel', serif;
    text-align: left;
    transition: background 0.15s, border-color 0.15s, color 0.15s;
  }

  .player-tab:hover {
    background: rgba(40, 30, 20, 0.8);
    color: #f5ede0;
  }

  .player-tab.active {
    background: rgba(201, 168, 76, 0.2);
    border-color: #c9a84c;
    color: #f5ede0;
  }

  .tab-text {
    display: flex;
    flex-direction: column;
    line-height: 1.15;
  }

  .tab-name {
    font-size: 0.8rem;
    font-weight: 600;
    white-space: nowrap;
    display: flex;
    align-items: center;
    gap: 5px;
  }

  .tab-meta {
    display: flex;
    gap: 8px;
    font-family: 'Cormorant Garamond', serif;
    font-size: 0.85rem;
    white-space: nowrap;
  }

  .score {
    color: #e8c766;
    font-weight: 700;
  }

  .badge {
    font-size: 0.55rem;
    font-weight: 700;
    padding: 1px 4px;
    border-radius: 3px;
    letter-spacing: 0.05em;
  }

  .badge.ai {
    background: rgba(139, 32, 32, 0.85);
    color: #f5ede0;
  }

  .badge.you {
    background: rgba(201, 168, 76, 0.85);
    color: #2c1e12;
  }

  .status {
    display: inline-flex;
    align-items: center;
    gap: 4px;
  }

  .status.acting {
    color: #e8c766;
  }

  .status.done {
    color: rgba(245, 237, 224, 0.45);
  }

  .dot {
    width: 7px;
    height: 7px;
    border-radius: 50%;
    background: #e8c766;
    box-shadow: 0 0 6px rgba(232, 199, 102, 0.8);
  }

  .spinner {
    width: 10px;
    height: 10px;
    border: 2px solid rgba(201, 168, 76, 0.3);
    border-top-color: #c9a84c;
    border-radius: 50%;
    animation: spin 0.8s linear infinite;
  }

  @keyframes spin {
    to { transform: rotate(360deg); }
  }
</style>

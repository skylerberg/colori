<script lang="ts">
  import type { GameState } from '../data/types';
  import { executeDrawPhase } from '../engine/drawPhase';
  import PlayerStatus from './PlayerStatus.svelte';
  import DrawPhaseView from './DrawPhaseView.svelte';
  import DraftPhaseView from './DraftPhaseView.svelte';
  import ActionPhaseView from './ActionPhaseView.svelte';

  let { gameState, onGameUpdated }: {
    gameState: GameState;
    onGameUpdated: (state: GameState) => void;
  } = $props();

  // Track whether we have executed the draw phase for the current round
  // so we don't re-execute on every re-render.
  let drawExecutedForRound: number | null = $state(null);
  let showDrawPhase = $state(false);

  // When the phase is 'draw' and we haven't executed for this round,
  // automatically execute the draw phase.
  $effect(() => {
    if (gameState.phase.type === 'draw' && drawExecutedForRound !== gameState.round) {
      drawExecutedForRound = gameState.round;
      executeDrawPhase(gameState);
      showDrawPhase = true;
      onGameUpdated(gameState);
    }
  });

  function handleDrawContinue() {
    showDrawPhase = false;
    // The draw phase already transitioned to draft via executeDrawPhase,
    // so we just need to trigger a re-render.
    onGameUpdated(gameState);
  }

  function handleGameUpdated() {
    onGameUpdated(gameState);
  }

  let activePlayerIndex = $derived(getActivePlayerIndex(gameState));

  function getActivePlayerIndex(gs: GameState): number {
    if (gs.phase.type === 'draft') {
      return gs.phase.draftState.currentPlayerIndex;
    }
    if (gs.phase.type === 'action') {
      return gs.phase.actionState.currentPlayerIndex;
    }
    return -1;
  }
</script>

<div class="game-screen">
  <div class="top-bar">
    <div class="round-indicator">Round {gameState.round} of 8</div>
    <div class="player-bar">
      {#each gameState.players as player, i}
        <PlayerStatus {player} active={i === activePlayerIndex} />
      {/each}
    </div>
  </div>

  <div class="phase-content">
    {#if showDrawPhase && gameState.phase.type === 'draft'}
      <DrawPhaseView {gameState} onContinue={handleDrawContinue} />
    {:else if gameState.phase.type === 'draft'}
      <DraftPhaseView {gameState} onGameUpdated={handleGameUpdated} />
    {:else if gameState.phase.type === 'action'}
      <ActionPhaseView {gameState} onGameUpdated={handleGameUpdated} />
    {/if}
  </div>
</div>

<style>
  .game-screen {
    display: flex;
    flex-direction: column;
    gap: 1rem;
  }

  .top-bar {
    display: flex;
    flex-direction: column;
    gap: 8px;
    padding-bottom: 8px;
    border-bottom: 2px solid #e0d5c5;
  }

  .round-indicator {
    font-size: 0.85rem;
    font-weight: 600;
    color: #4a3728;
    text-align: center;
  }

  .player-bar {
    display: flex;
    gap: 8px;
    overflow-x: auto;
    justify-content: center;
    flex-wrap: wrap;
  }

  .phase-content {
    min-height: 300px;
  }
</style>

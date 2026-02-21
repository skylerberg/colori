<script lang="ts">
  import type { GameState, CardInstance } from '../data/types';
  import { executeDrawPhase } from '../engine/drawPhase';
  import { playerPick, confirmPass } from '../engine/draftPhase';
  import {
    destroyDraftedCard, endPlayerTurn, resolveStoreColors,
    resolveMixColors, skipMix, resolveDestroyCards,
    resolveChooseGarment, resolveGarmentPayment,
  } from '../engine/actionPhase';
  import { AIController } from '../ai/aiController';
  import type { ColoriChoice } from '../ai/coloriGame';
  import PlayerStatus from './PlayerStatus.svelte';
  import DrawPhaseView from './DrawPhaseView.svelte';
  import DraftPhaseView from './DraftPhaseView.svelte';
  import ActionPhaseView from './ActionPhaseView.svelte';

  let { gameState, onGameUpdated }: {
    gameState: GameState;
    onGameUpdated: (state: GameState) => void;
  } = $props();

  let drawExecutedForRound: number | null = $state(null);
  let showDrawPhase = $state(false);
  let aiThinking = $state(false);

  const aiController = new AIController();

  // Per-AI-player seen hands for draft knowledge tracking
  let seenHands: Map<number, CardInstance[][]> = $state(new Map());

  // When the phase is 'draw', automatically execute the draw phase.
  $effect(() => {
    if (gameState.phase.type === 'draw' && drawExecutedForRound !== gameState.round) {
      drawExecutedForRound = gameState.round;
      executeDrawPhase(gameState);
      // Reset seenHands for the new draft round
      seenHands = new Map();
      // Check if any human players exist to show draw phase
      const hasHuman = gameState.aiPlayers.some(ai => !ai);
      if (hasHuman) {
        showDrawPhase = true;
      }
      onGameUpdated(gameState);
    }
  });

  function handleDrawContinue() {
    showDrawPhase = false;
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

  function isCurrentPlayerAI(gs: GameState): boolean {
    const idx = getActivePlayerIndex(gs);
    return idx >= 0 && gs.aiPlayers[idx];
  }

  // Apply an AI choice to the game state
  function applyAIChoice(choice: ColoriChoice) {
    switch (choice.type) {
      case 'draftPick':
        playerPick(gameState, choice.cardInstanceId);
        if (gameState.phase.type === 'draft' && gameState.phase.draftState.waitingForPass) {
          confirmPass(gameState);
        }
        break;
      case 'destroyDraftedCard':
        destroyDraftedCard(gameState, choice.cardInstanceId);
        break;
      case 'endTurn':
        endPlayerTurn(gameState);
        if (gameState.phase.type === 'draw') {
          // Will be handled by the draw phase effect
        }
        break;
      case 'storeColors':
        resolveStoreColors(gameState, choice.cardInstanceIds);
        break;
      case 'destroyDrawnCards':
        resolveDestroyCards(gameState, choice.cardInstanceIds);
        break;
      case 'mix':
        resolveMixColors(gameState, choice.colorA, choice.colorB);
        break;
      case 'skipMix':
        skipMix(gameState);
        break;
      case 'chooseGarment':
        resolveChooseGarment(gameState, choice.garmentInstanceId);
        break;
      case 'garmentPayment':
        resolveGarmentPayment(gameState, choice.fabricCardId, choice.paymentType, choice.dyeCardId);
        break;
    }
    onGameUpdated(gameState);
  }

  // Trigger AI turn when the active player is AI
  $effect(() => {
    if (aiThinking) return;
    if (showDrawPhase) return;
    if (gameState.phase.type === 'gameOver') return;
    if (gameState.phase.type === 'draw') return;

    // Handle draft waitingForPass for AI
    if (gameState.phase.type === 'draft' && gameState.phase.draftState.waitingForPass) {
      const nextPlayer = gameState.phase.draftState.currentPlayerIndex;
      if (gameState.aiPlayers[nextPlayer]) {
        confirmPass(gameState);
        onGameUpdated(gameState);
      }
      return;
    }

    if (!isCurrentPlayerAI(gameState)) return;

    const playerIdx = getActivePlayerIndex(gameState);

    // Record seen hand for draft knowledge tracking
    if (gameState.phase.type === 'draft') {
      const ds = gameState.phase.draftState;
      const hand = ds.hands[playerIdx];
      if (!seenHands.has(playerIdx)) {
        seenHands.set(playerIdx, []);
      }
      const playerSeenHands = seenHands.get(playerIdx)!;
      if (playerSeenHands.length <= ds.pickNumber) {
        playerSeenHands.push([...hand]);
      }
    }

    aiThinking = true;
    const playerSeenHands = seenHands.get(playerIdx);

    aiController.getAIChoice(gameState, playerIdx, 10000, playerSeenHands).then((choice) => {
      aiThinking = false;
      applyAIChoice(choice);
    });
  });
</script>

<div class="game-screen">
  <div class="top-bar">
    <div class="round-indicator">Round {gameState.round} of 8</div>
    <div class="player-bar">
      {#each gameState.players as player, i}
        <PlayerStatus {player} active={i === activePlayerIndex} isAI={gameState.aiPlayers[i]} />
      {/each}
    </div>
  </div>

  <div class="phase-content">
    {#if aiThinking}
      <div class="ai-thinking">
        <div class="spinner"></div>
        <p>AI is thinking...</p>
      </div>
    {/if}

    {#if showDrawPhase && gameState.phase.type === 'draft'}
      <DrawPhaseView {gameState} onContinue={handleDrawContinue} />
    {:else if gameState.phase.type === 'draft' && !isCurrentPlayerAI(gameState)}
      <DraftPhaseView {gameState} onGameUpdated={handleGameUpdated} />
    {:else if gameState.phase.type === 'action' && !isCurrentPlayerAI(gameState)}
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

  .ai-thinking {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 1rem;
    padding: 3rem 1rem;
    color: #666;
  }

  .ai-thinking p {
    font-size: 1.1rem;
    font-weight: 600;
    color: #e67e22;
  }

  .spinner {
    width: 40px;
    height: 40px;
    border: 4px solid #e0d5c5;
    border-top-color: #e67e22;
    border-radius: 50%;
    animation: spin 0.8s linear infinite;
  }

  @keyframes spin {
    to { transform: rotate(360deg); }
  }
</style>

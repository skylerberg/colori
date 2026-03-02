<script lang="ts">
  import type { GameState, Choice } from '../data/types';
  import MixColorPrompt from './MixColorPrompt.svelte';
  import BuyerSelectPrompt from './BuyerSelectPrompt.svelte';
  import PrimaryColorPrompt from './PrimaryColorPrompt.svelte';
  import SecondaryColorPrompt from './SecondaryColorPrompt.svelte';
  import TertiarySwapPrompt from './TertiarySwapPrompt.svelte';

  let { gameState, onAction }: {
    gameState: GameState;
    onAction: (choice: Choice) => void;
  } = $props();

  let actionState = $derived(
    gameState.phase.type === 'action' ? gameState.phase.actionState : null
  );

  let pendingChoice = $derived(actionState?.pendingChoice ?? null);

  let currentPlayer = $derived(
    actionState ? gameState.players[actionState.currentPlayerIndex] : null
  );
</script>

{#if pendingChoice && currentPlayer}
  <div class="ability-prompt">
    {#if pendingChoice.type === 'chooseMix'}
      <MixColorPrompt
        colorWheel={currentPlayer.colorWheel}
        remaining={pendingChoice.remaining}
        {onAction}
      />
    {:else if pendingChoice.type === 'chooseBuyer'}
      <BuyerSelectPrompt {gameState} {onAction} />
    {:else if pendingChoice.type === 'choosePrimaryColor'}
      <PrimaryColorPrompt {onAction} />
    {:else if pendingChoice.type === 'chooseSecondaryColor'}
      <SecondaryColorPrompt {onAction} />
    {:else if pendingChoice.type === 'chooseTertiaryToLose'}
      <TertiarySwapPrompt colorWheel={currentPlayer.colorWheel} {onAction} />
    {/if}
  </div>
{/if}

<style>
  .ability-prompt {
    border: 2px solid #d4a017;
    border-radius: 10px;
    padding: 16px;
    background: #fffef0;
  }
</style>

<script lang="ts">
  import type { GameState, Choice } from '../data/types';
  import { buyerCapacity } from '../gameUtils';
  import { tableInteraction } from '../stores/tableInteraction';

  // Shown to the player whose claim it is.
  let { gameState, playerIndex, onAction }: {
    gameState: GameState;
    playerIndex: number;
    onAction: (choice: Choice) => void;
  } = $props();

  const interaction = tableInteraction();

  let player = $derived(gameState.players[playerIndex]);
  let deckCount = $derived(gameState.sellCardDeck.length);

  function claim(instanceId: number) {
    const instance = gameState.sellCardDisplay.find(c => c.instanceId === instanceId);
    if (instance) onAction({ type: 'takeBuyer', sellCard: instance.card });
  }

  function drawUnseen() {
    onAction({ type: 'drawBuyer' });
  }

  $effect(() => {
    interaction.market = { onClaim: claim, onDrawUnseen: deckCount > 0 ? drawUnseen : null };
    return () => {
      interaction.market = null;
    };
  });
</script>

<div class="buyers-phase">
  <div class="panel-heading">Claim a buyer</div>
  <p class="panel-text">
    Click one of the sell cards on the Buyer's Board to take it into your free buyer slot
    ({player.buyers.length + 1} of {buyerCapacity(gameState.round)}). Only buyers on your own board can be sold to
    this round, and the one you take is replaced straight away.
  </p>
  <div class="panel-actions">
    <button class="panel-btn" onclick={drawUnseen} disabled={deckCount === 0}>
      Draw the top of the sell deck, unseen ({deckCount} left)
    </button>
  </div>
</div>

<style>
  .buyers-phase {
    display: flex;
    flex-direction: column;
    gap: 10px;
  }
</style>

<script lang="ts">
  import type { GameState, Choice, Ability, Card, GlassCard, Color, MaterialType } from '../data/types';
  import { ALL_MATERIAL_TYPES } from '../data/types';
  import { orderByDraftOrder } from '../gameUtils';
  import { getGlassCardData, GLASS_CARD_ORDER } from '../data/glassCards';
  import { getAnyCardData, getCardData, PRIMARIES } from '../data/cards';
  import { colorToHex, textColorForBackground } from '../data/colors';
  import { canSell } from '../engine/wasmEngine';
  import CardList from './CardList.svelte';
  import AbilityPrompt from './AbilityPrompt.svelte';
  import MixColorPrompt from './MixColorPrompt.svelte';
  import SellCardSelectPrompt from './SellCardSelectPrompt.svelte';

  let { gameState, onAction, onUndo, undoAvailable, draftCardOrder }: {
    gameState: GameState;
    onAction: (choice: Choice) => void;
    onUndo: () => void;
    undoAvailable: boolean;
    draftCardOrder?: number[][];
  } = $props();

  let actionState = $derived(
    gameState.phase.type === 'action' ? gameState.phase.actionState : null
  );

  let currentPlayer = $derived(
    actionState ? gameState.players[actionState.currentPlayerIndex] : null
  );

  let topAbility: Ability | null = $derived(
    actionState && actionState.abilityStack.length > 0
      ? actionState.abilityStack[actionState.abilityStack.length - 1]
      : null
  );
  let hasPendingChoice = $derived(topAbility !== null);

  let workshopAndWorkshopped = $derived(
    currentPlayer ? [...currentPlayer.workshopCards, ...currentPlayer.workshoppedCards] : []
  );
  let workshoppedIds = $derived(
    currentPlayer ? currentPlayer.workshoppedCards.map(c => c.instanceId) : []
  );
  let hasAbilitiesQueued = $derived((actionState?.abilityStack.length ?? 0) > 0);

  let workshopPendingChoice = $derived(
    topAbility?.type === 'workshop' || topAbility?.type === 'destroyCards'
      ? topAbility : null
  );

  // ── Compound destroy state ──
  let pendingDestroyCard: { card: Card; instanceId: number } | null = $state(null);
  let pendingDestroyAbility: Ability | null = $state(null);
  let compoundWorkshopSelectedIds: number[] = $state([]);
  let compoundDestroySelectedIds: number[] = $state([]);

  let hasPendingCompound = $derived(pendingDestroyCard !== null);

  $effect(() => {
    // Reset pending destroy when ability stack changes (e.g. after action resolves)
    topAbility;
    pendingDestroyCard = null;
    pendingDestroyAbility = null;
    compoundWorkshopSelectedIds = [];
    compoundDestroySelectedIds = [];
  });

  function cancelPendingDestroy() {
    pendingDestroyCard = null;
    pendingDestroyAbility = null;
    compoundWorkshopSelectedIds = [];
    compoundDestroySelectedIds = [];
  }

  function compoundAction(choice: Choice) {
    const card = pendingDestroyCard!.card;
    cancelPendingDestroy();
    if (choice.type === 'mixAll') {
      onAction({ type: 'destroyAndMix', card, mixes: choice.mixes });
    } else if (choice.type === 'selectSellCard') {
      onAction({ type: 'destroyAndSell', card, sellCard: choice.sellCard });
    } else if (choice.type === 'selectGlass') {
      onAction({ type: 'destroyAndSelectGlass', card, glass: choice.glass, payColor: choice.payColor });
    }
  }

  function toggleCompoundWorkshopCard(instanceId: number) {
    if (!pendingDestroyAbility || pendingDestroyAbility.type !== 'workshop') return;
    const idx = compoundWorkshopSelectedIds.indexOf(instanceId);
    if (idx >= 0) {
      compoundWorkshopSelectedIds = compoundWorkshopSelectedIds.filter(id => id !== instanceId);
    } else if (compoundWorkshopSelectedIds.length < pendingDestroyAbility.count) {
      compoundWorkshopSelectedIds = [...compoundWorkshopSelectedIds, instanceId];
    }
  }

  function confirmCompoundWorkshop() {
    if (!currentPlayer || !pendingDestroyCard) return;
    const card = pendingDestroyCard.card;
    const workshopCards = compoundWorkshopSelectedIds.map(id => {
      const ci = currentPlayer!.workshopCards.find(c => c.instanceId === id);
      return ci!.card;
    });
    cancelPendingDestroy();
    onAction({ type: 'destroyAndWorkshop', card, workshopCards });
  }

  function skipCompoundWorkshop() {
    if (!pendingDestroyCard) return;
    const card = pendingDestroyCard.card;
    cancelPendingDestroy();
    onAction({ type: 'destroyAndWorkshop', card, workshopCards: [] });
  }

  function toggleCompoundDestroyCard(instanceId: number) {
    const idx = compoundDestroySelectedIds.indexOf(instanceId);
    if (idx >= 0) {
      compoundDestroySelectedIds = compoundDestroySelectedIds.filter(id => id !== instanceId);
    } else if (compoundDestroySelectedIds.length < 1) {
      compoundDestroySelectedIds = [...compoundDestroySelectedIds, instanceId];
    }
  }

  function confirmCompoundDestroy() {
    if (!currentPlayer || !pendingDestroyCard) return;
    const card = pendingDestroyCard.card;
    const target = compoundDestroySelectedIds.length > 0
      ? currentPlayer.workshopCards.find(c => c.instanceId === compoundDestroySelectedIds[0])!.card
      : null;
    cancelPendingDestroy();
    onAction({ type: 'destroyAndDestroyCards', card, target });
  }

  let selectedWorkshopIds: number[] = $state([]);
  let selectedDestroyIds: number[] = $state([]);
  let reworkshopCardId: number | null = $state(null);

  $effect(() => {
    topAbility;
    selectedWorkshopIds = [];
    selectedDestroyIds = [];
    reworkshopCardId = null;
  });

  function toggleWorkshopCard(instanceId: number) {
    if (!topAbility || topAbility.type !== 'workshop') return;
    // Don't allow selecting the card that's marked for x2 reworkshop
    if (instanceId === reworkshopCardId) return;
    const idx = selectedWorkshopIds.indexOf(instanceId);
    const reworkshopSlots = reworkshopCardId !== null ? 2 : 0;
    const maxOther = topAbility.count - reworkshopSlots;
    if (idx >= 0) {
      selectedWorkshopIds = selectedWorkshopIds.filter(id => id !== instanceId);
    } else if (selectedWorkshopIds.length < maxOther) {
      selectedWorkshopIds = [...selectedWorkshopIds, instanceId];
    }
  }

  function confirmWorkshop() {
    if (!currentPlayer) return;
    if (reworkshopCardId !== null) {
      const reworkshopCard = currentPlayer.workshopCards.find(c => c.instanceId === reworkshopCardId);
      if (!reworkshopCard) return;
      const otherCards = selectedWorkshopIds.map(id => {
        const ci = currentPlayer!.workshopCards.find(c => c.instanceId === id);
        return ci!.card;
      });
      onAction({ type: 'workshopWithReworkshop', reworkshopCard: reworkshopCard.card, otherCards });
    } else {
      const cardTypes = selectedWorkshopIds.map(id => {
        const ci = currentPlayer!.workshopCards.find(c => c.instanceId === id);
        return ci!.card;
      });
      onAction({ type: 'workshop', cardTypes });
    }
  }

  function handleSkipWorkshop() {
    onAction({ type: 'skipWorkshop' });
  }

  function toggleDestroyCard(instanceId: number) {
    if (!topAbility || topAbility.type !== 'destroyCards') return;
    const idx = selectedDestroyIds.indexOf(instanceId);
    if (idx >= 0) {
      selectedDestroyIds = selectedDestroyIds.filter(id => id !== instanceId);
    } else if (selectedDestroyIds.length < 1) {
      selectedDestroyIds = [...selectedDestroyIds, instanceId];
    }
  }

  function confirmDestroy() {
    if (!currentPlayer) return;
    const card = selectedDestroyIds.length > 0
      ? currentPlayer.workshopCards.find(c => c.instanceId === selectedDestroyIds[0])!.card
      : null;
    onAction({ type: 'destroyDrawnCards', card });
  }

  function handleDestroyDrafted(cardInstanceId: number) {
    if (hasPendingChoice || hasPendingCompound || !currentPlayer) return;
    const ci = currentPlayer.draftedCards.find(c => c.instanceId === cardInstanceId);
    if (!ci) return;

    const cardData = getCardData(ci.card as Card);
    if (!cardData || cardData.kind === 'sellCard') {
      onAction({ type: 'destroyDraftedCard', card: ci.card });
      return;
    }

    const ability = cardData.ability;
    if (ability.type === 'mixColors' || ability.type === 'workshop' || ability.type === 'destroyCards') {
      pendingDestroyCard = { card: ci.card as Card, instanceId: cardInstanceId };
      pendingDestroyAbility = ability;
      compoundWorkshopSelectedIds = [];
      compoundDestroySelectedIds = [];
    } else if (ability.type === 'sell') {
      const canSellAny = gameState.sellCardDisplay.some(g => canSell(gameState, g.instanceId));
      const canBuyGlass = gameState.expansions?.glass
        && gameState.glassDisplay.length > 0
        && PRIMARIES.some(c => currentPlayer!.colorWheel[c] >= 4);
      if (canSellAny || canBuyGlass) {
        pendingDestroyCard = { card: ci.card as Card, instanceId: cardInstanceId };
        pendingDestroyAbility = ability;
      } else {
        onAction({ type: 'destroyDraftedCard', card: ci.card });
      }
    } else {
      onAction({ type: 'destroyDraftedCard', card: ci.card });
    }
  }

  function handleEndTurn() {
    onAction({ type: 'endTurn' });
  }

  // ── Glass Abilities ──

  function isGlassUsed(glass: GlassCard): boolean {
    if (!actionState) return true;
    const idx = GLASS_CARD_ORDER.indexOf(glass);
    if (idx < 0) return true;
    return (actionState.usedGlass & (1 << idx)) !== 0;
  }

  let availableGlass = $derived(
    currentPlayer && actionState && gameState.expansions?.glass
      ? (currentPlayer.completedGlass ?? []).filter(
          g => g.card !== 'GlassKeepBoth' && !isGlassUsed(g.card)
        )
      : []
  );

  let activeGlassPrompt: GlassCard | null = $state(null);
  let glassExchangeLose: MaterialType | undefined = $state(undefined);
  let glassExchangeGain: MaterialType | undefined = $state(undefined);

  $effect(() => {
    // Reset glass prompt when ability stack changes
    topAbility;
    activeGlassPrompt = null;
    glassExchangeLose = undefined;
    glassExchangeGain = undefined;
  });

  // Clear sub-selection when the active glass ability is no longer available (e.g. just used)
  $effect(() => {
    if (activeGlassPrompt && !availableGlass.some(g => g.card === activeGlassPrompt)) {
      activeGlassPrompt = null;
      glassExchangeLose = undefined;
      glassExchangeGain = undefined;
    }
  });

  function handleGlassClick(glass: GlassCard) {
    // Simple activations (no parameters)
    if (glass === 'GlassDraw') {
      onAction({ type: 'activateGlassDraw' });
      return;
    }
    if (glass === 'GlassMix') {
      onAction({ type: 'activateGlassMix' });
      return;
    }
    if (glass === 'GlassGainPrimary') {
      onAction({ type: 'activateGlassGainPrimary' });
      return;
    }
    if (glass === 'GlassWorkshop') {
      onAction({ type: 'activateGlassWorkshop' });
      return;
    }

    // Parameterized — toggle inline sub-selection
    if (activeGlassPrompt === glass) {
      activeGlassPrompt = null;
    } else {
      activeGlassPrompt = glass;
      glassExchangeLose = undefined;
      glassExchangeGain = undefined;
    }
  }

  function cancelGlassPrompt() {
    activeGlassPrompt = null;
    glassExchangeLose = undefined;
    glassExchangeGain = undefined;
  }

  // Glass Exchange helpers
  let exchangeLoseOptions = $derived(
    currentPlayer
      ? ALL_MATERIAL_TYPES.filter(m => currentPlayer!.materials[m] >= 1)
      : []
  );

  let exchangeGainOptions = $derived(
    glassExchangeLose
      ? ALL_MATERIAL_TYPES.filter(m => m !== glassExchangeLose)
      : []
  );

  function confirmExchange() {
    if (!glassExchangeLose || !glassExchangeGain) return;
    onAction({ type: 'activateGlassExchange', lose: glassExchangeLose, gain: glassExchangeGain });
  }

  // Glass Unmix helpers
  const NON_PRIMARY_COLORS: Color[] = ['Vermilion', 'Orange', 'Amber', 'Chartreuse', 'Green', 'Teal', 'Indigo', 'Purple', 'Magenta'];
  const TERTIARY_COLORS: Color[] = ['Vermilion', 'Amber', 'Chartreuse', 'Teal', 'Indigo', 'Magenta'];

  let unmixableColors = $derived(
    currentPlayer
      ? NON_PRIMARY_COLORS.filter(c => currentPlayer!.colorWheel[c] > 0)
      : []
  );

  let tertiaryWithCount = $derived(
    currentPlayer
      ? TERTIARY_COLORS.filter(c => currentPlayer!.colorWheel[c] > 0)
      : []
  );

  let canUseReworkshopX2 = $derived(
    topAbility?.type === 'workshop'
    && topAbility.count >= 2
    && gameState.expansions?.glass
    && currentPlayer
    && (currentPlayer.completedGlass ?? []).some(g => g.card === 'GlassReworkshop')
    && !isGlassUsed('GlassReworkshop')
  );

  // Glass Reworkshop helpers (available during workshop ability too)
  let glassReworkshopAvailable = $derived(
    currentPlayer && actionState && gameState.expansions?.glass
    && (currentPlayer.completedGlass ?? []).some(g => g.card === 'GlassReworkshop')
    && !isGlassUsed('GlassReworkshop')
    && currentPlayer.workshoppedCards.length > 0
  );
</script>

{#if actionState && currentPlayer}
  <div class="action-phase">
    <h2 class="phase-title">Action Phase - {gameState.playerNames[actionState.currentPlayerIndex]}'s Turn</h2>
    <div class="queue-status">
      {#if hasAbilitiesQueued}
        <span class="queue-info">Abilities queued: {actionState.abilityStack.length}</span>
      {/if}
      {#if hasPendingChoice || hasPendingCompound}
        <span class="pending-info">Awaiting your choice...</span>
      {/if}
    </div>

    {#if hasPendingChoice && !workshopPendingChoice}
      <AbilityPrompt {gameState} {onAction} />
    {/if}

    {#if hasPendingCompound && pendingDestroyAbility}
      {@const cardName = (() => { const d = getAnyCardData(pendingDestroyCard!.card); return d && 'name' in d ? d.name : pendingDestroyCard!.card; })()}
      <div class="ability-prompt">
        <div class="compound-header">
          <span class="compound-title">Destroying {cardName}</span>
          <button class="confirm-btn skip-btn compound-cancel" onclick={cancelPendingDestroy}>Cancel</button>
        </div>
        {#if pendingDestroyAbility.type === 'mixColors'}
          <MixColorPrompt
            colorWheel={currentPlayer.colorWheel}
            remaining={pendingDestroyAbility.count}
            onAction={compoundAction}
          />
        {:else if pendingDestroyAbility.type === 'sell'}
          <SellCardSelectPrompt {gameState} onAction={compoundAction} />
        {:else if pendingDestroyAbility.type === 'workshop'}
          <h3>Workshop — Select cards ({pendingDestroyAbility.count} available)</h3>
          <CardList
            cards={workshopAndWorkshopped}
            selectable={true}
            selectedIds={compoundWorkshopSelectedIds}
            rotatedIds={workshoppedIds}
            onCardClick={toggleCompoundWorkshopCard}
          />
          <div class="workshop-actions">
            <button class="confirm-btn" onclick={confirmCompoundWorkshop}>
              Confirm Workshop ({compoundWorkshopSelectedIds.length} selected)
            </button>
            {#if compoundWorkshopSelectedIds.length === 0}
              <button class="confirm-btn skip-btn" onclick={skipCompoundWorkshop}>
                Skip Workshop
              </button>
            {/if}
          </div>
        {:else if pendingDestroyAbility.type === 'destroyCards'}
          <h3>Workshop — Select a card to destroy</h3>
          {#if currentPlayer.workshopCards.length > 0}
            <CardList
              cards={workshopAndWorkshopped}
              selectable={true}
              selectedIds={compoundDestroySelectedIds}
              rotatedIds={workshoppedIds}
              onCardClick={toggleCompoundDestroyCard}
            />
          {/if}
          <button class="confirm-btn" onclick={confirmCompoundDestroy}>
            {currentPlayer.workshopCards.length > 0
              ? `Confirm Destroy (${compoundDestroySelectedIds.length} selected)`
              : 'Confirm (nothing to destroy)'}
          </button>
        {/if}
      </div>
    {/if}

    <div class="sections">
      <div class="section">
        <h3>Drafted Cards <span class="hint">(click to destroy and activate ability)</span></h3>
        <CardList
          cards={draftCardOrder && actionState ? orderByDraftOrder(currentPlayer.draftedCards, draftCardOrder[actionState.currentPlayerIndex]) : currentPlayer.draftedCards}
          selectable={!hasPendingChoice && !hasPendingCompound}
          destroyingIds={pendingDestroyCard ? [pendingDestroyCard.instanceId] : []}
          onCardClick={handleDestroyDrafted}
        />
      </div>

      <div class="section" class:active-choice={workshopPendingChoice}>
        {#if topAbility?.type === 'workshop'}
          <h3>Workshop — Select cards ({topAbility.count} available)</h3>
          <CardList
            cards={workshopAndWorkshopped}
            selectable={true}
            selectedIds={selectedWorkshopIds}
            rotatedIds={workshoppedIds}
            onCardClick={toggleWorkshopCard}
          />
          {#if canUseReworkshopX2}
            <div class="reworkshop-x2-section">
              <span class="reworkshop-label">Use x2 w/ Glass Reworkshop:</span>
              <div class="reworkshop-cards">
                {#each currentPlayer.workshopCards as card}
                  {@const cardData = getAnyCardData(card.card)}
                  <button
                    class="glass-action-btn reworkshop-btn"
                    class:active={reworkshopCardId === card.instanceId}
                    onclick={() => {
                      if (reworkshopCardId === card.instanceId) {
                        reworkshopCardId = null;
                      } else {
                        reworkshopCardId = card.instanceId;
                        selectedWorkshopIds = selectedWorkshopIds.filter(id => id !== card.instanceId);
                      }
                    }}
                  >
                    {'name' in cardData ? cardData.name : card.card} x2
                  </button>
                {/each}
              </div>
            </div>
          {/if}
          <div class="workshop-actions">
            <button class="confirm-btn" onclick={confirmWorkshop}>
              Confirm Workshop ({(reworkshopCardId !== null ? 2 : 0) + selectedWorkshopIds.length} selected{reworkshopCardId !== null ? ', incl. x2' : ''})
            </button>
            {#if selectedWorkshopIds.length === 0 && reworkshopCardId === null}
              <button class="confirm-btn skip-btn" onclick={handleSkipWorkshop}>
                Skip Workshop
              </button>
            {/if}
            {#if glassReworkshopAvailable}
              <div class="reworkshop-section">
                <span class="reworkshop-label">Reworkshop (Glass):</span>
                <div class="reworkshop-cards">
                  {#each currentPlayer.workshoppedCards as card}
                    {@const cardData = getAnyCardData(card.card)}
                    <button
                      class="glass-action-btn reworkshop-btn"
                      onclick={() => onAction({ type: 'activateGlassReworkshop', card: card.card })}
                    >
                      {'name' in cardData ? cardData.name : card.card}
                    </button>
                  {/each}
                </div>
              </div>
            {/if}
          </div>
        {:else if topAbility?.type === 'destroyCards'}
          <h3>Workshop — Select a card to destroy</h3>
          <CardList
            cards={workshopAndWorkshopped}
            selectable={true}
            selectedIds={selectedDestroyIds}
            rotatedIds={workshoppedIds}
            onCardClick={toggleDestroyCard}
          />
          <button class="confirm-btn" onclick={confirmDestroy}>
            Confirm Destroy ({selectedDestroyIds.length} selected)
          </button>
        {:else}
          <h3>Workshop</h3>
          <CardList cards={workshopAndWorkshopped} rotatedIds={workshoppedIds} />
        {/if}
      </div>

      {#if gameState.expansions?.glass && availableGlass.length > 0 && !hasPendingChoice && !hasPendingCompound}
        <div class="section glass-section">
          <h3>Glass Abilities</h3>
          <div class="glass-buttons">
            {#each availableGlass as glass}
              {@const data = getGlassCardData(glass.card)}
              <button
                class="glass-action-btn"
                class:active={activeGlassPrompt === glass.card}
                onclick={() => handleGlassClick(glass.card)}
              >
                {data.name}
              </button>
            {/each}
          </div>

          {#if activeGlassPrompt === 'GlassExchange'}
            <div class="glass-sub-selection">
              <div class="sub-row">
                <span class="sub-label">Lose:</span>
                {#each exchangeLoseOptions as mat}
                  <button
                    class="sub-btn"
                    class:selected={glassExchangeLose === mat}
                    onclick={() => { glassExchangeLose = mat; glassExchangeGain = undefined; }}
                  >{mat}</button>
                {/each}
              </div>
              {#if glassExchangeLose}
                <div class="sub-row">
                  <span class="sub-label">Gain:</span>
                  {#each exchangeGainOptions as mat}
                    <button
                      class="sub-btn"
                      class:selected={glassExchangeGain === mat}
                      onclick={() => glassExchangeGain = mat}
                    >{mat}</button>
                  {/each}
                </div>
              {/if}
              <div class="sub-actions">
                <button class="confirm-btn" disabled={!glassExchangeLose || !glassExchangeGain} onclick={confirmExchange}>Confirm</button>
                <button class="confirm-btn skip-btn" onclick={cancelGlassPrompt}>Cancel</button>
              </div>
            </div>
          {:else if activeGlassPrompt === 'GlassMoveDrafted'}
            <div class="glass-sub-selection">
              <span class="sub-label">Select a drafted card to move to workshop:</span>
              <div class="sub-row">
                {#each currentPlayer.draftedCards as card}
                  {@const cardData = getAnyCardData(card.card)}
                  <button
                    class="sub-btn"
                    onclick={() => onAction({ type: 'activateGlassMoveDrafted', card: card.card })}
                  >{'name' in cardData ? cardData.name : card.card}</button>
                {/each}
              </div>
              <button class="confirm-btn skip-btn" onclick={cancelGlassPrompt}>Cancel</button>
            </div>
          {:else if activeGlassPrompt === 'GlassUnmix'}
            <div class="glass-sub-selection">
              <span class="sub-label">Select a non-primary color to unmix:</span>
              <div class="sub-row">
                {#each unmixableColors as color}
                  <button
                    class="color-btn"
                    style="background-color: {colorToHex(color)}; color: {textColorForBackground(colorToHex(color))}"
                    onclick={() => onAction({ type: 'activateGlassUnmix', color })}
                  >{color}</button>
                {/each}
              </div>
              <button class="confirm-btn skip-btn" onclick={cancelGlassPrompt}>Cancel</button>
            </div>
          {:else if activeGlassPrompt === 'GlassTertiaryDucat'}
            <div class="glass-sub-selection">
              <span class="sub-label">Select a tertiary color to convert to 1 ducat:</span>
              <div class="sub-row">
                {#each tertiaryWithCount as color}
                  <button
                    class="color-btn"
                    style="background-color: {colorToHex(color)}; color: {textColorForBackground(colorToHex(color))}"
                    onclick={() => onAction({ type: 'activateGlassTertiaryDucat', color })}
                  >{color}</button>
                {/each}
              </div>
              <button class="confirm-btn skip-btn" onclick={cancelGlassPrompt}>Cancel</button>
            </div>
          {:else if activeGlassPrompt === 'GlassReworkshop'}
            <div class="glass-sub-selection">
              <span class="sub-label">Select a workshopped card to un-rotate:</span>
              <div class="sub-row">
                {#each currentPlayer.workshoppedCards as card}
                  {@const cardData = getAnyCardData(card.card)}
                  <button
                    class="sub-btn"
                    onclick={() => onAction({ type: 'activateGlassReworkshop', card: card.card })}
                  >{'name' in cardData ? cardData.name : card.card}</button>
                {/each}
              </div>
              <button class="confirm-btn skip-btn" onclick={cancelGlassPrompt}>Cancel</button>
            </div>
          {:else if activeGlassPrompt === 'GlassDestroyClean'}
            <div class="glass-sub-selection">
              <span class="sub-label">Select a workshop card to destroy:</span>
              <div class="sub-row">
                {#each workshopAndWorkshopped as card}
                  {@const cardData = getAnyCardData(card.card)}
                  <button
                    class="sub-btn"
                    onclick={() => onAction({ type: 'activateGlassDestroyClean', card: card.card })}
                  >{'name' in cardData ? cardData.name : card.card}</button>
                {/each}
              </div>
              <button class="confirm-btn skip-btn" onclick={cancelGlassPrompt}>Cancel</button>
            </div>
          {/if}
        </div>
      {/if}
    </div>

    <div class="action-footer">
      <button class="undo-btn" onclick={onUndo} disabled={!undoAvailable}>
        Undo
      </button>
      <button
        class="end-turn-btn"
        onclick={handleEndTurn}
        disabled={hasPendingChoice || hasPendingCompound}
      >
        End Turn
      </button>
    </div>
  </div>
{/if}

<style>
  .action-phase {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
    padding: 0 0.25rem;
  }

  .phase-title {
    font-family: 'Cinzel', serif;
    color: #c9a84c;
    font-size: 0.8rem;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    text-align: center;
    margin: 0;
  }

  .queue-status {
    display: flex;
    flex-wrap: wrap;
    gap: 0.5rem;
    justify-content: center;
    font-size: 0.8rem;
  }

  .queue-info {
    color: #c9a84c;
    font-weight: 600;
  }

  .pending-info {
    color: #8b2020;
    font-weight: 600;
  }

  .sections {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
  }

  .section {
    border: 1px solid rgba(201, 168, 76, 0.4);
    border-radius: 8px;
    padding: 8px;
    background: rgba(20, 15, 10, 0.6);
    text-align: left;
  }

  .section h3 {
    font-family: 'Cinzel', serif;
    font-size: 0.75rem;
    color: #c9a84c;
    margin-bottom: 6px;
  }

  .hint {
    font-size: 0.65rem;
    color: rgba(245, 237, 224, 0.4);
    font-weight: 400;
  }

  .ability-prompt {
    border: 2px solid var(--accent-gold, #c9a84c);
    border-radius: 8px;
    padding: 0.5rem;
    background: rgba(201, 168, 76, 0.06);
    max-width: 100%;
    overflow-x: auto;
  }

  .compound-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
    margin-bottom: 8px;
  }

  .compound-title {
    font-family: 'Cinzel', serif;
    font-size: 0.85rem;
    font-weight: 600;
    color: #c9a84c;
  }

  .compound-cancel {
    width: auto;
    margin-top: 0;
    padding: 6px 14px;
    font-size: 0.75rem;
    min-height: unset;
  }

  .active-choice {
    border-color: #c9a84c;
    border-width: 2px;
    background: rgba(201, 168, 76, 0.1);
  }

  .confirm-btn {
    padding: 10px 16px;
    font-family: 'Cinzel', serif;
    font-weight: 600;
    font-size: 0.85rem;
    background: rgba(42, 107, 207, 0.8);
    color: #f5ede0;
    border: none;
    border-radius: 6px;
    cursor: pointer;
    margin-top: 6px;
    margin-right: 0;
    min-height: 44px;
    width: 100%;
  }

  .confirm-btn:hover:not(:disabled) {
    background: rgba(30, 86, 168, 0.9);
  }

  .confirm-btn:disabled {
    opacity: 0.4;
    cursor: not-allowed;
  }

  .skip-btn {
    background: rgba(100, 100, 100, 0.6);
  }

  .skip-btn:hover {
    background: rgba(80, 80, 80, 0.8);
  }

  .action-footer {
    display: flex;
    gap: 8px;
    padding-top: 4px;
  }

  .undo-btn {
    padding: 10px 16px;
    font-family: 'Cinzel', serif;
    font-size: 0.85rem;
    font-weight: 600;
    letter-spacing: 1px;
    background: rgba(20, 15, 10, 0.6);
    color: #c9a84c;
    border: 1px solid rgba(201, 168, 76, 0.4);
    border-radius: 8px;
    cursor: pointer;
    min-height: 44px;
    flex: 1;
  }

  .undo-btn:hover:not(:disabled) {
    background: rgba(40, 30, 20, 0.8);
  }

  .undo-btn:disabled {
    opacity: 0.4;
    cursor: not-allowed;
  }

  .end-turn-btn {
    padding: 10px 16px;
    font-family: 'Cinzel', serif;
    font-size: 0.85rem;
    font-weight: 600;
    letter-spacing: 1px;
    background: #8b2020;
    color: #f5ede0;
    border: none;
    border-radius: 8px;
    cursor: pointer;
    min-height: 44px;
    flex: 1;
  }

  .end-turn-btn:hover:not(:disabled) {
    background: #6b1818;
  }

  .end-turn-btn:disabled {
    opacity: 0.4;
    cursor: not-allowed;
  }

  /* Glass Abilities */
  .glass-section {
    border-color: rgba(100, 160, 200, 0.5);
  }

  .glass-buttons {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
    margin-bottom: 6px;
  }

  .glass-action-btn {
    padding: 10px 14px;
    font-family: 'Cormorant Garamond', serif;
    font-size: 0.85rem;
    font-weight: 600;
    background: rgba(100, 160, 200, 0.2);
    color: rgba(245, 237, 224, 0.9);
    border: 1px solid rgba(100, 160, 200, 0.5);
    border-radius: 6px;
    cursor: pointer;
    min-height: 44px;
  }

  .glass-action-btn:hover {
    background: rgba(100, 160, 200, 0.35);
  }

  .glass-action-btn.active {
    border-color: #c9a84c;
    background: rgba(201, 168, 76, 0.15);
  }

  .glass-sub-selection {
    display: flex;
    flex-direction: column;
    gap: 8px;
    padding: 8px;
    background: rgba(20, 15, 10, 0.5);
    border-radius: 6px;
  }

  .sub-label {
    font-family: 'Cormorant Garamond', serif;
    font-size: 0.85rem;
    color: rgba(245, 237, 224, 0.7);
  }

  .sub-row {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
    align-items: center;
  }

  .sub-btn {
    padding: 10px 12px;
    font-family: 'Cormorant Garamond', serif;
    font-size: 0.85rem;
    background: rgba(100, 160, 200, 0.15);
    color: rgba(245, 237, 224, 0.9);
    border: 1px solid rgba(100, 160, 200, 0.4);
    border-radius: 4px;
    cursor: pointer;
    min-height: 44px;
  }

  .sub-btn:hover {
    background: rgba(100, 160, 200, 0.3);
  }

  .sub-btn.selected {
    border-color: #c9a84c;
    background: rgba(201, 168, 76, 0.15);
  }

  .sub-actions {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
  }

  .color-btn {
    padding: 10px 14px;
    font-weight: 600;
    font-size: 0.85rem;
    border: 2px solid rgba(0, 0, 0, 0.2);
    border-radius: 6px;
    cursor: pointer;
    min-height: 44px;
    min-width: 44px;
  }

  .color-btn:hover {
    opacity: 0.85;
  }

  .workshop-actions {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .reworkshop-x2-section {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 6px;
    margin-bottom: 6px;
  }

  .reworkshop-section {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 6px;
    margin-left: 0;
  }

  .reworkshop-label {
    font-family: 'Cormorant Garamond', serif;
    font-size: 0.8rem;
    color: rgba(245, 237, 224, 0.6);
  }

  .reworkshop-cards {
    display: flex;
    flex-wrap: wrap;
    gap: 4px;
  }

  .reworkshop-btn {
    font-size: 0.8rem;
    padding: 8px 10px;
    min-height: 44px;
  }

  /* ── Responsive: sm (640px) ── */
  @media (min-width: 640px) {
    .action-phase {
      padding: 0;
    }

    .phase-title {
      font-size: 0.9rem;
      letter-spacing: 0.08em;
    }

    .section {
      padding: 10px 12px;
    }

    .section h3 {
      font-size: 0.8rem;
    }

    .hint {
      font-size: 0.7rem;
    }

    .confirm-btn {
      width: auto;
      padding: 8px 20px;
      margin-right: 6px;
      min-height: unset;
      font-size: inherit;
    }

    .action-footer {
      justify-content: center;
      gap: 12px;
    }

    .undo-btn,
    .end-turn-btn {
      flex: 0 1 auto;
      min-height: unset;
    }

    .undo-btn {
      padding: 8px 20px;
      font-size: 0.95rem;
    }

    .end-turn-btn {
      padding: 8px 28px;
      font-size: 0.95rem;
    }

    .glass-action-btn {
      padding: 6px 14px;
      min-height: unset;
    }

    .sub-btn {
      padding: 4px 12px;
      min-height: unset;
    }

    .color-btn {
      padding: 6px 14px;
      min-height: unset;
      min-width: unset;
    }

    .workshop-actions {
      flex-direction: row;
      flex-wrap: wrap;
      align-items: center;
    }

    .reworkshop-section {
      margin-left: 8px;
    }

    .reworkshop-btn {
      font-size: 0.75rem;
      padding: 4px 10px;
      min-height: unset;
    }
  }

  /* ── Responsive: md (768px) ── */
  @media (min-width: 768px) {
    .phase-title {
      font-size: 1rem;
      letter-spacing: 0.1em;
    }

    .sections {
      gap: 0.75rem;
    }

    .section h3 {
      font-size: 0.85rem;
    }
  }
</style>

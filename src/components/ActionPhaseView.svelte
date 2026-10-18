<script lang="ts">
  import type { GameState, Choice, Ability, Color, DucatPurchase } from '../data/types';
  import { canSell } from '../engine/wasmEngine';
  import { canMixAny, colorToHex, isPrimary, mixResult, mixWithWildPrimaries, textColorForBackground, WHEEL_ORDER } from '../data/colors';
  import { SECONDARIES } from '../data/cards';
  import { formatSingleAbility } from '../data/abilities';
  import { tableInteraction } from '../stores/tableInteraction';

  let { gameState, playerIndex, onAction, onUndo, undoAvailable }: {
    gameState: GameState;
    playerIndex: number;
    onAction: (choice: Choice) => void;
    // Absent where moves cannot be taken back.
    onUndo?: () => void;
    undoAvailable?: boolean;
  } = $props();

  const interaction = tableInteraction();

  let turn = $derived(gameState.phase.type === 'action' ? gameState.phase.actionState.turns[playerIndex] : null);
  let player = $derived(gameState.players[playerIndex]);
  let topAbility: Ability | null = $derived(turn?.abilityStack.at(-1) ?? null);
  let queued = $derived(turn ? turn.abilityStack.slice(0, -1).reverse() : []);

  // Choices half made on the board. Other players move all through this phase,
  // so these reset only when this player's own position changes.
  let selectedWorkshopIds: number[] = $state([]);
  let plannedMixes: [Color, Color][] = $state([]);
  let firstMixColor: Color | null = $state(null);
  let decisionKey = $derived(JSON.stringify([
    turn?.abilityStack, player.colorWheel, player.wildPrimaries, player.workshopCards.map(c => c.instanceId),
  ]));
  $effect(() => {
    decisionKey;
    selectedWorkshopIds = [];
    plannedMixes = [];
    firstMixColor = null;
  });

  // ── Workshop ──

  function toggleWorkshopCard(instanceId: number) {
    if (topAbility?.type !== 'workshop') return;
    if (selectedWorkshopIds.includes(instanceId)) {
      selectedWorkshopIds = selectedWorkshopIds.filter(id => id !== instanceId);
    } else if (selectedWorkshopIds.length < topAbility.count) {
      selectedWorkshopIds = [...selectedWorkshopIds, instanceId];
    } else if (topAbility.count === 1) {
      selectedWorkshopIds = [instanceId];
    }
  }

  function confirmWorkshop() {
    const cardTypes = selectedWorkshopIds.map(id => player.workshopCards.find(c => c.instanceId === id)!.card);
    onAction(cardTypes.length > 0 ? { type: 'workshop', cardTypes } : { type: 'skipWorkshop' });
  }

  function moveToDraftPool(instanceId: number) {
    const ci = player.workshopCards.find(c => c.instanceId === instanceId);
    if (ci) onAction({ type: 'moveToDraftPool', card: ci.card });
  }

  // ── Mixing ──

  // The wheel as the planned mixes would leave it.
  let mixedStock = $derived.by(() => {
    let stock = { wheel: { ...player.colorWheel }, wildPrimaries: player.wildPrimaries };
    for (const [a, b] of plannedMixes) {
      stock = mixWithWildPrimaries(stock.wheel, stock.wildPrimaries, a, b) ?? stock;
    }
    return stock;
  });

  function mixesWith(a: Color, b: Color): boolean {
    return mixWithWildPrimaries(mixedStock.wheel, mixedStock.wildPrimaries, a, b) !== null;
  }

  let mixesLeft = $derived(topAbility?.type === 'mixColors' ? topAbility.count - plannedMixes.length : 0);
  let partners = $derived(firstMixColor ? WHEEL_ORDER.filter(c => mixesWith(firstMixColor!, c)) : []);
  let canStartMix = $derived(WHEEL_ORDER.filter(a => WHEEL_ORDER.some(b => mixesWith(a, b))));

  function submitMixes(mixes: [Color, Color][]) {
    onAction({ type: 'mixAll', mixes });
  }

  function pickMixColor(color: Color) {
    if (firstMixColor === null) {
      firstMixColor = color;
    } else if (firstMixColor === color) {
      firstMixColor = null;
    } else if (mixesWith(firstMixColor, color)) {
      const mixes: [Color, Color][] = [...plannedMixes, [firstMixColor, color]];
      firstMixColor = null;
      if (topAbility?.type === 'mixColors' && mixes.length === topAbility.count) submitMixes(mixes);
      else plannedMixes = mixes;
    }
  }

  let wildPrimaryHint = $derived.by(() => {
    if (mixedStock.wildPrimaries === 0) return '';
    if (firstMixColor && isPrimary(firstMixColor) && mixedStock.wheel[firstMixColor] === 0) {
      return 'This mix spends a wild primary for the primary you lack.';
    }
    return `Your ${mixedStock.wildPrimaries} wild ${mixedStock.wildPrimaries === 1 ? 'primary stands' : 'primaries stand'} in for a Red, Yellow or Blue you lack.`;
  });

  // ── Selling ──

  let sellableBuyerIds = $derived(player.buyers.filter(b => canSell(player, b.instanceId)).map(b => b.instanceId));

  function sellTo(instanceId: number) {
    const buyer = player.buyers.find(b => b.instanceId === instanceId);
    if (buyer) onAction({ type: 'selectSellCard', sellCard: buyer.card });
  }

  // ── Between destroys ──

  function destroyDrafted(instanceId: number) {
    const ci = player.draftedCards.find(c => c.instanceId === instanceId);
    if (ci) onAction({ type: 'destroyDraftedCard', card: ci.card });
  }

  // Mirrors the engine: a purchase is offered only when it can act, and only
  // between destroys. See `action_phase::can_buy_with_ducat`.
  let purchasable = $derived.by((): DucatPurchase[] => {
    if (topAbility || player.ducats < 1) return [];
    const options: DucatPurchase[] = [];
    if (player.workshopCards.length > 0) options.push('workshop');
    if (canMixAny(player.colorWheel, player.wildPrimaries)) options.push('mixColors');
    if (sellableBuyerIds.length > 0) options.push('sell');
    return options;
  });

  const PURCHASE_LABELS: Record<DucatPurchase, string> = { workshop: 'Workshop ×1', mixColors: 'Mix ×1', sell: 'Sell' };

  function spendDucat(purchase: DucatPurchase) {
    onAction({ type: 'spendDucat', purchase });
  }

  // ── What the boards offer ──

  $effect(() => {
    interaction.seat = playerIndex;
    interaction.destroyDrafted = turn && !topAbility ? destroyDrafted : null;
    const movable = player.workshopCards.map(c => c.instanceId);
    interaction.workshop =
      topAbility?.type === 'workshop' ? { selectable: movable, selected: selectedWorkshopIds, onClick: toggleWorkshopCard }
      : topAbility?.type === 'moveToDraftPool' ? { selectable: movable, selected: [], onClick: moveToDraftPool }
      : null;
    interaction.buyers = topAbility?.type === 'sell' ? { selectable: sellableBuyerIds, onClick: sellTo } : null;
    if (topAbility?.type === 'mixColors') {
      interaction.wheel = {
        wheel: mixedStock.wheel,
        wildPrimaries: mixedStock.wildPrimaries,
        pickable: firstMixColor ? partners : canStartMix,
        selected: firstMixColor ? [firstMixColor] : [],
        mixesInto: firstMixColor ? Object.fromEntries(partners.map(c => [c, mixResult(firstMixColor!, c)])) : {},
        onClick: pickMixColor,
      };
    } else if (topAbility?.type === 'gainSecondary') {
      interaction.wheel = {
        wheel: player.colorWheel,
        wildPrimaries: player.wildPrimaries,
        pickable: SECONDARIES,
        selected: [],
        mixesInto: {},
        onClick: color => onAction({ type: 'gainSecondary', color }),
      };
    } else {
      interaction.wheel = null;
    }
    interaction.ducats = purchasable.length > 0 ? { available: purchasable, onSpend: spendDucat } : null;
  });

  $effect(() => () => {
    interaction.seat = -1;
    interaction.destroyDrafted = null;
    interaction.workshop = null;
    interaction.buyers = null;
    interaction.wheel = null;
    interaction.ducats = null;
  });
</script>

{#if turn}
  <div class="action-phase">
    <div class="panel-heading">Your turn</div>

    {#if topAbility === null}
      <p class="panel-text">
        {#if player.draftedCards.length > 0}
          <strong>Destroy a drafted card</strong> — click one sticking out of the top of your board — to use the
          ability on its banner. Cards you keep move into your workshop for next round.
        {:else}
          You have no drafted cards left to destroy.
        {/if}
      </p>
    {:else}
      <div class="prompt">
        <div class="prompt-title">{formatSingleAbility(topAbility)}</div>

        {#if topAbility.type === 'workshop'}
          <p class="panel-text">
            Choose up to {topAbility.count} {topAbility.count === 1 ? 'card' : 'cards'} along the bottom of your board.
            Each gives you what its lower half shows.
          </p>
          <div class="panel-actions">
            <button class="panel-btn primary" onclick={confirmWorkshop} disabled={selectedWorkshopIds.length === 0}>
              Workshop {selectedWorkshopIds.length || ''} {selectedWorkshopIds.length === 1 ? 'card' : 'cards'}
            </button>
            <button class="panel-btn" onclick={() => onAction({ type: 'skipWorkshop' })}>Skip</button>
          </div>
        {:else if topAbility.type === 'moveToDraftPool'}
          {#if player.workshopCards.length > 0}
            <p class="panel-text">
              Click a card in your workshop (bottom of your board) to move it up to your draft pool, where you can
              destroy it for its ability.
            </p>
          {:else}
            <p class="panel-text">Every card in your workshop has been used, so there is nothing to move.</p>
            <div class="panel-actions">
              <button class="panel-btn primary" onclick={() => onAction({ type: 'moveToDraftPool', card: null })}>Continue</button>
            </div>
          {/if}
        {:else if topAbility.type === 'mixColors'}
          <p class="panel-text">
            {#if firstMixColor}
              Now click a color that mixes with <strong>{firstMixColor}</strong> — the dashed outline shows what it makes.
              Click {firstMixColor} again to choose another.
            {:else if canStartMix.length > 0}
              Click two colors two steps apart on your wheel; they make the color between them.
              {mixesLeft} {mixesLeft === 1 ? 'mix' : 'mixes'} left.
            {:else}
              Nothing on your wheel can be mixed.
            {/if}
          </p>
          {#if wildPrimaryHint}<p class="panel-hint">{wildPrimaryHint}</p>{/if}
          {#if plannedMixes.length > 0}
            <ul class="planned-mixes">
              {#each plannedMixes as [a, b]}
                <li>
                  <span class="swatch" style:background={colorToHex(a)}></span>{a} +
                  <span class="swatch" style:background={colorToHex(b)}></span>{b} →
                  <span class="swatch" style:background={colorToHex(mixResult(a, b))}></span><strong>{mixResult(a, b)}</strong>
                </li>
              {/each}
            </ul>
          {/if}
          <div class="panel-actions">
            <button class="panel-btn primary" onclick={() => submitMixes(plannedMixes)}>
              {plannedMixes.length > 0 ? 'Done mixing' : 'Skip mixing'}
            </button>
            {#if plannedMixes.length > 0}
              <button class="panel-btn" onclick={() => { plannedMixes = plannedMixes.slice(0, -1); firstMixColor = null; }}>Undo last mix</button>
            {/if}
          </div>
        {:else if topAbility.type === 'sell'}
          <p class="panel-text">Click a glowing buyer on your board to sell to them.</p>
          {#if player.boats > 0 || player.wildPrimaries > 0}
            <p class="panel-hint">A boat covers a material you lack and a wild primary a primary you lack; what a buyer asks for is spent first.</p>
          {/if}
        {:else if topAbility.type === 'gainSecondary'}
          <p class="panel-text">Choose a secondary color to gain — here, or on a petal of your wheel.</p>
          <div class="panel-actions">
            {#each SECONDARIES as color}
              <button
                class="panel-btn color-btn"
                style:background={colorToHex(color)}
                style:color={textColorForBackground(colorToHex(color))}
                onclick={() => onAction({ type: 'gainSecondary', color })}
              >{color}</button>
            {/each}
          </div>
        {/if}
      </div>

      {#if queued.length > 0}
        <div class="queue">
          Then: {#each queued as ability, i}{#if i > 0}<span class="arrow">→</span>{/if}<span class="chip">{formatSingleAbility(ability)}</span>{/each}
        </div>
      {/if}
    {/if}

    {#if purchasable.length > 0}
      <div class="ducats">
        <span class="panel-hint">Spend 1 of your {player.ducats} {player.ducats === 1 ? 'ducat' : 'ducats'} (a point each) for:</span>
        <div class="panel-actions">
          {#each purchasable as purchase}
            <button class="panel-btn" onclick={() => spendDucat(purchase)}>{PURCHASE_LABELS[purchase]}</button>
          {/each}
        </div>
      </div>
    {/if}

    <div class="panel-actions footer">
      {#if onUndo}
        <button class="panel-btn" onclick={onUndo} disabled={!undoAvailable}>Undo</button>
      {/if}
      <button class="panel-btn danger" onclick={() => onAction({ type: 'endTurn' })} disabled={topAbility !== null}
        title={topAbility ? 'Finish the ability first' : 'End your turn'}>End turn</button>
    </div>
  </div>
{/if}

<style>
  .action-phase {
    display: flex;
    flex-direction: column;
    gap: 10px;
  }

  .prompt {
    display: flex;
    flex-direction: column;
    gap: 8px;
    padding: 10px 12px;
    border: 1px solid #c9a84c;
    border-radius: 8px;
    background: rgba(201, 168, 76, 0.1);
  }

  .prompt-title {
    font-family: 'Cinzel', serif;
    font-weight: 700;
    font-size: 1.05rem;
    color: #f3dc8a;
  }

  .planned-mixes {
    list-style: none;
    display: flex;
    flex-direction: column;
    gap: 4px;
    font-size: 0.95rem;
  }

  .swatch {
    display: inline-block;
    width: 12px;
    height: 12px;
    margin: 0 4px 0 2px;
    border-radius: 50%;
    border: 1px solid rgba(0, 0, 0, 0.5);
    vertical-align: -1px;
  }

  .queue {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 6px;
    font-size: 0.9rem;
    color: rgba(245, 237, 224, 0.7);
  }

  .chip {
    padding: 1px 8px;
    border-radius: 999px;
    border: 1px solid rgba(201, 168, 76, 0.5);
    font-family: 'Cinzel', serif;
    font-size: 0.75rem;
    color: #e8c766;
  }

  .arrow {
    color: rgba(201, 168, 76, 0.6);
  }

  .ducats {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .color-btn {
    border-color: rgba(0, 0, 0, 0.3);
    font-weight: 700;
  }

  .footer {
    justify-content: flex-end;
    padding-top: 4px;
    border-top: 1px solid rgba(201, 168, 76, 0.2);
  }
</style>

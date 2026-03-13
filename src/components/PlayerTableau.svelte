<script lang="ts">
  import type { PlayerState, CardInstance, SellCardInstance } from '../data/types';
  import { DEFAULT_ZONES, getZone, type ZoneConfig } from '../data/zoneConfig';
  import CardDisplay from './CardDisplay.svelte';
  import ColorWheelDisplay from './ColorWheelDisplay.svelte';

  let {
    player,
    draftedCards = [],
    workshopCards = [],
    completedSellCards = [],
    onDraftedCardClick,
    onWorkshopCardClick,
    selectedWorkshopIds = [],
    interactive = false,
  }: {
    player: PlayerState;
    draftedCards?: CardInstance[];
    workshopCards?: CardInstance[];
    completedSellCards?: SellCardInstance[];
    onDraftedCardClick?: (instanceId: number) => void;
    onWorkshopCardClick?: (instanceId: number) => void;
    selectedWorkshopIds?: number[];
    interactive?: boolean;
  } = $props();

  const zones = DEFAULT_ZONES;
  const draftSlots = ['draft1', 'draft2', 'draft3', 'draft4'] as const;

  const cwZone = getZone(zones, 'colorWheel');
  const sellCardsZone = getZone(zones, 'buyers');
  const wsZone = getZone(zones, 'workshop');
  const textilesZone = getZone(zones, 'materialTextiles');
  const ceramicsZone = getZone(zones, 'materialCeramics');
  const paintingsZone = getZone(zones, 'materialPaintings');
</script>

<div class="tableau">
  <!-- Color Wheel -->
  {#if cwZone}
    <div class="zone color-wheel-zone" style="left:{cwZone.x}%;top:{cwZone.y}%;width:{cwZone.width}%;height:{cwZone.height}%;">
      <ColorWheelDisplay wheel={player.colorWheel} size={300} hidden />
    </div>
  {/if}

  <!-- Draft Slots -->
  {#each draftSlots as slotId, i}
    {@const zone = getZone(zones, slotId)}
    {#if zone}
      <div class="zone draft-zone" style="left:{zone.x}%;top:{zone.y}%;width:{zone.width}%;height:{zone.height}%;">
        {#if draftedCards[i]}
          <CardDisplay
            card={draftedCards[i].card}
            selected={false}
            onclick={onDraftedCardClick ? () => onDraftedCardClick!(draftedCards[i].instanceId) : undefined}
          />
        {:else}
          <div class="empty-slot"></div>
        {/if}
      </div>
    {/if}
  {/each}

  <!-- Workshop Cards -->
  {#if wsZone}
    <div class="zone workshop-zone" style="left:{wsZone.x}%;top:{wsZone.y}%;width:{wsZone.width}%;height:{wsZone.height}%;">
      {#each workshopCards as card}
        <CardDisplay
          card={card.card}
          selected={selectedWorkshopIds.includes(card.instanceId)}
          onclick={onWorkshopCardClick ? () => onWorkshopCardClick(card.instanceId) : undefined}
        />
      {/each}
    </div>
  {/if}

  <!-- Completed Sell Cards -->
  {#if sellCardsZone && completedSellCards.length > 0}
    <div class="zone sell-cards-zone" style="left:{sellCardsZone.x}%;top:{sellCardsZone.y}%;width:{sellCardsZone.width}%;height:{sellCardsZone.height}%;">
      {#each completedSellCards as sellCard, i}
        <div class="sell-card-stack-item" style="top:{i * 30}px;">
          <CardDisplay card={sellCard.card} />
        </div>
      {/each}
    </div>
  {/if}

  <!-- Materials -->
  {#if textilesZone}
    <div class="zone material-zone" style="left:{textilesZone.x}%;top:{textilesZone.y}%;width:{textilesZone.width}%;height:{textilesZone.height}%;">
      <span class="material-count">{player.materials.Textiles}</span>
      <span class="material-label">Tex</span>
    </div>
  {/if}

  {#if ceramicsZone}
    <div class="zone material-zone" style="left:{ceramicsZone.x}%;top:{ceramicsZone.y}%;width:{ceramicsZone.width}%;height:{ceramicsZone.height}%;">
      <span class="material-count">{player.materials.Ceramics}</span>
      <span class="material-label">Cer</span>
    </div>
  {/if}

  {#if paintingsZone}
    <div class="zone material-zone" style="left:{paintingsZone.x}%;top:{paintingsZone.y}%;width:{paintingsZone.width}%;height:{paintingsZone.height}%;">
      <span class="material-count">{player.materials.Paintings}</span>
      <span class="material-label">Pnt</span>
    </div>
  {/if}
</div>

<style>
  .tableau {
    position: relative;
    aspect-ratio: 1328 / 800;
    background-image: url('/workshop.webp');
    background-size: 100% 100%;
    overflow: visible;
    border-radius: 8px;
    box-shadow: var(--shadow-panel, 0 2px 12px rgba(44, 30, 18, 0.1));
  }

  .zone {
    position: absolute;
    box-sizing: border-box;
  }

  .color-wheel-zone {
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .draft-zone {
    display: flex;
    align-items: stretch;
    justify-content: center;
  }

  .draft-zone :global(.card) {
    width: 100%;
    height: 100%;
    flex-shrink: 1;
  }

  .empty-slot {
    width: 100%;
    height: 100%;
    border: 2px dashed var(--border-gold, rgba(201, 168, 76, 0.3));
    border-radius: 6px;
    box-sizing: border-box;
  }

  .workshop-zone {
    display: flex;
    flex-direction: row;
    gap: 6px;
    align-items: flex-start;
    overflow-x: auto;
    overflow-y: hidden;
  }

  .sell-cards-zone {
    position: absolute;
    overflow: hidden;
  }

  .sell-card-stack-item {
    position: absolute;
    left: 0;
  }

  .material-zone {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    background: rgba(40, 28, 16, 0.5);
    border-radius: 6px;
  }

  .material-count {
    font-family: var(--font-display, 'Cinzel', serif);
    font-size: 1rem;
    font-weight: bold;
    color: var(--accent-gold, #c9a84c);
    text-shadow: 0 1px 3px rgba(0, 0, 0, 0.7);
  }

  .material-label {
    font-size: 0.5rem;
    color: #c4956a;
    text-transform: uppercase;
    letter-spacing: 0.5px;
    text-shadow: 0 1px 2px rgba(0, 0, 0, 0.6);
  }

  .tableau {
    width: 100%;
    max-width: 100%;
  }

  .workshop-zone {
    gap: 3px;
  }

  /* ===== RESPONSIVE OVERRIDES (mobile-first) ===== */

  @media (min-width: 768px) {
    .material-count {
      font-size: 1.2rem;
    }

    .material-label {
      font-size: 0.6rem;
    }

    .workshop-zone {
      gap: 6px;
    }
  }

  @media (min-width: 1024px) {
    .material-count {
      font-size: 1.4rem;
    }
  }
</style>

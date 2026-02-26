<script lang="ts">
  import type { DyeCardData, MaterialCardData, ActionCardData } from '../data/types';
  import {
    DYE_CARDS,
    DRAFT_MATERIAL_CARDS,
    MATERIAL_CARDS,
    BASIC_DYE_CARDS,
    ACTION_CARDS,
    CHALK_CARD,
    BUYER_CARDS,
    DRAFT_CARD_CATEGORIES,
    DRAFT_COPY_COUNTS,
  } from '../data/cards';
  import type { CardCategory } from '../data/cards';
  import { formatSingleAbility, formatAbility } from '../data/abilities';

  // Build lookup of all draft cards by name
  const allDraftCards = new Map<string, DyeCardData | MaterialCardData | ActionCardData>();
  for (const c of DYE_CARDS) allDraftCards.set(c.name, c);
  for (const c of DRAFT_MATERIAL_CARDS) allDraftCards.set(c.name, c);
  for (const c of ACTION_CARDS) allDraftCards.set(c.name, c);

  // Ability distribution across draft deck
  const abilityDistribution = new Map<string, number>();
  for (const card of DYE_CARDS) {
    const key = formatSingleAbility(card.ability);
    const copies = DRAFT_COPY_COUNTS[card.name] ?? 1;
    abilityDistribution.set(key, (abilityDistribution.get(key) ?? 0) + copies);
  }
  for (const card of DRAFT_MATERIAL_CARDS) {
    const key = formatSingleAbility(card.ability);
    const copies = DRAFT_COPY_COUNTS[card.name] ?? 1;
    abilityDistribution.set(key, (abilityDistribution.get(key) ?? 0) + copies);
  }
  for (const card of ACTION_CARDS) {
    const key = formatSingleAbility(card.ability);
    const copies = DRAFT_COPY_COUNTS[card.name] ?? 1;
    abilityDistribution.set(key, (abilityDistribution.get(key) ?? 0) + copies);
  }
  const abilitySorted = [...abilityDistribution.entries()].sort((a, b) => b[1] - a[1]);

  // Draft deck totals
  const draftTotalUnique = DRAFT_CARD_CATEGORIES.reduce((s, c) => s + c.cardNames.length, 0);
  const draftTotalCopies = DRAFT_CARD_CATEGORIES.reduce((s, c) => s + c.totalCopies, 0);

  // Buyer summary groups
  const buyerGroups = [
    {
      stars: 2,
      material: 'Textiles',
      description: '1 tertiary',
      count: BUYER_CARDS.filter(b => b.stars === 2 && b.colorCost.length === 1).length,
    },
    {
      stars: 2,
      material: 'Textiles',
      description: '1 secondary + 1 primary',
      count: BUYER_CARDS.filter(b => b.stars === 2 && b.colorCost.length === 2).length,
    },
    {
      stars: 3,
      material: 'Ceramics',
      description: '1 tertiary + 1 primary',
      count: BUYER_CARDS.filter(b => b.stars === 3).length,
    },
    {
      stars: 4,
      material: 'Paintings',
      description: '1 tertiary + 1 secondary',
      count: BUYER_CARDS.filter(b => b.stars === 4).length,
    },
  ];
  const buyerTotal = BUYER_CARDS.length;

  // Buyers grouped by star rating
  const buyersByStars = new Map<number, typeof BUYER_CARDS>();
  for (const b of BUYER_CARDS) {
    if (!buyersByStars.has(b.stars)) buyersByStars.set(b.stars, []);
    buyersByStars.get(b.stars)!.push(b);
  }
  const starGroups = [...buyersByStars.entries()].sort((a, b) => a[0] - b[0]);

  function getCategoryKind(cat: CardCategory): string {
    const first = allDraftCards.get(cat.cardNames[0]);
    return first?.kind ?? 'unknown';
  }

  function copiesPerCard(cat: CardCategory): number {
    return cat.cardNames.length > 0 ? cat.totalCopies / cat.cardNames.length : 0;
  }

  function formatMaterialTypes(card: MaterialCardData): string {
    if (card.materialTypes.length === 2 && card.materialTypes[0] === card.materialTypes[1]) {
      return `2x ${card.materialTypes[0]}`;
    }
    return card.materialTypes.join(' + ');
  }
</script>

<!-- Draft Deck Summary -->
<details open>
  <summary>Draft Deck Summary</summary>
  <table>
    <thead>
      <tr><th>Category</th><th>Unique Cards</th><th>Copies per Card</th><th>Total Copies</th></tr>
    </thead>
    <tbody>
      {#each DRAFT_CARD_CATEGORIES as cat}
        <tr>
          <td>{cat.label}</td>
          <td>{cat.cardNames.length}</td>
          <td>{copiesPerCard(cat)}</td>
          <td>{cat.totalCopies}</td>
        </tr>
      {/each}
      <tr class="total-row">
        <td><strong>Total</strong></td>
        <td><strong>{draftTotalUnique}</strong></td>
        <td></td>
        <td><strong>{draftTotalCopies}</strong></td>
      </tr>
    </tbody>
  </table>
</details>

<!-- Ability Distribution in Draft Deck -->
<details open>
  <summary>Ability Distribution in Draft Deck</summary>
  <table>
    <thead>
      <tr><th>Ability</th><th>Total Copies</th></tr>
    </thead>
    <tbody>
      {#each abilitySorted as [ability, count]}
        <tr>
          <td>{ability}</td>
          <td>{count}</td>
        </tr>
      {/each}
    </tbody>
  </table>
</details>

<!-- Buyer Summary -->
<details open>
  <summary>Buyer Summary</summary>
  <table>
    <thead>
      <tr><th>Stars</th><th>Material</th><th>Color Pattern</th><th>Count</th></tr>
    </thead>
    <tbody>
      {#each buyerGroups as group}
        <tr>
          <td>{group.stars}</td>
          <td>{group.material}</td>
          <td>{group.description}</td>
          <td>{group.count}</td>
        </tr>
      {/each}
      <tr class="total-row">
        <td colspan="3"><strong>Total</strong></td>
        <td><strong>{buyerTotal}</strong></td>
      </tr>
    </tbody>
  </table>
</details>

<!-- Per-category card tables -->
{#each DRAFT_CARD_CATEGORIES as cat}
  {@const kind = getCategoryKind(cat)}
  <details>
    <summary>{cat.label}</summary>
    {#if kind === 'dye'}
      <table>
        <thead>
          <tr><th>Name</th><th>Color Pips</th><th>Ability</th><th>Copies</th></tr>
        </thead>
        <tbody>
          {#each cat.cardNames as name}
            {@const card = allDraftCards.get(name) as DyeCardData}
            <tr>
              <td>{name}</td>
              <td>{card.colors.join(', ')}</td>
              <td>{formatSingleAbility(card.ability)}</td>
              <td>{DRAFT_COPY_COUNTS[name]}</td>
            </tr>
          {/each}
        </tbody>
      </table>
    {:else if kind === 'material'}
      <table>
        <thead>
          <tr><th>Name</th><th>Material Types</th><th>Color Pip</th><th>Ability</th><th>Copies</th></tr>
        </thead>
        <tbody>
          {#each cat.cardNames as name}
            {@const card = allDraftCards.get(name) as MaterialCardData}
            <tr>
              <td>{name}</td>
              <td>{formatMaterialTypes(card)}</td>
              <td>{card.colorPip ?? '—'}</td>
              <td>{formatSingleAbility(card.ability)}</td>
              <td>{DRAFT_COPY_COUNTS[name]}</td>
            </tr>
          {/each}
        </tbody>
      </table>
    {:else if kind === 'action'}
      <table>
        <thead>
          <tr><th>Name</th><th>Main Ability</th><th>Workshop Ability</th><th>Copies</th></tr>
        </thead>
        <tbody>
          {#each cat.cardNames as name}
            {@const card = allDraftCards.get(name) as ActionCardData}
            <tr>
              <td>{name}</td>
              <td>{formatSingleAbility(card.ability)}</td>
              <td>{card.workshopAbilities.map(formatSingleAbility).join(', ')}</td>
              <td>{DRAFT_COPY_COUNTS[name]}</td>
            </tr>
          {/each}
        </tbody>
      </table>
    {/if}
  </details>
{/each}

<!-- Starter Cards -->
<details>
  <summary>Starter Cards</summary>
  <table>
    <thead>
      <tr><th>Name</th><th>Type</th><th>Details</th><th>Ability</th></tr>
    </thead>
    <tbody>
      {#each BASIC_DYE_CARDS as card}
        <tr>
          <td>{card.name}</td>
          <td>Basic Dye</td>
          <td>{card.color}</td>
          <td>{formatSingleAbility(card.ability)}</td>
        </tr>
      {/each}
      {#each MATERIAL_CARDS as card}
        <tr>
          <td>{card.name}</td>
          <td>Material</td>
          <td>{card.materialTypes.join(', ')}</td>
          <td>{formatSingleAbility(card.ability)}</td>
        </tr>
      {/each}
      <tr>
        <td>{CHALK_CARD.name}</td>
        <td>Action</td>
        <td>Workshop: {CHALK_CARD.workshopAbilities.map(formatSingleAbility).join(', ')}</td>
        <td>{formatSingleAbility(CHALK_CARD.ability)}</td>
      </tr>
    </tbody>
  </table>
</details>

<!-- Buyers by Star Rating -->
<details>
  <summary>Buyers by Star Rating</summary>
  {#each starGroups as [stars, buyers]}
    <h3>{stars}-Star Buyers ({buyers.length})</h3>
    <table>
      <thead>
        <tr><th>Required Material</th><th>Color Cost</th></tr>
      </thead>
      <tbody>
        {#each buyers as buyer}
          <tr>
            <td>{buyer.requiredMaterial}</td>
            <td>{buyer.colorCost.join(', ')}</td>
          </tr>
        {/each}
      </tbody>
    </table>
  {/each}
</details>

<style>
  details {
    margin: 1rem 0;
    border: 1px solid #ddd;
    border-radius: 4px;
    padding: 0.5rem 1rem;
  }
  summary {
    cursor: pointer;
    font-weight: bold;
    font-size: 1.1rem;
    padding: 0.5rem 0;
  }
  table {
    width: 100%;
    border-collapse: collapse;
    margin: 0.5rem 0;
  }
  th, td {
    text-align: left;
    padding: 0.25rem 0.5rem;
    border-bottom: 1px solid #eee;
  }
  .total-row td {
    border-top: 2px solid #ccc;
  }
  h3 {
    margin-top: 1rem;
    margin-bottom: 0.5rem;
  }
</style>

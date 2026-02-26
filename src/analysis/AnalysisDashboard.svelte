<script lang="ts">
  import type { StructuredGameLog } from '../gameLog';
  import {
    computeActionDistribution,
    computeDestroyedFromDraft,
    computeDestroyedFromWorkshop,
    computeCardsAddedToDeck,
    computeBuyerAcquisitions,
    computeDeckSizeStats,
    computeScoreDistribution,
    computeWinRateByPosition,
    computeDraftFrequency,
    computeAverageGameLength,
    computeColorWheelStats,
    computeDurationStats,
    normalizeByDraftCopies,
    computeCategoryStats,
    computeDestroyRate,
    computeWinRateByVariant,
    wilsonConfidenceInterval,
  } from './logAnalysis';
  import { DRAFT_CARD_CATEGORIES, getStarterCardCategories } from '../data/cards';

  let { logs }: { logs: StructuredGameLog[] } = $props();

  let actionDist = $derived(computeActionDistribution(logs));
  let destroyedDraft = $derived(computeDestroyedFromDraft(logs));
  let destroyedWorkshop = $derived(computeDestroyedFromWorkshop(logs));
  let cardsAdded = $derived(computeCardsAddedToDeck(logs));
  let buyerAcq = $derived(computeBuyerAcquisitions(logs));
  let deckStats = $derived(computeDeckSizeStats(logs));
  let scoreDist = $derived(computeScoreDistribution(logs));
  let winRate = $derived(computeWinRateByPosition(logs));
  let draftFreq = $derived(computeDraftFrequency(logs));
  let gameLength = $derived(computeAverageGameLength(logs));
  let colorStats = $derived(computeColorWheelStats(logs));
  let durationStats = $derived(computeDurationStats(logs));
  let variantWinRate = $derived(computeWinRateByVariant(logs));

  let numPlayers = $derived(logs.length > 0 ? logs[0].playerNames.length : 2);
  let allCategories = $derived([...DRAFT_CARD_CATEGORIES, ...getStarterCardCategories(numPlayers)]);

  let draftFreqNormalized = $derived(normalizeByDraftCopies(draftFreq));
  let draftFreqCategories = $derived(computeCategoryStats(draftFreq, DRAFT_CARD_CATEGORIES));

  let cardsAddedNormalized = $derived(normalizeByDraftCopies(cardsAdded));
  let cardsAddedCategories = $derived(computeCategoryStats(cardsAdded, allCategories));

  let destroyRate = $derived(computeDestroyRate(destroyedDraft, draftFreq));
  let destroyRateCategories = $derived(computeCategoryStats(destroyedDraft, DRAFT_CARD_CATEGORIES));
  let destroyRateCategoryNormalized = $derived(
    destroyRateCategories.map(cat => {
      const draftCat = draftFreqCategories.find(d => d.label === cat.label);
      const draftRaw = draftCat?.rawTotal ?? 0;
      return {
        label: cat.label,
        rawTotal: cat.rawTotal,
        totalCopies: cat.totalCopies,
        normalizedRate: draftRaw > 0 ? cat.rawTotal / draftRaw : 0,
      };
    })
  );

  function sortedByValue(map: Map<string, number>): [string, number][] {
    return [...map.entries()].sort((a, b) => b[1] - a[1]);
  }

  function sortedByNumericKey(map: Map<number, number>): [number, number][] {
    return [...map.entries()].sort((a, b) => a[0] - b[0]);
  }

  function maxOfMap(map: Map<string | number, number>): number {
    let max = 0;
    for (const v of map.values()) if (v > max) max = v;
    return max;
  }
</script>

<!-- 1. Game Overview -->
<details open>
  <summary>Game Overview</summary>
  <div class="stat-grid">
    <div class="stat-card">
      <div class="value">{logs.length}</div>
      <div class="label">Games</div>
    </div>
    <div class="stat-card">
      <div class="value">{gameLength.avgRounds.toFixed(1)}</div>
      <div class="label">Avg Rounds</div>
    </div>
    <div class="stat-card">
      <div class="value">{gameLength.avgChoices.toFixed(1)}</div>
      <div class="label">Avg Choices</div>
    </div>
    <div class="stat-card">
      <div class="value">{deckStats.mean.toFixed(1)}</div>
      <div class="label">Avg Deck Size</div>
    </div>
    <div class="stat-card">
      <div class="value">{deckStats.median}</div>
      <div class="label">Median Deck Size</div>
    </div>
    <div class="stat-card">
      <div class="value">{deckStats.min} - {deckStats.max}</div>
      <div class="label">Deck Size Range</div>
    </div>
    {#if durationStats}
      <div class="stat-card">
        <div class="value">{(durationStats.avgMs / 1000).toFixed(1)}s</div>
        <div class="label">Avg Duration</div>
      </div>
      <div class="stat-card">
        <div class="value">{(durationStats.medianMs / 1000).toFixed(1)}s</div>
        <div class="label">Median Duration</div>
      </div>
    {/if}
  </div>
</details>

<!-- 2. Score Distribution -->
<details open>
  <summary>Score Distribution</summary>
  {#if scoreDist.size === 0}
    <p>No score data available.</p>
  {:else}
    {@const sorted = sortedByNumericKey(scoreDist)}
    {@const maxVal = maxOfMap(scoreDist)}
    <table>
      <thead>
        <tr><th>Score</th><th>Count</th></tr>
      </thead>
      <tbody>
        {#each sorted as [score, count]}
          <tr>
            <td>{score}</td>
            <td class="bar-cell">
              <div class="bar" style="width: {(count / maxVal) * 100}%"></div>
              <span>{count}</span>
            </td>
          </tr>
        {/each}
      </tbody>
    </table>
  {/if}
</details>

<!-- 3. Win Rate by Position -->
<details open>
  <summary>Win Rate by Position</summary>
  {#if winRate.size === 0}
    <p>No win rate data available.</p>
  {:else}
    {@const sorted = [...winRate.entries()].sort((a, b) => a[0] - b[0])}
    <table>
      <thead>
        <tr><th>Position</th><th>Wins</th><th>Games</th><th>Win %</th><th>95% CI</th></tr>
      </thead>
      <tbody>
        {#each sorted as [position, stats]}
          {@const ci = wilsonConfidenceInterval(stats.wins, stats.games)}
          <tr>
            <td>Player {position + 1}</td>
            <td>{Number.isInteger(stats.wins) ? stats.wins : stats.wins.toFixed(1)}</td>
            <td>{stats.games}</td>
            <td>{stats.games > 0 ? ((stats.wins / stats.games) * 100).toFixed(1) : '0.0'}%</td>
            <td>{ci ? `[${ci.lower.toFixed(1)}%, ${ci.upper.toFixed(1)}%]` : '–'}</td>
          </tr>
        {/each}
      </tbody>
    </table>
  {/if}
</details>

<!-- Win Rate by Variant -->
{#if variantWinRate}
  {@const sorted = [...variantWinRate.entries()].sort((a, b) => {
    const rateA = a[1].games > 0 ? a[1].wins / a[1].games : 0;
    const rateB = b[1].games > 0 ? b[1].wins / b[1].games : 0;
    return rateB - rateA;
  })}
  <details open>
    <summary>Win Rate by Variant</summary>
    <table>
      <thead>
        <tr><th>Variant</th><th>Wins</th><th>Games</th><th>Win %</th><th>95% CI</th></tr>
      </thead>
      <tbody>
        {#each sorted as [variant, stats]}
          {@const ci = wilsonConfidenceInterval(stats.wins, stats.games)}
          <tr>
            <td>{variant}</td>
            <td>{Number.isInteger(stats.wins) ? stats.wins : stats.wins.toFixed(1)}</td>
            <td>{stats.games}</td>
            <td>{stats.games > 0 ? ((stats.wins / stats.games) * 100).toFixed(1) : '0.0'}%</td>
            <td>{ci ? `[${ci.lower.toFixed(1)}%, ${ci.upper.toFixed(1)}%]` : '–'}</td>
          </tr>
        {/each}
      </tbody>
    </table>
  </details>
{/if}

<!-- 4. Action Distribution -->
<details>
  <summary>Action Distribution</summary>
  {#if actionDist.size === 0}
    <p>No action data available.</p>
  {:else}
    {@const sorted = sortedByValue(actionDist)}
    {@const maxVal = maxOfMap(actionDist)}
    <table>
      <thead>
        <tr><th>Action</th><th>Count</th></tr>
      </thead>
      <tbody>
        {#each sorted as [key, value]}
          <tr>
            <td>{key}</td>
            <td class="bar-cell">
              <div class="bar" style="width: {(value / maxVal) * 100}%"></div>
              <span>{value}</span>
            </td>
          </tr>
        {/each}
      </tbody>
    </table>
  {/if}
</details>

<!-- 5. Draft Frequency -->
<details>
  <summary>Draft Frequency</summary>
  {#if draftFreq.size === 0}
    <p>No draft data available.</p>
  {:else}
    <h3>By Category</h3>
    {@const catSorted = [...draftFreqCategories].sort((a, b) => b.normalizedRate - a.normalizedRate)}
    {@const catMax = Math.max(...draftFreqCategories.map(c => c.normalizedRate))}
    <table>
      <thead>
        <tr><th>Category</th><th>Drafts per Copy</th></tr>
      </thead>
      <tbody>
        {#each catSorted as cat}
          <tr>
            <td>{cat.label}</td>
            <td class="bar-cell">
              <div class="bar" style="width: {catMax > 0 ? (cat.normalizedRate / catMax) * 100 : 0}%"></div>
              <span>{cat.normalizedRate.toFixed(2)}</span>
            </td>
          </tr>
        {/each}
      </tbody>
    </table>

    <h3>By Card</h3>
    {@const sorted = sortedByValue(draftFreqNormalized)}
    {@const maxVal = maxOfMap(draftFreqNormalized)}
    <table>
      <thead>
        <tr><th>Card</th><th>Drafts per Copy</th></tr>
      </thead>
      <tbody>
        {#each sorted as [key, value]}
          <tr>
            <td>{key}</td>
            <td class="bar-cell">
              <div class="bar" style="width: {(value / maxVal) * 100}%"></div>
              <span>{value.toFixed(2)}</span>
            </td>
          </tr>
        {/each}
      </tbody>
    </table>
  {/if}
</details>

<!-- 6. Cards Added to Deck -->
<details>
  <summary>Cards Added to Deck</summary>
  {#if cardsAdded.size === 0}
    <p>No data available.</p>
  {:else}
    <h3>By Category</h3>
    {@const catSorted = [...cardsAddedCategories].sort((a, b) => b.normalizedRate - a.normalizedRate)}
    {@const catMax = Math.max(...cardsAddedCategories.map(c => c.normalizedRate))}
    <table>
      <thead>
        <tr><th>Category</th><th>Added per Copy</th></tr>
      </thead>
      <tbody>
        {#each catSorted as cat}
          <tr>
            <td>{cat.label}</td>
            <td class="bar-cell">
              <div class="bar" style="width: {catMax > 0 ? (cat.normalizedRate / catMax) * 100 : 0}%"></div>
              <span>{cat.normalizedRate.toFixed(2)}</span>
            </td>
          </tr>
        {/each}
      </tbody>
    </table>

    <h3>By Card</h3>
    {@const sorted = sortedByValue(cardsAddedNormalized)}
    {@const maxVal = maxOfMap(cardsAddedNormalized)}
    <table>
      <thead>
        <tr><th>Card</th><th>Added per Copy</th></tr>
      </thead>
      <tbody>
        {#each sorted as [key, value]}
          <tr>
            <td>{key}</td>
            <td class="bar-cell">
              <div class="bar" style="width: {(value / maxVal) * 100}%"></div>
              <span>{value.toFixed(2)}</span>
            </td>
          </tr>
        {/each}
      </tbody>
    </table>
  {/if}
</details>

<!-- 7. Destroyed from Draft -->
<details>
  <summary>Destroyed from Draft</summary>
  {#if destroyedDraft.size === 0}
    <p>No data available.</p>
  {:else}
    <h3>By Category</h3>
    {@const catSorted = [...destroyRateCategoryNormalized].sort((a, b) => b.normalizedRate - a.normalizedRate)}
    {@const catMax = Math.max(...destroyRateCategoryNormalized.map(c => c.normalizedRate))}
    <table>
      <thead>
        <tr><th>Category</th><th>Destroy Rate</th></tr>
      </thead>
      <tbody>
        {#each catSorted as cat}
          <tr>
            <td>{cat.label}</td>
            <td class="bar-cell">
              <div class="bar" style="width: {catMax > 0 ? (cat.normalizedRate / catMax) * 100 : 0}%"></div>
              <span>{(cat.normalizedRate * 100).toFixed(1)}%</span>
            </td>
          </tr>
        {/each}
      </tbody>
    </table>

    <h3>By Card</h3>
    {@const sorted = sortedByValue(destroyRate)}
    {@const maxVal = maxOfMap(destroyRate)}
    <table>
      <thead>
        <tr><th>Card</th><th>Destroy Rate</th></tr>
      </thead>
      <tbody>
        {#each sorted as [key, value]}
          <tr>
            <td>{key}</td>
            <td class="bar-cell">
              <div class="bar" style="width: {maxVal > 0 ? (value / maxVal) * 100 : 0}%"></div>
              <span>{(value * 100).toFixed(1)}%</span>
            </td>
          </tr>
        {/each}
      </tbody>
    </table>
  {/if}
</details>

<!-- 8. Destroyed from Workshop -->
<details>
  <summary>Destroyed from Workshop</summary>
  {#if destroyedWorkshop.size === 0}
    <p>No data available.</p>
  {:else}
    {@const sorted = sortedByValue(destroyedWorkshop)}
    {@const maxVal = maxOfMap(destroyedWorkshop)}
    <table>
      <thead>
        <tr><th>Card</th><th>Count</th></tr>
      </thead>
      <tbody>
        {#each sorted as [key, value]}
          <tr>
            <td>{key}</td>
            <td class="bar-cell">
              <div class="bar" style="width: {(value / maxVal) * 100}%"></div>
              <span>{value}</span>
            </td>
          </tr>
        {/each}
      </tbody>
    </table>
  {/if}
</details>

<!-- 9. Buyer Acquisitions -->
<details>
  <summary>Buyer Acquisitions</summary>

  <h3>By Buyer</h3>
  {#if buyerAcq.byBuyer.size === 0}
    <p>No buyer data available.</p>
  {:else}
    {@const sorted = sortedByValue(buyerAcq.byBuyer)}
    {@const maxVal = maxOfMap(buyerAcq.byBuyer)}
    <table>
      <thead>
        <tr><th>Buyer</th><th>Count</th></tr>
      </thead>
      <tbody>
        {#each sorted as [key, value]}
          <tr>
            <td>{key}</td>
            <td class="bar-cell">
              <div class="bar" style="width: {(value / maxVal) * 100}%"></div>
              <span>{value}</span>
            </td>
          </tr>
        {/each}
      </tbody>
    </table>
  {/if}

  <h3>By Stars</h3>
  {#if buyerAcq.byStars.size === 0}
    <p>No data available.</p>
  {:else}
    {@const sorted = [...buyerAcq.byStars.entries()].sort((a, b) => a[0] - b[0])}
    {@const maxVal = maxOfMap(buyerAcq.byStars)}
    <table>
      <thead>
        <tr><th>Stars</th><th>Count</th></tr>
      </thead>
      <tbody>
        {#each sorted as [stars, count]}
          <tr>
            <td>{stars}</td>
            <td class="bar-cell">
              <div class="bar" style="width: {(count / maxVal) * 100}%"></div>
              <span>{count}</span>
            </td>
          </tr>
        {/each}
      </tbody>
    </table>
  {/if}

  <h3>By Material</h3>
  {#if buyerAcq.byMaterial.size === 0}
    <p>No data available.</p>
  {:else}
    {@const sorted = sortedByValue(buyerAcq.byMaterial)}
    {@const maxVal = maxOfMap(buyerAcq.byMaterial)}
    <table>
      <thead>
        <tr><th>Material</th><th>Count</th></tr>
      </thead>
      <tbody>
        {#each sorted as [key, value]}
          <tr>
            <td>{key}</td>
            <td class="bar-cell">
              <div class="bar" style="width: {(value / maxVal) * 100}%"></div>
              <span>{value}</span>
            </td>
          </tr>
        {/each}
      </tbody>
    </table>
  {/if}
</details>

<!-- 10. Color Wheel at End -->
<details>
  <summary>Color Wheel at End</summary>
  {#if colorStats.size === 0}
    <p>No color data available.</p>
  {:else}
    {@const sorted = sortedByValue(colorStats)}
    {@const maxVal = maxOfMap(colorStats)}
    <table>
      <thead>
        <tr><th>Color</th><th>Average Count</th></tr>
      </thead>
      <tbody>
        {#each sorted as [color, avg]}
          <tr>
            <td>{color}</td>
            <td class="bar-cell">
              <div class="bar" style="width: {maxVal > 0 ? (avg / maxVal) * 100 : 0}%"></div>
              <span>{avg.toFixed(2)}</span>
            </td>
          </tr>
        {/each}
      </tbody>
    </table>
  {/if}
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
  .bar-cell {
    position: relative;
    min-width: 200px;
  }
  .bar {
    position: absolute;
    left: 0;
    top: 0;
    bottom: 0;
    background: #4a9eff33;
    border-radius: 2px;
  }
  .bar-cell span {
    position: relative;
    z-index: 1;
  }
  .stat-grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(200px, 1fr));
    gap: 1rem;
    margin: 1rem 0;
  }
  .stat-card {
    background: #f8f9fa;
    padding: 1rem;
    border-radius: 4px;
    text-align: center;
  }
  .stat-card .value {
    font-size: 2rem;
    font-weight: bold;
  }
  .stat-card .label {
    color: #666;
    font-size: 0.9rem;
  }
  h3 {
    margin-top: 1rem;
    margin-bottom: 0.5rem;
  }
</style>

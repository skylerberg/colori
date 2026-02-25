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
  } from './logAnalysis';

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
        <tr><th>Position</th><th>Wins</th><th>Games</th><th>Win %</th></tr>
      </thead>
      <tbody>
        {#each sorted as [position, stats]}
          <tr>
            <td>Player {position + 1}</td>
            <td>{stats.wins}</td>
            <td>{stats.games}</td>
            <td>{stats.games > 0 ? ((stats.wins / stats.games) * 100).toFixed(1) : '0.0'}%</td>
          </tr>
        {/each}
      </tbody>
    </table>
  {/if}
</details>

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
    {@const sorted = sortedByValue(draftFreq)}
    {@const maxVal = maxOfMap(draftFreq)}
    <table>
      <thead>
        <tr><th>Card</th><th>Times Drafted</th></tr>
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

<!-- 6. Cards Added to Deck -->
<details>
  <summary>Cards Added to Deck</summary>
  {#if cardsAdded.size === 0}
    <p>No data available.</p>
  {:else}
    {@const sorted = sortedByValue(cardsAdded)}
    {@const maxVal = maxOfMap(cardsAdded)}
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

<!-- 7. Destroyed from Draft -->
<details>
  <summary>Destroyed from Draft</summary>
  {#if destroyedDraft.size === 0}
    <p>No data available.</p>
  {:else}
    {@const sorted = sortedByValue(destroyedDraft)}
    {@const maxVal = maxOfMap(destroyedDraft)}
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

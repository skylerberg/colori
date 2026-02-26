import type { StructuredGameLog, FinalPlayerStats } from '../gameLog';
import type { AnyCardData } from '../data/types';
import { getAnyCardData } from '../data/cards';

function getCardName(card: string): string {
  const data = getAnyCardData(card);
  if (!data) return card;
  if (data.kind === 'buyer') {
    const matLabel = data.requiredMaterial;
    const colorLabels = data.colorCost.join(', ');
    return `${data.stars}-star ${matLabel} [${colorLabels}]`;
  }
  return data.name;
}

function buildCardInstanceMap(log: StructuredGameLog): Map<number, { instanceId: number; card: string }> {
  const map = new Map<number, { instanceId: number; card: string }>();
  const addCards = (cards: { instanceId: number; card: string }[]) => {
    for (const c of cards) map.set(c.instanceId, c);
  };
  const state = log.initialState;
  for (const p of state.players) {
    addCards(p.deck);
    addCards(p.discard);
    addCards(p.workshopCards);
    addCards(p.draftedCards);
  }
  addCards(state.draftDeck);
  addCards(state.destroyedPile);
  return map;
}

function buildBuyerInstanceMap(log: StructuredGameLog): Map<number, { instanceId: number; card: string }> {
  const map = new Map<number, { instanceId: number; card: string }>();
  const addBuyers = (buyers: { instanceId: number; card: string }[]) => {
    for (const b of buyers) map.set(b.instanceId, b);
  };
  const state = log.initialState;
  for (const p of state.players) {
    addBuyers(p.completedBuyers);
  }
  addBuyers(state.buyerDeck);
  addBuyers(state.buyerDisplay);
  return map;
}

export function computeActionDistribution(logs: StructuredGameLog[]): Map<string, number> {
  const counts = new Map<string, number>();
  for (const log of logs) {
    for (const entry of log.entries) {
      const type = entry.choice.type;
      counts.set(type, (counts.get(type) ?? 0) + 1);
    }
  }
  return counts;
}

export function computeDestroyedFromDraft(logs: StructuredGameLog[]): Map<string, number> {
  const counts = new Map<string, number>();
  for (const log of logs) {
    const instanceMap = buildCardInstanceMap(log);
    for (const entry of log.entries) {
      if (entry.choice.type === 'destroyDraftedCard' || entry.choice.type === 'destroyAndMixAll' || entry.choice.type === 'destroyAndSell') {
        const inst = instanceMap.get(entry.choice.cardInstanceId);
        if (inst) {
          const name = getCardName(inst.card);
          counts.set(name, (counts.get(name) ?? 0) + 1);
        }
      }
    }
  }
  return counts;
}

export function computeDestroyedFromWorkshop(logs: StructuredGameLog[]): Map<string, number> {
  const counts = new Map<string, number>();
  for (const log of logs) {
    const instanceMap = buildCardInstanceMap(log);
    for (const entry of log.entries) {
      if (entry.choice.type === 'destroyDrawnCards') {
        for (const id of entry.choice.cardInstanceIds) {
          const inst = instanceMap.get(id);
          if (inst) {
            const name = getCardName(inst.card);
            counts.set(name, (counts.get(name) ?? 0) + 1);
          }
        }
      }
    }
  }
  return counts;
}

export function computeCardsAddedToDeck(logs: StructuredGameLog[]): Map<string, number> {
  const counts = new Map<string, number>();
  for (const log of logs) {
    const instanceMap = buildCardInstanceMap(log);
    const drafted = new Map<number, string>();
    const destroyed = new Set<number>();

    for (const entry of log.entries) {
      if (entry.choice.type === 'draftPick') {
        const inst = instanceMap.get(entry.choice.cardInstanceId);
        if (inst) {
          drafted.set(entry.choice.cardInstanceId, getCardName(inst.card));
        }
      } else if (entry.choice.type === 'destroyDraftedCard' || entry.choice.type === 'destroyAndMixAll' || entry.choice.type === 'destroyAndSell') {
        destroyed.add(entry.choice.cardInstanceId);
      }
    }

    for (const [id, name] of drafted) {
      if (!destroyed.has(id)) {
        counts.set(name, (counts.get(name) ?? 0) + 1);
      }
    }
  }
  return counts;
}

export function computeBuyerAcquisitions(logs: StructuredGameLog[]): {
  byBuyer: Map<string, number>;
  byStars: Map<number, number>;
  byMaterial: Map<string, number>;
} {
  const byBuyer = new Map<string, number>();
  const byStars = new Map<number, number>();
  const byMaterial = new Map<string, number>();

  for (const log of logs) {
    const instanceMap = buildBuyerInstanceMap(log);
    for (const entry of log.entries) {
      if (entry.choice.type === 'selectBuyer' || entry.choice.type === 'destroyAndSell') {
        const inst = instanceMap.get(entry.choice.buyerInstanceId);
        if (inst) {
          const data = getAnyCardData(inst.card);
          if (data && data.kind === 'buyer') {
            const name = getCardName(inst.card);
            byBuyer.set(name, (byBuyer.get(name) ?? 0) + 1);
            byStars.set(data.stars, (byStars.get(data.stars) ?? 0) + 1);
            byMaterial.set(data.requiredMaterial, (byMaterial.get(data.requiredMaterial) ?? 0) + 1);
          }
        }
      }
    }
  }

  return { byBuyer, byStars, byMaterial };
}

export function computeDeckSizeStats(logs: StructuredGameLog[]): {
  mean: number;
  median: number;
  min: number;
  max: number;
} {
  const sizes: number[] = [];
  for (const log of logs) {
    if (log.finalPlayerStats) {
      for (const ps of log.finalPlayerStats) {
        sizes.push(ps.deckSize);
      }
    }
  }

  if (sizes.length === 0) {
    return { mean: 0, median: 0, min: 0, max: 0 };
  }

  sizes.sort((a, b) => a - b);
  const sum = sizes.reduce((a, b) => a + b, 0);
  const mean = sum / sizes.length;
  const mid = Math.floor(sizes.length / 2);
  const median = sizes.length % 2 === 0
    ? (sizes[mid - 1] + sizes[mid]) / 2
    : sizes[mid];

  return {
    mean,
    median,
    min: sizes[0],
    max: sizes[sizes.length - 1],
  };
}

export function computeScoreDistribution(logs: StructuredGameLog[]): Map<number, number> {
  const counts = new Map<number, number>();
  for (const log of logs) {
    if (log.finalScores) {
      for (const fs of log.finalScores) {
        counts.set(fs.score, (counts.get(fs.score) ?? 0) + 1);
      }
    }
  }
  return counts;
}

export function computeWinRateByPosition(logs: StructuredGameLog[]): Map<number, { wins: number; games: number }> {
  const stats = new Map<number, { wins: number; games: number }>();

  for (const log of logs) {
    if (!log.finalScores) continue;
    const numPlayers = log.finalScores.length;

    // Initialize positions for this game
    for (let i = 0; i < numPlayers; i++) {
      if (!stats.has(i)) {
        stats.set(i, { wins: 0, games: 0 });
      }
      stats.get(i)!.games++;
    }

    // Find highest score
    let maxScore = -Infinity;
    for (const fs of log.finalScores) {
      if (fs.score > maxScore) maxScore = fs.score;
    }

    // Match final scores back to player indices by name
    for (let i = 0; i < log.playerNames.length; i++) {
      const playerName = log.playerNames[i];
      const scoreEntry = log.finalScores.find(fs => fs.name === playerName);
      if (scoreEntry && scoreEntry.score === maxScore) {
        stats.get(i)!.wins++;
      }
    }
  }

  return stats;
}

export function computeDraftFrequency(logs: StructuredGameLog[]): Map<string, number> {
  const counts = new Map<string, number>();
  for (const log of logs) {
    const instanceMap = buildCardInstanceMap(log);
    for (const entry of log.entries) {
      if (entry.choice.type === 'draftPick') {
        const inst = instanceMap.get(entry.choice.cardInstanceId);
        if (inst) {
          const name = getCardName(inst.card);
          counts.set(name, (counts.get(name) ?? 0) + 1);
        }
      }
    }
  }
  return counts;
}

export function computeAverageGameLength(logs: StructuredGameLog[]): {
  avgRounds: number;
  avgChoices: number;
} {
  if (logs.length === 0) {
    return { avgRounds: 0, avgChoices: 0 };
  }

  let totalRounds = 0;
  let totalChoices = 0;

  for (const log of logs) {
    let maxRound = 0;
    for (const entry of log.entries) {
      if (entry.round > maxRound) maxRound = entry.round;
    }
    totalRounds += maxRound;
    totalChoices += log.entries.length;
  }

  return {
    avgRounds: totalRounds / logs.length,
    avgChoices: totalChoices / logs.length,
  };
}

export function computeDurationStats(logs: StructuredGameLog[]): {
  avgMs: number;
  medianMs: number;
  minMs: number;
  maxMs: number;
} | null {
  const durations: number[] = [];
  for (const log of logs) {
    if (log.durationMs != null) {
      durations.push(log.durationMs);
    }
  }

  if (durations.length === 0) return null;

  durations.sort((a, b) => a - b);
  const sum = durations.reduce((a, b) => a + b, 0);
  const avgMs = sum / durations.length;
  const mid = Math.floor(durations.length / 2);
  const medianMs = durations.length % 2 === 0
    ? (durations[mid - 1] + durations[mid]) / 2
    : durations[mid];

  return {
    avgMs,
    medianMs,
    minMs: durations[0],
    maxMs: durations[durations.length - 1],
  };
}

export function computeColorWheelStats(logs: StructuredGameLog[]): Map<string, number> {
  const totals = new Map<string, number>();
  let playerCount = 0;

  for (const log of logs) {
    if (!log.finalPlayerStats) continue;
    for (const ps of log.finalPlayerStats) {
      playerCount++;
      for (const [color, count] of Object.entries(ps.colorWheel)) {
        totals.set(color, (totals.get(color) ?? 0) + count);
      }
    }
  }

  const averages = new Map<string, number>();
  if (playerCount > 0) {
    for (const [color, total] of totals) {
      averages.set(color, total / playerCount);
    }
  }

  return averages;
}

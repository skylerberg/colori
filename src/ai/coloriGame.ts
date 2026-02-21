import type { Game, GameStatus } from './ismcts';
import type {
  GameState, PlayerState, CardInstance, Color, GarmentCard,
  DraftState, ActionState, Ability, PendingChoice, GamePhase,
} from '../data/types';
import { getCardPips } from '../data/cards';
import { ALL_COLORS, canMix } from '../data/colors';
import { canPayCost } from '../engine/colorWheel';
import { playerPick, confirmPass } from '../engine/draftPhase';
import { executeDrawPhase } from '../engine/drawPhase';
import {
  destroyDraftedCard, endPlayerTurn, resolveStoreColors,
  resolveMixColors, skipMix, resolveDestroyCards,
  resolveChooseGarment, resolveGarmentPayment,
} from '../engine/actionPhase';
import { calculateScore } from '../engine/scoring';
import { shuffle } from '../engine/deckUtils';

// ── Choice type ──

export type ColoriChoice =
  | { type: 'draftPick'; cardInstanceId: number }
  | { type: 'destroyDraftedCard'; cardInstanceId: number }
  | { type: 'endTurn' }
  | { type: 'storeColors'; cardInstanceIds: number[] }
  | { type: 'destroyDrawnCards'; cardInstanceIds: number[] }
  | { type: 'mix'; colorA: Color; colorB: Color }
  | { type: 'skipMix' }
  | { type: 'chooseGarment'; garmentInstanceId: number }
  | { type: 'garmentPayment'; fabricCardId: number };

// ── Deep clone ──

function clonePlayerState(p: PlayerState): PlayerState {
  return {
    name: p.name,
    deck: [...p.deck],
    discard: [...p.discard],
    drawnCards: [...p.drawnCards],
    draftedCards: [...p.draftedCards],
    colorWheel: { ...p.colorWheel },
    completedGarments: [...p.completedGarments],
  };
}

function clonePhase(phase: GamePhase): GamePhase {
  switch (phase.type) {
    case 'draw':
      return { type: 'draw' };
    case 'draft': {
      const ds = phase.draftState;
      return {
        type: 'draft',
        draftState: {
          pickNumber: ds.pickNumber,
          currentPlayerIndex: ds.currentPlayerIndex,
          hands: ds.hands.map(h => [...h]),
          direction: ds.direction,
          waitingForPass: ds.waitingForPass,
        },
      };
    }
    case 'action': {
      const as_ = phase.actionState;
      return {
        type: 'action',
        actionState: {
          currentPlayerIndex: as_.currentPlayerIndex,
          abilityQueue: as_.abilityQueue.map(a => ({ ...a })),
          pendingChoice: as_.pendingChoice ? { ...as_.pendingChoice } : null,
        },
      };
    }
    case 'gameOver':
      return { type: 'gameOver' };
  }
}

export function cloneGameState(state: GameState): GameState {
  return {
    players: state.players.map(clonePlayerState),
    draftDeck: [...state.draftDeck],
    destroyedPile: [...state.destroyedPile],
    garmentDeck: [...state.garmentDeck],
    garmentDisplay: [...state.garmentDisplay],
    phase: clonePhase(state.phase),
    round: state.round,
    aiPlayers: [...state.aiPlayers],
  };
}

// ── Subset enumeration ──

function getSubsets(items: number[], maxSize: number): number[][] {
  const result: number[][] = [];
  function recurse(start: number, current: number[]) {
    if (current.length > 0) result.push([...current]);
    if (current.length >= maxSize) return;
    for (let i = start; i < items.length; i++) {
      current.push(items[i]);
      recurse(i + 1, current);
      current.pop();
    }
  }
  recurse(0, []);
  return result;
}

// ── Garment affordability helpers ──

function canAffordGarment(
  player: PlayerState,
  garment: GarmentCard,
): boolean {
  const hasFabric = player.drawnCards.some(
    c => c.card.kind === 'fabric' && c.card.fabricType === garment.requiredFabric,
  );
  if (!hasFabric) return false;
  return canPayCost(player.colorWheel, garment.colorCost);
}

// ── Choice enumeration ──

function enumerateChoices(state: GameState): ColoriChoice[] {
  const phase = state.phase;

  if (phase.type === 'draft') {
    const ds = phase.draftState;
    if (ds.waitingForPass) return [];
    const hand = ds.hands[ds.currentPlayerIndex];
    return hand.map(c => ({ type: 'draftPick' as const, cardInstanceId: c.instanceId }));
  }

  if (phase.type === 'action') {
    const as_ = phase.actionState;
    const player = state.players[as_.currentPlayerIndex];
    const pending = as_.pendingChoice;

    if (pending === null) {
      const choices: ColoriChoice[] = player.draftedCards.map(c => ({
        type: 'destroyDraftedCard' as const,
        cardInstanceId: c.instanceId,
      }));
      choices.push({ type: 'endTurn' });
      return choices;
    }

    switch (pending.type) {
      case 'chooseCardsForStore': {
        const cardsWithPips = player.drawnCards
          .filter(c => getCardPips(c.card).length > 0)
          .map(c => c.instanceId);
        const storeSubsets = getSubsets(cardsWithPips, pending.count)
          .map(ids => ({ type: 'storeColors' as const, cardInstanceIds: ids }));
        // If no cards with pips, must still resolve with empty selection
        if (storeSubsets.length === 0) {
          return [{ type: 'storeColors' as const, cardInstanceIds: [] }];
        }
        return storeSubsets;
      }
      case 'chooseCardsToDestroy': {
        const cardIds = player.drawnCards.map(c => c.instanceId);
        const destroySubsets = getSubsets(cardIds, pending.count)
          .map(ids => ({ type: 'destroyDrawnCards' as const, cardInstanceIds: ids }));
        // If no drawn cards, must still resolve with empty selection
        if (destroySubsets.length === 0) {
          return [{ type: 'destroyDrawnCards' as const, cardInstanceIds: [] }];
        }
        return destroySubsets;
      }
      case 'chooseMix': {
        const choices: ColoriChoice[] = [{ type: 'skipMix' }];
        for (let i = 0; i < ALL_COLORS.length; i++) {
          for (let j = i + 1; j < ALL_COLORS.length; j++) {
            const a = ALL_COLORS[i];
            const b = ALL_COLORS[j];
            if (player.colorWheel[a] > 0 && player.colorWheel[b] > 0 && canMix(a, b)) {
              choices.push({ type: 'mix', colorA: a, colorB: b });
            }
          }
        }
        return choices;
      }
      case 'chooseGarment': {
        return state.garmentDisplay
          .filter(g => canAffordGarment(player, g.card))
          .map(g => ({ type: 'chooseGarment' as const, garmentInstanceId: g.instanceId }));
      }
      case 'chooseGarmentPayment': {
        const garmentInst = state.garmentDisplay.find(
          g => g.instanceId === pending.garmentInstanceId,
        );
        if (!garmentInst) return [];
        const garment = garmentInst.card;
        const choices: ColoriChoice[] = [];

        for (const fabric of player.drawnCards) {
          if (fabric.card.kind !== 'fabric' || fabric.card.fabricType !== garment.requiredFabric) continue;

          if (canPayCost(player.colorWheel, garment.colorCost)) {
            choices.push({
              type: 'garmentPayment',
              fabricCardId: fabric.instanceId,
            });
          }
        }
        return choices;
      }
    }
  }

  return [];
}

// ── Choice key ──

function choiceToKey(choice: ColoriChoice): string {
  switch (choice.type) {
    case 'draftPick':
      return `draftPick:${choice.cardInstanceId}`;
    case 'destroyDraftedCard':
      return `destroyDrafted:${choice.cardInstanceId}`;
    case 'endTurn':
      return 'endTurn';
    case 'storeColors':
      return `storeColors:${[...choice.cardInstanceIds].sort((a, b) => a - b).join(',')}`;
    case 'destroyDrawnCards':
      return `destroyDrawn:${[...choice.cardInstanceIds].sort((a, b) => a - b).join(',')}`;
    case 'mix':
      return `mix:${choice.colorA}:${choice.colorB}`;
    case 'skipMix':
      return 'skipMix';
    case 'chooseGarment':
      return `chooseGarment:${choice.garmentInstanceId}`;
    case 'garmentPayment':
      return `garmentPayment:${choice.fabricCardId}`;
  }
}

// ── Apply choice ──

function applyChoiceToState(state: GameState, choice: ColoriChoice): void {
  switch (choice.type) {
    case 'draftPick':
      playerPick(state, choice.cardInstanceId);
      // Auto-confirm pass (UI-only concern)
      if (state.phase.type === 'draft' && state.phase.draftState.waitingForPass) {
        confirmPass(state);
      }
      break;
    case 'destroyDraftedCard':
      destroyDraftedCard(state, choice.cardInstanceId);
      break;
    case 'endTurn':
      endPlayerTurn(state);
      // Auto-execute draw phase if transitioning
      if (state.phase.type === 'draw') {
        executeDrawPhase(state);
      }
      break;
    case 'storeColors':
      resolveStoreColors(state, choice.cardInstanceIds);
      break;
    case 'destroyDrawnCards':
      resolveDestroyCards(state, choice.cardInstanceIds);
      break;
    /* empty selections are valid — the engine handles [] gracefully */
    case 'mix':
      resolveMixColors(state, choice.colorA, choice.colorB);
      break;
    case 'skipMix':
      skipMix(state);
      break;
    case 'chooseGarment':
      resolveChooseGarment(state, choice.garmentInstanceId);
      break;
    case 'garmentPayment':
      resolveGarmentPayment(state, choice.fabricCardId);
      break;
  }
}

// ── Choice availability ──

function checkChoiceAvailable(state: GameState, choice: ColoriChoice): boolean {
  switch (choice.type) {
    case 'draftPick': {
      if (state.phase.type !== 'draft') return false;
      const ds = state.phase.draftState;
      return ds.hands[ds.currentPlayerIndex].some(c => c.instanceId === choice.cardInstanceId);
    }
    case 'destroyDraftedCard': {
      if (state.phase.type !== 'action') return false;
      const player = state.players[state.phase.actionState.currentPlayerIndex];
      return player.draftedCards.some(c => c.instanceId === choice.cardInstanceId);
    }
    case 'endTurn':
    case 'skipMix':
      return true;
    case 'storeColors': {
      if (state.phase.type !== 'action') return false;
      const player = state.players[state.phase.actionState.currentPlayerIndex];
      return choice.cardInstanceIds.every(id => player.drawnCards.some(c => c.instanceId === id));
    }
    case 'destroyDrawnCards': {
      if (state.phase.type !== 'action') return false;
      const player = state.players[state.phase.actionState.currentPlayerIndex];
      return choice.cardInstanceIds.every(id => player.drawnCards.some(c => c.instanceId === id));
    }
    case 'mix': {
      if (state.phase.type !== 'action') return false;
      const player = state.players[state.phase.actionState.currentPlayerIndex];
      return player.colorWheel[choice.colorA] > 0 &&
             player.colorWheel[choice.colorB] > 0 &&
             canMix(choice.colorA, choice.colorB);
    }
    case 'chooseGarment': {
      if (state.phase.type !== 'action') return false;
      const player = state.players[state.phase.actionState.currentPlayerIndex];
      const garmentInst = state.garmentDisplay.find(g => g.instanceId === choice.garmentInstanceId);
      if (!garmentInst) return false;
      return canAffordGarment(player, garmentInst.card);
    }
    case 'garmentPayment': {
      if (state.phase.type !== 'action') return false;
      const player = state.players[state.phase.actionState.currentPlayerIndex];
      const hasFabric = player.drawnCards.some(c => c.instanceId === choice.fabricCardId);
      if (!hasFabric) return false;
      const pending = state.phase.actionState.pendingChoice;
      if (!pending || pending.type !== 'chooseGarmentPayment') return false;
      const garmentInst = state.garmentDisplay.find(g => g.instanceId === pending.garmentInstanceId);
      if (!garmentInst) return false;
      return canPayCost(player.colorWheel, garmentInst.card.colorCost);
    }
  }
}

// ── Status ──

function getGameStatus(state: GameState): GameStatus {
  const phase = state.phase;

  if (phase.type === 'draft' && !phase.draftState.waitingForPass) {
    return { type: 'awaitingAction', playerId: phase.draftState.currentPlayerIndex };
  }
  if (phase.type === 'action') {
    return { type: 'awaitingAction', playerId: phase.actionState.currentPlayerIndex };
  }
  if (phase.type === 'gameOver') {
    const scores = state.players.map(p => calculateScore(p));
    const total = scores.reduce((a, b) => a + b, 0);
    return {
      type: 'terminated',
      scores: scores.map(s => s / Math.max(1, total)),
    };
  }

  // draw phase or waitingForPass shouldn't be seen, but handle gracefully
  return { type: 'awaitingAction', playerId: 0 };
}

// ── Determinization ──

export type SeenHands = CardInstance[][];

function determinize(
  state: GameState,
  perspectivePlayer: number,
  seenHands?: SeenHands,
): GameState {
  const clone = cloneGameState(state);

  if (clone.phase.type !== 'draft') {
    // Outside draft: only shuffle hidden decks
    clone.draftDeck = shuffle(clone.draftDeck);
    clone.garmentDeck = shuffle(clone.garmentDeck);
    for (const p of clone.players) {
      p.deck = shuffle(p.deck);
    }
    return clone;
  }

  const ds = clone.phase.draftState;
  const numPlayers = clone.players.length;
  const direction = ds.direction;

  // Determine which hands are known via the seenHands chain
  const knownHands = new Set<number>();
  knownHands.add(perspectivePlayer); // We always know our own hand

  if (seenHands) {
    for (let round = 0; round < seenHands.length; round++) {
      const hand = seenHands[round];
      if (!hand) continue;

      // The perspective player saw this hand at pick round `round`.
      // They picked their `round`-th drafted card from it.
      // The remaining cards went to the next player for the next pick round.
      let currentCards = [...hand];
      let receiver = perspectivePlayer;

      // Remove what perspective player picked at this round
      const perspPick = state.players[perspectivePlayer].draftedCards[round];
      if (perspPick) {
        currentCards = currentCards.filter(c => c.instanceId !== perspPick.instanceId);
      }

      // Chain through subsequent players
      for (let step = 0; step < numPlayers - 1; step++) {
        receiver = ((receiver + direction) % numPlayers + numPlayers) % numPlayers;
        if (receiver === perspectivePlayer) break;

        // At pick round (round + step + 1), this receiver had `currentCards`
        const pickRound = round + step + 1;
        if (pickRound > ds.pickNumber) break;

        // If the current pick round hasn't happened for all players yet,
        // we can only deduce hands up to completed pick rounds
        if (pickRound >= ds.pickNumber && ds.currentPlayerIndex <= receiver) break;

        // This receiver picked their (pickRound)-th drafted card
        const receiverPick = state.players[receiver].draftedCards[pickRound];
        if (receiverPick && currentCards.some(c => c.instanceId === receiverPick.instanceId)) {
          knownHands.add(receiver);
          currentCards = currentCards.filter(c => c.instanceId !== receiverPick.instanceId);
        } else {
          break; // Chain broken: pick doesn't match known cards
        }
      }
    }
  }

  // Pool cards from unknown hands, shuffle, redistribute
  const unknownPlayers: number[] = [];
  const pool: CardInstance[] = [];
  for (let i = 0; i < numPlayers; i++) {
    if (!knownHands.has(i)) {
      unknownPlayers.push(i);
      pool.push(...ds.hands[i]);
    }
  }

  if (unknownPlayers.length > 0) {
    const shuffled = shuffle(pool);
    let idx = 0;
    for (const pi of unknownPlayers) {
      const handSize = ds.hands[pi].length;
      ds.hands[pi] = shuffled.slice(idx, idx + handSize);
      idx += handSize;
    }
  }

  // Shuffle hidden decks
  clone.draftDeck = shuffle(clone.draftDeck);
  clone.garmentDeck = shuffle(clone.garmentDeck);
  for (const p of clone.players) {
    p.deck = shuffle(p.deck);
  }

  return clone;
}

// ── Rollout policy ──

function getRolloutChoice(state: GameState): ColoriChoice {
  const phase = state.phase;

  if (phase.type === 'draft') {
    const ds = phase.draftState;
    const hand = ds.hands[ds.currentPlayerIndex];
    return { type: 'draftPick', cardInstanceId: hand[Math.floor(Math.random() * hand.length)].instanceId };
  }

  if (phase.type === 'action') {
    const as_ = phase.actionState;
    const player = state.players[as_.currentPlayerIndex];
    const pending = as_.pendingChoice;

    if (pending === null) {
      if (player.draftedCards.length > 0 && Math.random() < 0.8) {
        const card = player.draftedCards[Math.floor(Math.random() * player.draftedCards.length)];
        return { type: 'destroyDraftedCard', cardInstanceId: card.instanceId };
      }
      return { type: 'endTurn' };
    }

    switch (pending.type) {
      case 'chooseCardsForStore': {
        const cardsWithPips = player.drawnCards.filter(c => getCardPips(c.card).length > 0);
        if (cardsWithPips.length === 0) return { type: 'storeColors', cardInstanceIds: [] };
        const count = Math.min(pending.count, cardsWithPips.length);
        const pick = Math.floor(Math.random() * count) + 1;
        const shuffled = [...cardsWithPips].sort(() => Math.random() - 0.5);
        return { type: 'storeColors', cardInstanceIds: shuffled.slice(0, pick).map(c => c.instanceId) };
      }
      case 'chooseCardsToDestroy': {
        const count = Math.min(pending.count, player.drawnCards.length);
        if (count === 0) return { type: 'destroyDrawnCards', cardInstanceIds: [] };
        const pick = Math.floor(Math.random() * count) + 1;
        const shuffled = [...player.drawnCards].sort(() => Math.random() - 0.5);
        return { type: 'destroyDrawnCards', cardInstanceIds: shuffled.slice(0, pick).map(c => c.instanceId) };
      }
      case 'chooseMix': {
        if (Math.random() < 0.5) return { type: 'skipMix' };
        const pairs: [Color, Color][] = [];
        for (let i = 0; i < ALL_COLORS.length; i++) {
          for (let j = i + 1; j < ALL_COLORS.length; j++) {
            const a = ALL_COLORS[i];
            const b = ALL_COLORS[j];
            if (player.colorWheel[a] > 0 && player.colorWheel[b] > 0 && canMix(a, b)) {
              pairs.push([a, b]);
            }
          }
        }
        if (pairs.length === 0) return { type: 'skipMix' };
        const [colorA, colorB] = pairs[Math.floor(Math.random() * pairs.length)];
        return { type: 'mix', colorA, colorB };
      }
      case 'chooseGarment': {
        const affordable = state.garmentDisplay.filter(g => canAffordGarment(player, g.card));
        if (affordable.length === 0) return { type: 'chooseGarment', garmentInstanceId: state.garmentDisplay[0]?.instanceId ?? 0 };
        return { type: 'chooseGarment', garmentInstanceId: affordable[Math.floor(Math.random() * affordable.length)].instanceId };
      }
      case 'chooseGarmentPayment': {
        const garmentInst = state.garmentDisplay.find(g => g.instanceId === pending.garmentInstanceId);
        if (!garmentInst) {
          // Fallback
          return { type: 'garmentPayment', fabricCardId: 0 };
        }
        const garment = garmentInst.card;
        const options: ColoriChoice[] = [];
        for (const fabric of player.drawnCards) {
          if (fabric.card.kind !== 'fabric' || fabric.card.fabricType !== garment.requiredFabric) continue;
          if (canPayCost(player.colorWheel, garment.colorCost)) {
            options.push({ type: 'garmentPayment', fabricCardId: fabric.instanceId });
          }
        }
        if (options.length === 0) {
          return { type: 'garmentPayment', fabricCardId: 0 };
        }
        return options[Math.floor(Math.random() * options.length)];
      }
    }
  }

  throw new Error('Cannot get rollout choice for current state');
}

// ── ColoriGame class implementing Game interface ──

export class ColoriGame implements Game<ColoriChoice> {
  state: GameState;
  seenHands?: SeenHands;

  constructor(state: GameState, seenHands?: SeenHands) {
    this.state = state;
    this.seenHands = seenHands;
  }

  getAllChoices(): ColoriChoice[] {
    return enumerateChoices(this.state);
  }

  applyChoice(choice: ColoriChoice): void {
    applyChoiceToState(this.state, choice);
  }

  status(): GameStatus {
    return getGameStatus(this.state);
  }

  getDeterminization(perspectivePlayer: number): Game<ColoriChoice> {
    const detState = determinize(this.state, perspectivePlayer, this.seenHands);
    return new ColoriGame(detState, this.seenHands);
  }

  choiceIsAvailable(choice: ColoriChoice): boolean {
    return checkChoiceAvailable(this.state, choice);
  }

  getRolloutChoice(): ColoriChoice {
    return getRolloutChoice(this.state);
  }

  choiceKey(choice: ColoriChoice): string {
    return choiceToKey(choice);
  }
}

import type { Game, GameStatus } from './ismcts';
import type {
  GameState, PlayerState, CardInstance, Color, BuyerCard,
  DraftState, ActionState, Ability, PendingChoice, GamePhase,
} from '../data/types';
import { getCardPips, PRIMARIES, SECONDARIES, TERTIARIES } from '../data/cards';
import { ALL_COLORS, canMix } from '../data/colors';
import { canPayCost } from '../engine/colorWheel';
import { playerPick, confirmPass } from '../engine/draftPhase';
import { executeDrawPhase } from '../engine/drawPhase';
import {
  destroyDraftedCard, endPlayerTurn, resolveWorkshopChoice,
  resolveMixColors, skipMix, skipWorkshop, resolveDestroyCards,
  resolveSelectBuyer, resolveGainSecondary, resolveGainPrimary,
  resolveChooseTertiaryToLose, resolveChooseTertiaryToGain,
} from '../engine/actionPhase';
import { calculateScore } from '../engine/scoring';
import { shuffle } from '../engine/deckUtils';

// ── Choice type ──

export type ColoriChoice =
  | { type: 'draftPick'; cardInstanceId: number }
  | { type: 'destroyDraftedCard'; cardInstanceId: number }
  | { type: 'endTurn' }
  | { type: 'workshop'; cardInstanceIds: number[] }
  | { type: 'skipWorkshop' }
  | { type: 'destroyDrawnCards'; cardInstanceIds: number[] }
  | { type: 'mix'; colorA: Color; colorB: Color }
  | { type: 'skipMix' }
  | { type: 'selectBuyer'; buyerInstanceId: number }
  | { type: 'gainSecondary'; color: Color }
  | { type: 'gainPrimary'; color: Color }
  | { type: 'chooseTertiaryToLose'; color: Color }
  | { type: 'chooseTertiaryToGain'; color: Color };

// ── Deep clone ──

function clonePlayerState(p: PlayerState): PlayerState {
  return {
    name: p.name,
    deck: [...p.deck],
    discard: [...p.discard],
    drawnCards: [...p.drawnCards],
    draftedCards: [...p.draftedCards],
    colorWheel: { ...p.colorWheel },
    ducats: p.ducats,
    materials: { ...p.materials },
    completedBuyers: [...p.completedBuyers],
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
          abilityStack: as_.abilityStack.map(a => ({ ...a })),
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
    buyerDeck: [...state.buyerDeck],
    buyerDisplay: [...state.buyerDisplay],
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

// ── Buyer affordability helpers ──

function canAffordBuyer(
  player: PlayerState,
  buyer: BuyerCard,
): boolean {
  if (player.materials[buyer.requiredMaterial] <= 0) return false;
  return canPayCost(player.colorWheel, buyer.colorCost);
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
      case 'chooseCardsForWorkshop': {
        const choices: ColoriChoice[] = [{ type: 'skipWorkshop' }];
        // Non-action card subsets
        const eligibleNonAction = player.drawnCards
          .filter(c => c.card.kind === 'dye' || c.card.kind === 'basicDye' || c.card.kind === 'material')
          .map(c => c.instanceId);
        const nonActionSubsets = getSubsets(eligibleNonAction, pending.count)
          .map(ids => ({ type: 'workshop' as const, cardInstanceIds: ids }));
        choices.push(...nonActionSubsets);
        // Each action card as a separate choice
        for (const card of player.drawnCards) {
          if (card.card.kind === 'action') {
            choices.push({ type: 'workshop', cardInstanceIds: [card.instanceId] });
          }
        }
        // If no choices besides skip, add empty
        if (choices.length === 1) {
          choices.push({ type: 'workshop', cardInstanceIds: [] });
        }
        return choices;
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
      case 'chooseBuyer': {
        return state.buyerDisplay
          .filter(g => canAffordBuyer(player, g.card))
          .map(g => ({ type: 'selectBuyer' as const, buyerInstanceId: g.instanceId }));
      }
      case 'chooseSecondaryColor': {
        return SECONDARIES.map(c => ({ type: 'gainSecondary' as const, color: c }));
      }
      case 'choosePrimaryColor': {
        return PRIMARIES.map(c => ({ type: 'gainPrimary' as const, color: c }));
      }
      case 'chooseTertiaryToLose': {
        return TERTIARIES
          .filter(c => player.colorWheel[c] > 0)
          .map(c => ({ type: 'chooseTertiaryToLose' as const, color: c }));
      }
      case 'chooseTertiaryToGain': {
        return TERTIARIES
          .filter(c => c !== pending.lostColor)
          .map(c => ({ type: 'chooseTertiaryToGain' as const, color: c }));
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
    case 'workshop':
      return `workshop:${[...choice.cardInstanceIds].sort((a, b) => a - b).join(',')}`;
    case 'skipWorkshop':
      return 'skipWorkshop';
    case 'destroyDrawnCards':
      return `destroyDrawn:${[...choice.cardInstanceIds].sort((a, b) => a - b).join(',')}`;
    case 'mix':
      return `mix:${choice.colorA}:${choice.colorB}`;
    case 'skipMix':
      return 'skipMix';
    case 'selectBuyer':
      return `selectBuyer:${choice.buyerInstanceId}`;
    case 'gainSecondary':
      return `gainSecondary:${choice.color}`;
    case 'gainPrimary':
      return `gainPrimary:${choice.color}`;
    case 'chooseTertiaryToLose':
      return `loseTertiary:${choice.color}`;
    case 'chooseTertiaryToGain':
      return `gainTertiary:${choice.color}`;
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
    case 'workshop':
      resolveWorkshopChoice(state, choice.cardInstanceIds);
      break;
    case 'skipWorkshop':
      skipWorkshop(state);
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
    case 'selectBuyer':
      resolveSelectBuyer(state, choice.buyerInstanceId);
      break;
    case 'gainSecondary':
      resolveGainSecondary(state, choice.color);
      break;
    case 'gainPrimary':
      resolveGainPrimary(state, choice.color);
      break;
    case 'chooseTertiaryToLose':
      resolveChooseTertiaryToLose(state, choice.color);
      break;
    case 'chooseTertiaryToGain':
      resolveChooseTertiaryToGain(state, choice.color);
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
      if (state.phase.actionState.pendingChoice !== null) return false;
      const player = state.players[state.phase.actionState.currentPlayerIndex];
      return player.draftedCards.some(c => c.instanceId === choice.cardInstanceId);
    }
    case 'endTurn': {
      if (state.phase.type !== 'action') return false;
      return state.phase.actionState.pendingChoice === null;
    }
    case 'workshop': {
      if (state.phase.type !== 'action') return false;
      const pending = state.phase.actionState.pendingChoice;
      if (!pending || pending.type !== 'chooseCardsForWorkshop') return false;
      const player = state.players[state.phase.actionState.currentPlayerIndex];
      return choice.cardInstanceIds.every(id => player.drawnCards.some(c => c.instanceId === id));
    }
    case 'skipWorkshop': {
      if (state.phase.type !== 'action') return false;
      const pending = state.phase.actionState.pendingChoice;
      return !!pending && pending.type === 'chooseCardsForWorkshop';
    }
    case 'destroyDrawnCards': {
      if (state.phase.type !== 'action') return false;
      const pending = state.phase.actionState.pendingChoice;
      if (!pending || pending.type !== 'chooseCardsToDestroy') return false;
      const player = state.players[state.phase.actionState.currentPlayerIndex];
      return choice.cardInstanceIds.every(id => player.drawnCards.some(c => c.instanceId === id));
    }
    case 'mix': {
      if (state.phase.type !== 'action') return false;
      const pending = state.phase.actionState.pendingChoice;
      if (!pending || pending.type !== 'chooseMix') return false;
      const player = state.players[state.phase.actionState.currentPlayerIndex];
      return player.colorWheel[choice.colorA] > 0 &&
             player.colorWheel[choice.colorB] > 0 &&
             canMix(choice.colorA, choice.colorB);
    }
    case 'skipMix': {
      if (state.phase.type !== 'action') return false;
      const pending = state.phase.actionState.pendingChoice;
      return !!pending && pending.type === 'chooseMix';
    }
    case 'selectBuyer': {
      if (state.phase.type !== 'action') return false;
      const pending = state.phase.actionState.pendingChoice;
      if (!pending || pending.type !== 'chooseBuyer') return false;
      const player = state.players[state.phase.actionState.currentPlayerIndex];
      const buyerInst = state.buyerDisplay.find(g => g.instanceId === choice.buyerInstanceId);
      if (!buyerInst) return false;
      return canAffordBuyer(player, buyerInst.card);
    }
    case 'gainSecondary': {
      if (state.phase.type !== 'action') return false;
      const pending = state.phase.actionState.pendingChoice;
      if (!pending || pending.type !== 'chooseSecondaryColor') return false;
      return (SECONDARIES as Color[]).includes(choice.color);
    }
    case 'gainPrimary': {
      if (state.phase.type !== 'action') return false;
      const pending = state.phase.actionState.pendingChoice;
      if (!pending || pending.type !== 'choosePrimaryColor') return false;
      return (PRIMARIES as Color[]).includes(choice.color);
    }
    case 'chooseTertiaryToLose': {
      if (state.phase.type !== 'action') return false;
      const pending = state.phase.actionState.pendingChoice;
      if (!pending || pending.type !== 'chooseTertiaryToLose') return false;
      const player = state.players[state.phase.actionState.currentPlayerIndex];
      return (TERTIARIES as Color[]).includes(choice.color) && player.colorWheel[choice.color] > 0;
    }
    case 'chooseTertiaryToGain': {
      if (state.phase.type !== 'action') return false;
      const pending = state.phase.actionState.pendingChoice;
      if (!pending || pending.type !== 'chooseTertiaryToGain') return false;
      return (TERTIARIES as Color[]).includes(choice.color) && choice.color !== pending.lostColor;
    }
  }
}

// ── Status ──

function getGameStatus(state: GameState, maxRound?: number): GameStatus {
  const phase = state.phase;

  if (maxRound !== undefined && state.round > maxRound) {
    const scores = state.players.map(p => calculateScore(p));
    const total = scores.reduce((a, b) => a + b, 0);
    return {
      type: 'terminated',
      scores: scores.map(s => s / Math.max(1, total)),
    };
  }

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
    clone.buyerDeck = shuffle(clone.buyerDeck);
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
  clone.buyerDeck = shuffle(clone.buyerDeck);
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
      case 'chooseCardsForWorkshop': {
        // Randomly decide: skip, pick non-action cards, or pick an action card
        const actionCards = player.drawnCards.filter(c => c.card.kind === 'action');
        const eligibleCards = player.drawnCards.filter(c => c.card.kind === 'dye' || c.card.kind === 'basicDye' || c.card.kind === 'material');

        if (eligibleCards.length === 0 && actionCards.length === 0) return { type: 'skipWorkshop' };

        // 20% chance to skip
        if (Math.random() < 0.2) return { type: 'skipWorkshop' };

        // 50% chance to pick action card if available
        if (actionCards.length > 0 && Math.random() < 0.5) {
          const card = actionCards[Math.floor(Math.random() * actionCards.length)];
          return { type: 'workshop', cardInstanceIds: [card.instanceId] };
        }

        if (eligibleCards.length === 0) {
          if (actionCards.length > 0) {
            const card = actionCards[Math.floor(Math.random() * actionCards.length)];
            return { type: 'workshop', cardInstanceIds: [card.instanceId] };
          }
          return { type: 'skipWorkshop' };
        }

        const count = Math.min(pending.count, eligibleCards.length);
        const pick = Math.floor(Math.random() * count) + 1;
        const shuffled = [...eligibleCards].sort(() => Math.random() - 0.5);
        return { type: 'workshop', cardInstanceIds: shuffled.slice(0, pick).map(c => c.instanceId) };
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
      case 'chooseBuyer': {
        const affordable = state.buyerDisplay.filter(g => canAffordBuyer(player, g.card));
        if (affordable.length === 0) throw new Error('chooseBuyer pending but no affordable buyers (should be unreachable)');
        return { type: 'selectBuyer', buyerInstanceId: affordable[Math.floor(Math.random() * affordable.length)].instanceId };
      }
      case 'chooseSecondaryColor': {
        return { type: 'gainSecondary', color: SECONDARIES[Math.floor(Math.random() * SECONDARIES.length)] };
      }
      case 'choosePrimaryColor': {
        return { type: 'gainPrimary', color: PRIMARIES[Math.floor(Math.random() * PRIMARIES.length)] };
      }
      case 'chooseTertiaryToLose': {
        const owned = TERTIARIES.filter(c => player.colorWheel[c] > 0);
        return { type: 'chooseTertiaryToLose', color: owned[Math.floor(Math.random() * owned.length)] };
      }
      case 'chooseTertiaryToGain': {
        const options = TERTIARIES.filter(c => c !== pending.lostColor);
        return { type: 'chooseTertiaryToGain', color: options[Math.floor(Math.random() * options.length)] };
      }
    }
  }

  throw new Error('Cannot get rollout choice for current state');
}

// ── ColoriGame class implementing Game interface ──

export class ColoriGame implements Game<ColoriChoice> {
  state: GameState;
  seenHands?: SeenHands;

  maxRound?: number;

  constructor(state: GameState, seenHands?: SeenHands, maxRound?: number) {
    this.state = state;
    this.seenHands = seenHands;
    this.maxRound = maxRound;
  }

  getAllChoices(): ColoriChoice[] {
    return enumerateChoices(this.state);
  }

  applyChoice(choice: ColoriChoice): void {
    applyChoiceToState(this.state, choice);
  }

  status(): GameStatus {
    return getGameStatus(this.state, this.maxRound);
  }

  getDeterminization(perspectivePlayer: number): Game<ColoriChoice> {
    const detState = determinize(this.state, perspectivePlayer, this.seenHands);
    return new ColoriGame(detState, this.seenHands, this.maxRound);
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

export interface Game<Choice> {
  getAllChoices(): Choice[];
  applyChoice(choice: Choice): void;
  status(): GameStatus;
  getDeterminization(perspectivePlayer: number): Game<Choice>;
  choiceIsAvailable(choice: Choice): boolean;
  getRolloutChoice(): Choice;
  choiceKey(choice: Choice): string;
}

export type GameStatus =
  | { type: 'awaitingAction'; playerId: number }
  | { type: 'terminated'; scores: number[] };

class MctsNode<Choice> {
  games = 0;
  cumulativeReward = 0;
  playerId: number;
  choice: Choice | null;
  children: Map<string, MctsNode<Choice>> = new Map();
  choiceAvailabilityCount: Map<string, number> = new Map();

  constructor(playerId: number, choice: Choice | null) {
    this.playerId = playerId;
    this.choice = choice;
  }

  isRoot(): boolean {
    return this.choice === null;
  }

  expand(game: Game<Choice>, choices: Choice[], choiceKey: (c: Choice) => string): void {
    shuffleArray(choices);
    const activePlayer = (game.status() as { type: 'awaitingAction'; playerId: number }).playerId;
    let addedNewNode = false;

    for (const choice of choices) {
      const key = choiceKey(choice);
      const existing = this.choiceAvailabilityCount.get(key);
      if (existing !== undefined) {
        this.choiceAvailabilityCount.set(key, existing + 1);
      } else {
        this.choiceAvailabilityCount.set(key, 0);
      }

      if (this.isRoot() || (!addedNewNode && !this.children.has(key))) {
        this.children.set(key, new MctsNode<Choice>(activePlayer, choice));
        addedNewNode = true;
      }
    }
  }
}

function shuffleArray<T>(array: T[]): void {
  for (let i = array.length - 1; i > 0; i--) {
    const j = Math.floor(Math.random() * (i + 1));
    [array[i], array[j]] = [array[j], array[i]];
  }
}

function upperConfidenceBound(cumulativeReward: number, games: number, totalGameCount: number, c: number): number {
  const winRate = cumulativeReward / games;
  return winRate + c * Math.sqrt(Math.log(totalGameCount) / games);
}

const C = Math.SQRT2;
const MAX_ROLLOUT_STEPS = 1000;

export function ismcts<Choice>(game: Game<Choice>, iterations: number): Choice {
  const status = game.status();
  if (status.type !== 'awaitingAction') {
    throw new Error('Game is not awaiting action');
  }
  const playerId = status.playerId;
  const root = new MctsNode<Choice>(playerId, null);
  const choiceKeyFn = (c: Choice) => game.choiceKey(c);

  for (let i = 0; i < iterations; i++) {
    const determinization = game.getDeterminization(playerId);
    iteration(root, determinization, choiceKeyFn);
  }

  if (root.children.size === 0) {
    const choices = game.getAllChoices();
    return choices[Math.floor(Math.random() * choices.length)];
  }

  let bestChild: MctsNode<Choice> | null = null;
  for (const child of root.children.values()) {
    if (bestChild === null || child.games > bestChild.games) {
      bestChild = child;
    }
  }

  return bestChild!.choice!;
}

function iteration<Choice>(
  node: MctsNode<Choice>,
  game: Game<Choice>,
  choiceKey: (c: Choice) => string,
): number[] {
  const status = game.status();
  if (status.type === 'terminated') {
    recordOutcome(node, status.scores);
    return status.scores;
  }

  // Expand: root skips re-expansion if already has children
  if (!(node.isRoot() && node.children.size > 0)) {
    const choices = game.getAllChoices();
    node.expand(game, choices, choiceKey);
  }

  // Select
  const bestChild = select(node, game);
  if (bestChild === null) {
    // No available children — treat as terminal with empty scores
    const emptyScores: number[] = [];
    recordOutcome(node, emptyScores);
    return emptyScores;
  }
  game.applyChoice(bestChild.choice!);

  let scores: number[];
  if (bestChild.games === 0) {
    scores = rollout(game);
    recordOutcome(bestChild, scores);
  } else {
    scores = iteration(bestChild, game, choiceKey);
  }

  recordOutcome(node, scores);
  return scores;
}

function select<Choice>(
  node: MctsNode<Choice>,
  game: Game<Choice>,
): MctsNode<Choice> | null {
  let bestChild: MctsNode<Choice> | null = null;
  let bestValue = -Infinity;

  for (const [key, child] of node.children) {
    if (!game.choiceIsAvailable(child.choice!)) continue;

    let value: number;
    if (child.games === 0) {
      value = Infinity;
    } else {
      const totalGameCount = node.isRoot()
        ? node.games
        : (node.choiceAvailabilityCount.get(key) ?? node.games);
      value = upperConfidenceBound(child.cumulativeReward, child.games, totalGameCount, C);
    }

    if (value > bestValue) {
      bestValue = value;
      bestChild = child;
    }
  }

  return bestChild;
}

function rollout<Choice>(game: Game<Choice>): number[] {
  for (let step = 0; step < MAX_ROLLOUT_STEPS; step++) {
    const status = game.status();
    if (status.type === 'terminated') {
      return status.scores;
    }
    const choice = game.getRolloutChoice();
    game.applyChoice(choice);
  }
  // Timeout: check final status
  const status = game.status();
  if (status.type === 'terminated') {
    return status.scores;
  }
  // Game didn't finish; return zeros
  return [];
}

function recordOutcome<Choice>(node: MctsNode<Choice>, scores: number[]): void {
  const reward = scores[node.playerId] ?? 0;
  node.cumulativeReward += reward;
  node.games += 1;
}

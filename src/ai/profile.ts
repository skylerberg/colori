import { ColoriGame, cloneGameState } from './coloriGame';
import { ismcts } from './ismcts';
import { setupDraftGame, setupActionGame } from './benchHelper';

const args = process.argv.slice(2);

function getArg(name: string, defaultValue: string): string {
  const index = args.indexOf(`--${name}`);
  if (index === -1 || index + 1 >= args.length) return defaultValue;
  return args[index + 1];
}

const iterations = parseInt(getArg('iterations', '100000'), 10);
const phase = getArg('phase', 'draft');
const players = parseInt(getArg('players', '3'), 10);

const state = phase === 'action' ? setupActionGame(players) : setupDraftGame(players);
const game = new ColoriGame(cloneGameState(state));

console.error(`Running ${iterations} ISMCTS iterations (${phase} phase, ${players} players)...`);

const start = performance.now();
ismcts(game, iterations);
const elapsed = performance.now() - start;

console.error(`Elapsed: ${(elapsed / 1000).toFixed(2)}s`);
console.error(`Iterations/sec: ${(iterations / (elapsed / 1000)).toFixed(0)}`);

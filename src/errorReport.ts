import { EngineError } from './engine/engineCall';

/** An error put in front of the player, ready for them to send on. */
export interface ErrorReport {
  /** What failed, in the player's terms. */
  title: string;
  message: string;
  /** Everything worth sending: the error, where it ran, and what it ran on. */
  text: string;
}

export function createErrorReport(title: string, error: unknown, gameState?: unknown): ErrorReport {
  const message = error instanceof Error ? error.message : String(error);
  const lines = [
    'Colori error report',
    `What: ${title}`,
    `Error: ${message}`,
    `Build: ${__APP_BUILD__}`,
    `Time: ${new Date().toISOString()}`,
    `Browser: ${navigator.userAgent}`,
  ];
  if (error instanceof Error && error.stack) {
    // Some browsers open the stack with the message, which is already above.
    const header = `${error.name}: ${error.message}`;
    const frames = error.stack.startsWith(header) ? error.stack.slice(header.length).trimStart() : error.stack;
    if (frames) lines.push('', 'Stack:', frames);
  }
  // The engine's own input reproduces the failure; the screen's state is the
  // next best thing.
  if (error instanceof EngineError && error.input !== undefined) {
    lines.push('', 'Input:', ...describe(error.input));
  } else if (gameState !== undefined) {
    lines.push('', 'Game state:', JSON.stringify(gameState));
  }
  return { title, message, text: lines.join('\n') };
}

function describe(input: unknown): string[] {
  if (typeof input !== 'object' || input === null) return [JSON.stringify(input)];
  return Object.entries(input).map(
    ([key, value]) => `${key}: ${typeof value === 'string' ? value : JSON.stringify(value)}`,
  );
}

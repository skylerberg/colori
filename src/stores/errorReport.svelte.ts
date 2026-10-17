import { createErrorReport, type ErrorReport } from '../errorReport';

let _shown = $state<ErrorReport | null>(null);

/** The error on show for failures no screen shows itself. */
export const shownError = {
  get report() { return _shown; },
  dismiss() { _shown = null; },
};

/**
 * Put an error in front of the player. The first stays until dismissed: what
 * follows a failure is usually its consequence.
 */
export function reportError(title: string, error: unknown, gameState?: unknown): void {
  console.error(title, error);
  _shown ??= createErrorReport(title, error, gameState);
}

/** Report errors nothing caught. Returns a function that stops reporting them. */
export function reportUncaughtErrors(): () => void {
  const onError = (event: ErrorEvent) => {
    // Benign, and reported by some browsers whenever layout settles late.
    if (event.message.includes('ResizeObserver loop')) return;
    reportError('Something went wrong', event.error ?? event.message);
  };
  const onRejection = (event: PromiseRejectionEvent) => reportError('Something went wrong', event.reason);
  window.addEventListener('error', onError);
  window.addEventListener('unhandledrejection', onRejection);
  return () => {
    window.removeEventListener('error', onError);
    window.removeEventListener('unhandledrejection', onRejection);
  };
}

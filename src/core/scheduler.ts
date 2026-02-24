export type BackoffOptions = {
  baseMs: number;
  maxMs: number;
  multiplier: number;
  jitterRatio: number;
};

export const DEFAULT_BACKOFF: BackoffOptions = {
  baseMs: 30_000,
  maxMs: 5 * 60_000,
  multiplier: 2,
  jitterRatio: 0.2,
};

export function computeBackoffDelayMs(
  consecutiveFailures: number,
  opts: BackoffOptions = DEFAULT_BACKOFF,
  random: () => number = Math.random
): number {
  const failures = Math.max(0, consecutiveFailures);
  const raw = opts.baseMs * opts.multiplier ** failures;
  const clamped = Math.min(opts.maxMs, raw);

  const jitter = clamped * opts.jitterRatio;
  const delta = (random() * 2 - 1) * jitter;
  const delayed = clamped + delta;
  return Math.max(opts.baseMs, Math.round(delayed));
}

/**
 * Diagnostics settings (18.B) — the single owning file for perf budgets, storage limits and the
 * redaction list, per CLAUDE.md "Centralize configuration."
 */

export const PERF_BUDGET_MS = {
  service: 150,
  route: 300,
  longTask: 50,
  startup: 1500,
} as const;

export const SNAPSHOT_BUDGET_BYTES = 5 * 1024 * 1024;

/** Desktop file rotation/retention (18.B3) — mirrored into the Rust side's `diag_rotate` call. */
export const RETENTION_DAYS = 14;
export const ERROR_RETENTION_DAYS = 60;
export const ROTATE_AT_BYTES = 5 * 1024 * 1024;

/** Browser/e2e ring buffer (18.B3) — last N entries per channel, kept in IndexedDB. */
export const RING_BUFFER_SIZE = 5000;

/** Writes are batched instead of hitting disk/IndexedDB per call. */
export const FLUSH_INTERVAL_MS = 1000;
export const FLUSH_BATCH_SIZE = 50;

/**
 * Field names redacted (replaced with `"[محجوب]"`) anywhere they appear in `LogEntry.data`, at any
 * nesting depth, case-insensitively — passwords, tokens and PINs must never reach a log file that
 * could end up in a support bundle.
 */
export const REDACTED_KEYS = [
  'password',
  'pin',
  'token',
  'secret',
  'apiKey',
  'api_key',
  'authorization',
  'creditCard',
  'cardNumber',
  'cvv',
];

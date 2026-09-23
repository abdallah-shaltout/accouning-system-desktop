/**
 * Helpers shared by the mock backend. Nothing here should be imported by pages/components —
 * they talk to module `services/`, which are the only layer that knows data is mocked.
 */
import { matchesSearch } from '@/modules/core/helpers/search';

/** Simulated network/IPC latency (ms). Set to 0 for instant responses. */
export const MOCK_LATENCY = { min: 120, max: 380 };

export type LatencyMode = 'off' | 'realistic' | 'slow';

const LATENCY_PRESETS: Record<LatencyMode, { min: number; max: number }> = {
  off: { min: 0, max: 0 },
  realistic: { min: 120, max: 380 },
  slow: { min: 800, max: 1800 },
};

/** Dev-menu latency switch. `null` (default) uses `MOCK_LATENCY` unchanged. */
let latencyOverride: LatencyMode | null = null;

export function setLatencyMode(mode: LatencyMode | null): void {
  latencyOverride = mode;
}

export function getLatencyMode(): LatencyMode | null {
  return latencyOverride;
}

export function delay(ms?: number): Promise<void> {
  if (ms !== undefined) return new Promise((resolve) => setTimeout(resolve, ms));
  const { min, max } = latencyOverride ? LATENCY_PRESETS[latencyOverride] : MOCK_LATENCY;
  const wait = min + Math.random() * (max - min);
  return new Promise((resolve) => setTimeout(resolve, wait));
}

/** Services return deep copies so callers can never mutate mock state by reference (like a real API). */
export function clone<T>(value: T): T {
  return structuredClone(value);
}

/** Error thrown by mock services; `message` is user-facing Arabic text. */
export class ApiError extends Error {
  constructor(
    message: string,
    public code: 'NOT_FOUND' | 'VALIDATION' | 'CONFLICT' | 'FORBIDDEN' | 'UNAUTHORIZED' = 'VALIDATION',
  ) {
    super(message);
    this.name = 'ApiError';
  }
}

export function round2(n: number): number {
  return Math.round((n + Number.EPSILON) * 100) / 100;
}

export function sum<T>(items: T[], pick: (item: T) => number): number {
  return round2(items.reduce((acc, item) => acc + pick(item), 0));
}

const idCounters: Record<string, number> = {};

/** Deterministic ids (`inv-12`) so seeded links stay stable across reloads. */
export function uid(prefix: string): string {
  idCounters[prefix] = (idCounters[prefix] ?? 0) + 1;
  return `${prefix}-${idCounters[prefix]}`;
}

/**
 * `idCounters` is an in-memory module variable — it does NOT get restored when a persisted
 * IndexedDB snapshot is loaded (`persist.ts#loadSnapshot`), so on a fresh page load `uid()` starts
 * back at 1 for every prefix while the loaded `db` already has entries like `je-884`. The next
 * `uid('je')` call then collides with an existing id instead of getting a fresh one, silently
 * overwriting/shadowing the old record wherever code does `array.find(x => x.id === id)`.
 * `bumpIdCounter` lets `persist.ts` resync each prefix's counter to at least the highest numeric
 * suffix found in the restored snapshot right after loading it — see its call site there.
 */
export function bumpIdCounter(prefix: string, atLeast: number): void {
  if ((idCounters[prefix] ?? 0) < atLeast) idCounters[prefix] = atLeast;
}

export function padNumber(n: number, width = 6): string {
  return String(n).padStart(width, '0');
}

/** Local calendar date (YYYY-MM-DD) of an ISO timestamp. */
export function localDateKey(iso: string | Date): string {
  const d = typeof iso === 'string' ? new Date(iso) : iso;
  const y = d.getFullYear();
  const m = String(d.getMonth() + 1).padStart(2, '0');
  const day = String(d.getDate()).padStart(2, '0');
  return `${y}-${m}-${day}`;
}

/** Inclusive date-range check on local calendar dates. `from`/`to` are YYYY-MM-DD. */
export function inDateRange(iso: string, from?: string, to?: string): boolean {
  const key = localDateKey(iso);
  if (from && key < from) return false;
  if (to && key > to) return false;
  return true;
}

export function includesText(haystack: (string | undefined)[], needle?: string): boolean {
  return matchesSearch(haystack, needle);
}

export type RandomSource = ReturnType<typeof createRandom>;

/** Small seeded PRNG (mulberry32) so seed data is reproducible. */
export function createRandom(seed: number) {
  let a = seed >>> 0;
  const next = () => {
    a = (a + 0x6d2b79f5) >>> 0;
    let t = a;
    t = Math.imul(t ^ (t >>> 15), t | 1);
    t ^= t + Math.imul(t ^ (t >>> 7), t | 61);
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
  };
  return {
    next,
    int: (min: number, max: number) => Math.floor(next() * (max - min + 1)) + min,
    pick: <T>(items: T[]): T => items[Math.floor(next() * items.length)],
    chance: (p: number) => next() < p,
  };
}

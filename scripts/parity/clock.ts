/**
 * Pinned clock + deterministic randomness for a parity pass (plan 21 Part 04, B-8, decision P4-4).
 *
 * While pinned, `globalThis.Date` is a subclass whose no-argument constructor and `Date.now()`
 * return the pinned instant, and `process.env.TZ` is the base's zone (Bun applies it at runtime).
 * `Math.random` is a seeded PRNG so mock-side random values (a generated barcode, a template id) are
 * the same on every run of a case — `--mock-only`'s determinism check depends on it.
 *
 * The Rust side is pinned separately by the runner (`__clock { iso }` to `parity_host`, which runs
 * `SET timestamp` before every command). This file only owns the TS process.
 */
const RealDate = Date;
const realRandom = Math.random;
/** The machine's own zone, restored by `unpinClock` — also the zone the Rust importer falls back to
 * for a snapshot with no known country (`infrastructure/import/run.rs`: unknown → OS timezone). */
export const ORIGINAL_TZ: string = process.env.TZ || Intl.DateTimeFormat().resolvedOptions().timeZone;

let pinnedMs: number | null = null;

class PinnedDate extends RealDate {
  constructor(...args: unknown[]) {
    if (args.length === 0) {
      super(pinnedMs ?? RealDate.now());
    } else {
      // @ts-expect-error — forwarding every Date constructor overload unchanged.
      super(...args);
    }
  }
  static now(): number {
    return pinnedMs ?? RealDate.now();
  }
  /** Dates made before the pin (or by `structuredClone`) are plain `Date`s; keep `instanceof Date` true for them. */
  static [Symbol.hasInstance](value: unknown): boolean {
    return value instanceof RealDate;
  }
}

/** SA → Asia/Riyadh, EG → Africa/Cairo (cross-cutting §7, `domains/settings/service/country.rs`);
 * anything else → the OS zone, exactly like the Rust importer. */
export function zoneForCountry(country: string | undefined | null): string {
  if (country === 'SA') return 'Asia/Riyadh';
  if (country === 'EG') return 'Africa/Cairo';
  return ORIGINAL_TZ;
}

function mulberry32(seed: number): () => number {
  let a = seed >>> 0;
  return () => {
    a = (a + 0x6d2b79f5) >>> 0;
    let t = a;
    t = Math.imul(t ^ (t >>> 15), t | 1);
    t ^= t + Math.imul(t ^ (t >>> 7), t | 61);
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
  };
}

function hashSeed(s: string): number {
  let h = 2166136261;
  for (let i = 0; i < s.length; i++) {
    h ^= s.charCodeAt(i);
    h = Math.imul(h, 16777619);
  }
  return h >>> 0;
}

/** Pins "now" to `iso`, the zone to `tz`, and `Math.random` to a PRNG seeded from `randomSeed`. */
export function pinClock(iso: string, tz: string, randomSeed: string): void {
  const ms = RealDate.parse(iso);
  if (Number.isNaN(ms)) throw new Error(`pinClock: "${iso}" is not an ISO instant`);
  pinnedMs = ms;
  process.env.TZ = tz;
  globalThis.Date = PinnedDate as DateConstructor;
  Math.random = mulberry32(hashSeed(randomSeed));
}

/** Moves the pinned instant without touching the zone or the PRNG (`s.setClock`). */
export function movePinnedClock(iso: string): void {
  const ms = RealDate.parse(iso);
  if (Number.isNaN(ms)) throw new Error(`setClock: "${iso}" is not an ISO instant`);
  pinnedMs = ms;
}

/** The pinned instant as ISO, or null when not pinned. */
export function pinnedIso(): string | null {
  return pinnedMs === null ? null : new RealDate(pinnedMs).toISOString();
}

export function unpinClock(): void {
  pinnedMs = null;
  globalThis.Date = RealDate;
  Math.random = realRandom;
  // Always REASSIGN — never `delete process.env.TZ`. Bun caches the ICU timezone by the last
  // non-empty `TZ` value it saw: once `TZ` is deleted, a later `process.env.TZ = 'Africa/Cairo'`
  // (the next case's `pinClock`) is silently ignored and every `Date` keeps using the previous
  // zone — a plain `bun run` probe confirms this (set Riyadh → delete → set Cairo → dates still
  // render in Riyadh time). A run that mixes SA and EG cases would then compute the "wrong" EG
  // case's local dates in Riyadh time on the mock side only, a false diff against Rust (which is
  // told the zone explicitly per case and has no such cache). `ORIGINAL_TZ` is always a concrete
  // IANA string (never `undefined`), so reassigning it is always safe.
  process.env.TZ = ORIGINAL_TZ;
}

/** Runs `fn` under a pinned clock, always unpinning afterwards. */
export function withPinnedClock<T>(iso: string, tz: string, randomSeed: string, fn: () => T): T {
  pinClock(iso, tz, randomSeed);
  try {
    return fn();
  } finally {
    unpinClock();
  }
}

/** The real (unpinned) wall clock, for run timestamps and timing measurements. */
export function realNow(): Date {
  return new RealDate();
}

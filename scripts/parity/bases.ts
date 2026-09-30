/**
 * The shared starting states for parity cases (plan 21 Part 04, B-6, decision P4-5).
 *
 * - `demo-sa` / `demo-eg`: `seedDatabase(new Date('2026-06-30T09:00:00.000Z'), country)` — the same
 *   call as `scripts/verify/export-snapshot.ts` — built under the pinned clock and the country's
 *   zone, so the seed's local-time history is the same on every machine.
 * - `edge`: `src-tauri/tests/fixtures/mock-snapshot-edge.json` (the importer's edge fixture).
 * - `empty`: no snapshot. The mock gets its own blank `db` (what the dev "reset to empty" action
 *   leaves behind after reload); Rust gets `__reset_empty` (`wipe_business_rows` only).
 * - a function: a custom `MockDb`, built under the pinned default clock.
 *
 * The same snapshot object feeds both passes: the mock restores it into `db`, the Rust host imports
 * it with the D10 importer (`__reset`).
 */
import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import { db, resetDb, type MockDb } from '../../src/mocks/db';
import { seedDatabase } from '../../src/mocks/seed';
import { SCHEMA_VERSION, type Snapshot } from '../../src/mocks/persist';
import { clone, resetIdCounters } from '../../src/mocks/utils';
import type { BaseSpec } from './case';
import { withPinnedClock, zoneForCountry } from './clock';

/** The seed's "today" — every demo base and every default clock starts here. */
export const SEED_NOW = '2026-06-30T09:00:00.000Z';

export const EDGE_FIXTURE = join(import.meta.dirname, '../../src-tauri/tests/fixtures/mock-snapshot-edge.json');

export interface Base {
  /** `'demo-sa'`, `'edge'`, `'custom:<case>'`, … — for logs. */
  key: string;
  /** `null` only for `empty`. */
  snapshot: Snapshot | null;
  /** Default pinned clock for a case on this base. */
  savedAt: string;
  /** IANA zone the mock pass runs under. */
  tz: string;
  /** username → password, for the default login. */
  credentials: Record<string, string>;
}

const cache = new Map<string, Base>();

function fromSnapshot(key: string, snapshot: Snapshot): Base {
  const data = snapshot.data as Partial<MockDb>;
  return {
    key,
    snapshot,
    savedAt: snapshot.savedAt || SEED_NOW,
    tz: zoneForCountry(data.settings?.country),
    credentials: { ...(data.credentials ?? {}) },
  };
}

function demo(country: 'SA' | 'EG'): Base {
  const data = withPinnedClock(SEED_NOW, zoneForCountry(country), `base:demo-${country}`, () => {
    // `loadBase` builds each base at most once per process, but which one runs first (demo-sa vs
    // demo-eg) depends on case load order — without resetting first, the second `demo()` call's
    // `uid()` calls (`je-`, `excat-`, `recexp-`, …) continue from wherever the first base's seeding
    // left the shared module-level counters, so a base's own ids (and therefore every id-based
    // assertion/allowlist entry) would depend on which base happened to be built first. Also start
    // from a blank `db` so a previous base build's rows can never leak into this one's `seedDatabase`
    // (which itself resets most tables, but not ones no seed step touches for this country).
    resetIdCounters();
    resetDb();
    seedDatabase(new Date(SEED_NOW), country);
    return clone(db);
  });
  // The SA seed's settings fixture predates `settings.country` (only the EG branch of `seedSettings`
  // sets it). Without it the Rust importer falls back to the OS zone, so results would depend on the
  // machine. Stamp the country the base was seeded for, so both sides run in the base's own zone.
  data.settings.country ??= country;
  return fromSnapshot(`demo-${country.toLowerCase()}`, { version: SCHEMA_VERSION, savedAt: SEED_NOW, data });
}

/** Builds (once per process, then cached) the base a case names. Custom bases are keyed by case name. */
export function loadBase(spec: BaseSpec, caseName: string): Base {
  const key = typeof spec === 'function' ? `custom:${caseName}` : spec;
  const hit = cache.get(key);
  if (hit) return hit;
  let base: Base;
  if (spec === 'demo-sa') base = demo('SA');
  else if (spec === 'demo-eg') base = demo('EG');
  else if (spec === 'edge') base = fromSnapshot('edge', JSON.parse(readFileSync(EDGE_FIXTURE, 'utf-8')) as Snapshot);
  else if (spec === 'empty') base = { key: 'empty', snapshot: null, savedAt: SEED_NOW, tz: zoneForCountry('EG'), credentials: {} };
  else {
    // Same reasoning as `demo()`: a custom base's own `uid()` calls must not continue from whatever
    // counters an earlier base build left behind.
    const data = withPinnedClock(SEED_NOW, zoneForCountry(null), `base:${key}`, () => {
      resetIdCounters();
      return clone(spec());
    });
    base = fromSnapshot(key, { version: SCHEMA_VERSION, savedAt: SEED_NOW, data });
  }
  cache.set(key, base);
  return base;
}

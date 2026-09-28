/**
 * `backendMirror.ts` (21.03 G-37): a reactive mirror so a **synchronous** service function
 * (`dashboardService.ts`'s `getInTransitTransfers`/`getPendingApprovalRequests`/
 * `getLastBackupFailedAt`/`getJournalDraftCount`/`getStockValueSnapshot`/`hasAnyProducts`, and
 * 14b's insight engine) can become Rust-backed without touching the pages/controllers that call it
 * inside a `computed()` (`AccountantHome.vue`, `StorekeeperHome.vue`, `useNotifications.ts`,
 * `useInsights.ts`) — none of those call sites can `await` an IPC round trip.
 *
 * The pattern: `mirrored(key, load, fallback)` returns the last value it fetched (or `fallback` the
 * first time), starts `load()` in the background at most once per key, and stores the resolved
 * value in a `shallowReactive` map so any `computed()` reading it re-runs the moment the value
 * lands — exactly like a Pinia store's own reactive state, but keyed by an arbitrary string instead
 * of a fixed set of fields, since the domains that need this (dashboard/insights) have per-argument
 * variants (`dashboard:inTransit:${homeBranch}`).
 *
 * This file's job ends at "keep the last known value fresh"; it is not a cache with LRU eviction or
 * request de-duplication beyond "don't start a second `load()` while one is already pending" —
 * `usesRust('dashboard')` is `false` on every mock-backed build, so in practice this whole module is
 * dormant (the switch lines in `dashboardService.ts`/`insightEngine.ts` only reach it once Part 04
 * flips the `dashboard` domain).
 */
import { reactive } from 'vue';
import { on, type MockEvent } from '@/mocks/events';
import { log } from '@/modules/diagnostics/services/logService';

interface Pending {
  state: 'pending';
}
interface Loaded<T> {
  state: 'loaded';
  value: T;
  loadedAt: number;
}
interface Failed {
  state: 'failed';
}

type Entry<T> = Pending | Loaded<T> | Failed;

/** `shallowReactive` would do too — plain `reactive` is used because each entry is itself a small
 * plain object (`state`/`value`/`loadedAt`) that Vue's proxy wrapping handles fine at this size,
 * and it keeps the map itself trackable by `computed()` without a second reactive wrapper per
 * entry. */
const entries = reactive(new Map<string, Entry<unknown>>());

/** Every category the mock event bus can emit (`src/mocks/events.ts`) invalidates every mirrored
 * entry — a mirror has no per-key subscription to "which category would affect this key", and a
 * dashboard/insight read is cheap enough that over-invalidating (a `catalog:changed` clearing a
 * `dashboard:pendingApprovals` entry too) just costs one extra background reload, never a wrong
 * answer. */
const INVALIDATING_EVENTS: MockEvent[] = ['ledger:changed', 'catalog:changed', 'parties:changed'];

let subscribed = false;
function ensureSubscribed(): void {
  if (subscribed) return;
  subscribed = true;
  for (const event of INVALIDATING_EVENTS) {
    on(event, () => clearMirrors());
  }
}

/**
 * Returns the last value `load()` resolved to for `key`, or `fallback` if nothing has resolved yet
 * (including while the very first `load()` is still in flight). Starts `load()` at most once per
 * key until the entry is cleared (by `clearMirrors`, a change event, or `opts.ttlMs` elapsing) —
 * a second `mirrored()` call for the same key while a load is `pending` does not start a second
 * request.
 *
 * `T` must be safe to read synchronously and cheap to hold in memory (these are always small
 * dashboard/insight payloads — a handful of rows or a single number/string).
 */
export function mirrored<T>(key: string, load: () => Promise<T>, fallback: T, opts?: { ttlMs?: number }): T {
  ensureSubscribed();

  const existing = entries.get(key) as Entry<T> | undefined;
  if (existing?.state === 'loaded') {
    const stale = opts?.ttlMs !== undefined && Date.now() - existing.loadedAt > opts.ttlMs;
    if (!stale) return existing.value;
  } else if (existing?.state === 'pending') {
    return fallback;
  }
  // `existing === undefined`, `existing.state === 'failed'` (retry on next read), or the loaded
  // entry just went stale: (re)start the load.

  entries.set(key, { state: 'pending' });
  void load()
    .then((value) => {
      entries.set(key, { state: 'loaded', value, loadedAt: Date.now() });
    })
    .catch((err: unknown) => {
      entries.set(key, { state: 'failed' });
      log.error('core.backendMirror', `mirrored("${key}") failed`, err instanceof Error ? err : undefined, { key });
    });

  return existing?.state === 'loaded' ? existing.value : fallback;
}

/**
 * Drops mirrored entries so the next `mirrored()` read for them starts a fresh `load()`. With no
 * `prefix`, clears every entry (called automatically on `ledger:changed`/`catalog:changed`/
 * `parties:changed`); with a `prefix`, clears only keys starting with it (e.g. `clearMirrors('dashboard:')`
 * after an explicit user action that a change event wouldn't otherwise cover).
 */
export function clearMirrors(prefix?: string): void {
  if (prefix === undefined) {
    entries.clear();
    return;
  }
  for (const key of Array.from(entries.keys())) {
    if (key.startsWith(prefix)) entries.delete(key);
  }
}

/**
 * One pass of one case on one backend (plan 21 Part 04, B-8/B-9).
 *
 * mock pass: restore the base into the mock `db` (in place, like `scripts/verify/replay.ts`), reset
 *            `uid()` counters, pin the clock, log in, `run`, books, invariants.
 * rust pass: blank the mock `db` (a service that still reads it in Rust mode shows up as a diff —
 *            the same state the real app is in once the mock boot is off), `__reset` the host with
 *            the same base, pin both clocks, `setParityTransport`, log in, `run`, books, `__invariants`.
 *
 * Invariants run after **every step** on both sides; a failure that was not already present in the
 * base fails the case (B-9, phase C C-4).
 */
import { createPinia, setActivePinia } from 'pinia';
import { db, resetDb, session, type MockDb } from '../../src/mocks/db';
import { bumpIdCounter, clone, resetIdCounters } from '../../src/mocks/utils';
import { clearPostingTraces } from '../../src/mocks/backend/posting-trace';
import { setParityTransport } from '../../src/modules/core/services/backend';
import { clearMirrors } from '../../src/modules/core/services/backendMirror';
import * as authService from '../../src/modules/users/services/authService';
import { useAuthStore } from '../../src/modules/users/controllers/useAuthStore';
import type { CaseContext, ParityCase, RecordedError, StepRecord } from './case';
import type { Base } from './bases';
import { failing, mockInvariants, readBooks, toRecordedError, type Books, type InvariantRow } from './books';
import { movePinnedClock, pinClock, unpinClock } from './clock';
import { normalize } from './diff';
import { resetPolyfills } from './polyfills';
import { addTerminalPair, HostError, pairsToMap, type HostTransport } from './transport';

export type Side = 'mock' | 'rust';

/** A whole pass (not one call) may take at most this long before it is recorded as timed out. */
export const PASS_TIMEOUT_MS = 60_000;

export interface PassResult {
  side: Side;
  steps: Record<string, StepRecord>;
  books: Books | null;
  invariants: InvariantRow[] | null;
  /** Invariant failures that appeared after a step and were not in the base. */
  invariantBreaks: { step: string; failures: string[] }[];
  /** Why the case stopped early (a failed `step`, a succeeding `expectError`, a throw in `run`). */
  aborted: string | null;
  /** A harness failure (host crash/timeout, meta-command error) — the pass result is unusable. */
  harnessError: string | null;
  idPairs: Map<string, string>;
  resetCounts?: unknown;
  timings: { resetMs: number; runMs: number; booksMs: number; invariantMs: number[] };
}

class Abort extends Error {}

// --- shared session plumbing (cases and bundles) ---------------------------------------------

/** `persist.ts`'s `resyncIdCounters`, reproduced: bump each `prefix-N` counter to the highest N in `data`. */
function resyncIdCounters(data: unknown): void {
  const idPattern = /^([a-z]+)-(\d+)$/;
  const visited = new Set<unknown>();
  const walk = (value: unknown): void => {
    if (!value || typeof value !== 'object' || visited.has(value)) return;
    visited.add(value);
    if (Array.isArray(value)) {
      for (const item of value) walk(item);
      return;
    }
    // `entityId` too — mirrors `persist.ts` (audit placeholders `unk-N` live only there).
    for (const field of ['id', 'entityId'] as const) {
      const id = (value as Record<string, unknown>)[field];
      if (typeof id === 'string') {
        const m = idPattern.exec(id);
        if (m) bumpIdCounter(m[1], Number(m[2]));
      }
    }
    for (const key of Object.keys(value)) walk((value as Record<string, unknown>)[key]);
  };
  walk(data);
}

/** Fresh Pinia, cleared mirrors, signed out, `db` = `data` (or blank), and a clean
 * localStorage/sessionStorage/indexedDB (`polyfills.ts`) — every pass (mock or rust) starts from the
 * same empty-browser-profile state, so `templateService`/`attachments.ts` never leak a row across
 * cases or across the two runs of `--mock-only`'s determinism check. */
export async function prepareFrontend(data: MockDb | null): Promise<void> {
  setActivePinia(createPinia());
  clearMirrors();
  session.userId = '';
  resetDb();
  resetIdCounters();
  // The posting-trace ring is per-process memory (16-diagnostics §5 / P2-35: "cleared on reload"),
  // not `db` — without this, the traces recorded while the harness *seeded* the base in this same
  // process leaked into every mock pass (`hasTrace: true` on imported entries Rust never traced).
  clearPostingTraces();
  await resetPolyfills();
  if (data) {
    Object.assign(db, clone(data));
    resyncIdCounters(db);
  }
}

export async function signIn(username: string, password: string): Promise<unknown> {
  const user = await authService.login(username, password);
  useAuthStore().user = user;
  return user;
}

export async function signOut(): Promise<void> {
  await authService.logout();
  useAuthStore().user = null;
}

async function withTimeout<T>(p: Promise<T>, ms: number, onTimeout: () => void): Promise<T> {
  let timer: ReturnType<typeof setTimeout> | undefined;
  const timeout = new Promise<never>((_, reject) => {
    timer = setTimeout(() => {
      onTimeout();
      reject(new HostError(`pass timed out after ${ms / 1000}s`));
    }, ms);
  });
  try {
    return await Promise.race([p, timeout]);
  } finally {
    clearTimeout(timer);
  }
}

// --- one case pass ---------------------------------------------------------------------------

export async function runPass(c: ParityCase, base: Base, side: Side, transport: HostTransport | null): Promise<PassResult> {
  const result: PassResult = {
    side,
    steps: {},
    books: null,
    invariants: null,
    invariantBreaks: [],
    aborted: null,
    harnessError: null,
    idPairs: new Map(),
    timings: { resetMs: 0, runMs: 0, booksMs: 0, invariantMs: [] },
  };
  const rust = side === 'rust';
  if (rust && !transport) throw new Error('rust pass needs a host transport');
  const baseIds = side === 'mock' ? mockIdsOf(base) : null;
  let signedIn = false;
  let baseline = new Set<string>();

  const invariantsNow = async (): Promise<InvariantRow[]> => {
    const t0 = performance.now();
    const rows = rust ? await transport!.invariants() : mockInvariants();
    result.timings.invariantMs.push(performance.now() - t0);
    return rows;
  };

  const afterStep = async (name: string): Promise<void> => {
    if (rust && transport!.lastFailure) throw new HostError(transport!.lastFailure);
    const fresh = failing(await invariantsNow()).filter((f) => !baseline.has(f));
    if (fresh.length) result.invariantBreaks.push({ step: name, failures: fresh });
  };

  const claim = (name: string): void => {
    if (!name || Object.prototype.hasOwnProperty.call(result.steps, name)) throw new Abort(`step name "${name}" is empty or used twice in this case`);
  };

  /** `login:<user>`/`logout`, de-duplicated with a `#2`, `#3`, … suffix — a case that logs the same
   * user in twice (e.g. re-authenticating mid-story) or logs out more than once used to collide with
   * `claim()`'s "used twice" guard on the exact same step name (L4 finding). Only `login`/`logout`
   * auto-suffix; a case's own `step`/`expectError` names must still be unique on their own. */
  const uniqueAutoStepName = (base: string): string => {
    if (!Object.prototype.hasOwnProperty.call(result.steps, base)) return base;
    for (let n = 2; ; n++) {
      const candidate = `${base}#${n}`;
      if (!Object.prototype.hasOwnProperty.call(result.steps, candidate)) return candidate;
    }
  };

  const ctx: CaseContext = {
    async step<T>(name: string, fn: () => Promise<T> | T): Promise<T> {
      claim(name);
      let value: T;
      try {
        value = await fn();
      } catch (e) {
        if (e instanceof HostError || e instanceof Abort) throw e;
        const error = toRecordedError(e);
        result.steps[name] = { ok: false, error, expectedError: false };
        await afterStep(name);
        throw new Abort(`step "${name}" failed: ${error.code}: ${error.message}`);
      }
      result.steps[name] = { ok: true, value: normalize(value) };
      await afterStep(name);
      return value;
    },
    async expectError(name: string, fn: () => Promise<unknown> | unknown): Promise<RecordedError> {
      claim(name);
      let value: unknown;
      try {
        value = await fn();
      } catch (e) {
        if (e instanceof HostError || e instanceof Abort) throw e;
        const error = toRecordedError(e);
        result.steps[name] = { ok: false, error, expectedError: true };
        await afterStep(name);
        return error;
      }
      result.steps[name] = { ok: true, value: normalize(value) };
      await afterStep(name);
      throw new Abort(`expectError "${name}" succeeded — the call was expected to fail`);
    },
    baseId(mockId: string): string {
      if (baseIds) {
        if (!baseIds.has(mockId)) throw new Abort(`baseId("${mockId}"): no row with that id in base "${base.key}"`);
        return mockId;
      }
      const mapped = result.idPairs.get(mockId);
      if (!mapped) throw new Abort(`baseId("${mockId}"): not in the importer's idPairs for base "${base.key}"`);
      return mapped;
    },
    async setClock(iso: string): Promise<void> {
      movePinnedClock(iso);
      if (rust) await transport!.clock(iso);
    },
    async login(username: string, password?: string): Promise<void> {
      const pw = password ?? base.credentials[username];
      if (pw === undefined) throw new Abort(`login("${username}"): no password given and none in base "${base.key}"`);
      await ctx.step(uniqueAutoStepName(`login:${username}`), () => signIn(username, pw));
      signedIn = true;
    },
    async logout(): Promise<void> {
      await ctx.step(uniqueAutoStepName('logout'), () => signOut());
      signedIn = false;
    },
  };

  try {
    await prepareFrontend(rust ? null : base.snapshot ? (base.snapshot.data as MockDb) : null);
    if (rust) {
      const t0 = performance.now();
      if (base.snapshot) {
        const reply = await transport!.reset({ snapshot: base.snapshot, templates: c.templatesJson ?? null, templateBranchId: null });
        result.idPairs = pairsToMap(reply.idPairs);
        addTerminalPair(result.idPairs, reply.terminalId);
        result.resetCounts = reply.counts;
      } else {
        const reply = await transport!.resetEmpty();
        addTerminalPair(result.idPairs, reply?.terminalId);
      }
      result.timings.resetMs = performance.now() - t0;
    }
    const clockIso = c.clock ?? base.savedAt;
    pinClock(clockIso, c.tz ?? base.tz, c.name);
    if (rust) {
      await transport!.clock(clockIso);
      transport!.clearFailure();
      setParityTransport(transport!.invoke);
    }
    baseline = new Set(failing(await invariantsNow()));

    const t1 = performance.now();
    try {
      const user = c.user === undefined ? 'admin' : c.user;
      const story = (async () => {
        if (user !== null) await ctx.login(user, c.password);
        await c.run(ctx);
      })();
      await withTimeout(story, PASS_TIMEOUT_MS, () => transport?.kill());
    } catch (e) {
      if (e instanceof HostError) throw e;
      result.aborted = e instanceof Abort ? e.message : `run() threw: ${e instanceof Error ? e.stack ?? e.message : String(e)}`;
    }
    result.timings.runMs = performance.now() - t1;
    if (rust && transport!.lastFailure) throw new HostError(transport!.lastFailure);

    if (signedIn) {
      const t2 = performance.now();
      result.books = await readBooks();
      result.timings.booksMs = performance.now() - t2;
    }
    result.invariants = await invariantsNow();
  } catch (e) {
    result.harnessError = e instanceof Error ? e.message : String(e);
  } finally {
    setParityTransport(null);
    unpinClock();
  }
  return result;
}

const idSets = new Map<string, Set<string>>();

/** Every `id` field in a base snapshot — `s.baseId` on the mock refuses anything else, so a typo
 * fails on the mock pass instead of only on Rust. */
function mockIdsOf(base: Base): Set<string> {
  let set = idSets.get(base.key);
  if (set) return set;
  set = new Set<string>();
  const walk = (v: unknown): void => {
    if (!v || typeof v !== 'object') return;
    if (Array.isArray(v)) return v.forEach(walk);
    const id = (v as { id?: unknown }).id;
    if (typeof id === 'string') set!.add(id);
    Object.values(v).forEach(walk);
  };
  walk(base.snapshot?.data);
  idSets.set(base.key, set);
  return set;
}

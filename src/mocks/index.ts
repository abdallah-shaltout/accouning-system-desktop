/**
 * Entry point of the mock backend. Only module `services/` files may import from here.
 *
 * Booting is now async and does NOT seed automatically (see `bootMockDb()`): a persisted
 * IndexedDB snapshot is loaded if one exists; otherwise `db` is left empty and the welcome
 * screen decides (demo data seeds it, "start company" creates an empty shell). `main.ts` awaits
 * `bootMockDb()` before mounting the app.
 */
import { db, session } from './db';
import { loadSnapshot } from './persist';

let booted = false;

/** Call once before mounting the app. Loads a persisted snapshot if one exists; otherwise leaves `db` empty. */
export async function bootMockDb(): Promise<{ hadSnapshot: boolean }> {
  if (booted) return { hadSnapshot: true };
  booted = true;
  const hadSnapshot = await loadSnapshot();
  return { hadSnapshot };
}

/** True once `bootMockDb()` has resolved — the router guard uses this to decide `/welcome` vs `/login`. */
export function isBooted(): boolean {
  return booted;
}

export { db, session };
export {
  ApiError,
  clone,
  delay,
  getLatencyMode,
  inDateRange,
  includesText,
  localDateKey,
  round2,
  setLatencyMode,
  sum,
  uid,
  type LatencyMode,
} from './utils';

/** 18.F2/F3: runtime accounting invariants — the one copy shared by `verify:mocks` and the app's
 * debug-mode watcher / `/dev/diagnostics` "المحاسبة" tab. */
export { runAllInvariants, type InvariantResult } from './backend/invariants';

/** 18.F1/F3: posting traces — the in-memory ring + debug-mode per-entry store built at the
 * `resolvePosting`/`postJournal` choke point. */
export { recentPostingTraces, postingTraceFor, type PostingTrace, type PostingTraceStep } from './backend/posting-trace';

/** 18.F3: party balance/allocation reads, reused by the drift report so it never recomputes its
 * own copy of the GL-vs-subledger math `invariants.ts` already defines. */
export { customerBalance, supplierBalance } from './backend/balances';
export { unallocatedCreditFor } from './backend/payments';

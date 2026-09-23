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

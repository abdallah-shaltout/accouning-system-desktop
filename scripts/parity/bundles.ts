/**
 * `bun run parity --bundles` (plan 21 Part 04, B-10): replays every `scripts/verify/cases/*.json`
 * `ReproBundle` on both backends through `serviceRegistry()`.
 *
 * Both sides start from the bundle's `startSnapshot` (mock: restored into `db`; Rust: `__reset`),
 * signed in as the snapshot's first active admin (the role every bundle is recorded under), with the
 * clock pinned to the bundle's `startedAt`. The Rust side remaps every argument id with the
 * importer's `idPairs` plus the ids learned by diffing each step's mock result against its Rust
 * result. Each step's `ok` must equal the recorded `ok` on both sides, rejections must match
 * `{code, message}` byte for byte, and no invariant may break that held at the start.
 */
import { readdirSync, readFileSync } from 'node:fs';
import { join } from 'node:path';
import type { MockDb } from '../../src/mocks/db';
import { SCHEMA_VERSION } from '../../src/mocks/persist';
import { serviceRegistry } from '../../src/modules/diagnostics/services/defineService';
import type { ActionJournalEntry, ReproBundle } from '../../src/modules/diagnostics/services/actionJournal';
import { setParityTransport } from '../../src/modules/core/services/backend';
import { failing, mockInvariants, toRecordedError } from './books';
import type { RecordedError } from './case';
import { pinClock, unpinClock, zoneForCountry } from './clock';
import { diffTrees, IdMap, normalize, type Diff } from './diff';
import { prepareFrontend, signIn } from './pass';
import { addTerminalPair, HostError, pairsToMap, type HostTransport } from './transport';

export const BUNDLE_DIR = join(import.meta.dirname, '../verify/cases');

interface BundleStep {
  seq: number;
  source: string;
  recordedOk: boolean;
  ok: boolean;
  value?: unknown;
  error?: RecordedError;
}

interface BundleSide {
  steps: BundleStep[];
  invariantBreaks: { seq: number; failures: string[] }[];
  harnessError: string | null;
}

export interface BundleResult {
  file: string;
  mock: BundleSide;
  rust: BundleSide | null;
  /** Human-readable failures; empty = green. */
  problems: string[];
  diffs: Diff[];
}

export function bundleFiles(): string[] {
  try {
    return readdirSync(BUNDLE_DIR)
      .filter((f) => f.endsWith('.json'))
      .sort()
      .map((f) => join(BUNDLE_DIR, f));
  } catch {
    return [];
  }
}

async function replaySide(bundle: ReproBundle, transport: HostTransport | null, ids: IdMap, mockSteps: BundleStep[] | null): Promise<{ side: BundleSide; diffs: Diff[] }> {
  const snapshot = bundle.startSnapshot as MockDb;
  const side: BundleSide = { steps: [], invariantBreaks: [], harnessError: null };
  const diffs: Diff[] = [];
  const rust = transport !== null;
  const registry = serviceRegistry();
  try {
    await prepareFrontend(rust ? null : snapshot);
    if (rust) {
      const reply = await transport.reset({ snapshot: { version: SCHEMA_VERSION, savedAt: bundle.startedAt, data: snapshot }, templates: null, templateBranchId: null });
      const pairs = pairsToMap(reply.idPairs);
      addTerminalPair(pairs, reply.terminalId);
      for (const [m, r] of pairs) ids.set(m, r);
    }
    pinClock(bundle.startedAt, zoneForCountry(snapshot.settings?.country), `bundle:${bundle.startedAt}`);
    if (rust) {
      await transport.clock(bundle.startedAt);
      transport.clearFailure();
      setParityTransport(transport.invoke);
    }
    const invariants = async () => failing(rust ? await transport.invariants() : mockInvariants());
    const baseline = new Set(await invariants());
    const admin = snapshot.users?.find((u) => u.role === 'admin' && u.active);
    if (admin) await signIn(admin.username, snapshot.credentials?.[admin.username] ?? '');

    for (const [i, action] of (bundle.actions as ActionJournalEntry[]).entries()) {
      const fn = registry.get(action.source);
      const step: BundleStep = { seq: action.seq, source: action.source, recordedOk: action.ok, ok: true };
      if (!fn) {
        step.ok = false;
        step.error = { code: 'HARNESS', message: `unknown service "${action.source}" (renamed or moved since the bundle was recorded?)` };
      } else {
        const args = rust ? (ids.remap(action.args) as unknown[]) : action.args;
        try {
          step.value = normalize(await fn(...args));
        } catch (e) {
          step.ok = false;
          step.error = toRecordedError(e);
        }
      }
      if (rust && transport.lastFailure) throw new HostError(transport.lastFailure);
      side.steps.push(step);
      // Learn ids from this step's results before the next step's args are remapped.
      const peer = mockSteps?.[i];
      if (rust && peer) {
        diffs.push(...diffTrees({ ok: peer.ok, value: peer.value, error: peer.error }, { ok: step.ok, value: step.value, error: step.error }, ids, {}, `actions[${i}]`));
      }
      const fresh = (await invariants()).filter((f) => !baseline.has(f));
      if (fresh.length) side.invariantBreaks.push({ seq: action.seq, failures: fresh });
    }
  } catch (e) {
    side.harnessError = e instanceof Error ? e.message : String(e);
  } finally {
    setParityTransport(null);
    unpinClock();
  }
  return { side, diffs };
}

export async function runBundle(file: string, transport: HostTransport | null): Promise<BundleResult> {
  const bundle = JSON.parse(readFileSync(file, 'utf-8')) as ReproBundle;
  const ids = new IdMap();
  const mock = (await replaySide(bundle, null, ids, null)).side;
  const result: BundleResult = { file, mock, rust: null, problems: [], diffs: [] };
  if (transport) {
    const r = await replaySide(bundle, transport, ids, mock.steps);
    result.rust = r.side;
    result.diffs = r.diffs;
  }
  for (const [label, side] of [['mock', mock], ['rust', result.rust]] as const) {
    if (!side) continue;
    if (side.harnessError) result.problems.push(`${label}: harness error — ${side.harnessError}`);
    for (const s of side.steps) {
      if (s.ok !== s.recordedOk) result.problems.push(`${label}: step ${s.seq} ${s.source} ok=${s.ok}, recorded ok=${s.recordedOk}${s.error ? ` (${s.error.code}: ${s.error.message})` : ''}`);
    }
    for (const b of side.invariantBreaks) result.problems.push(`${label}: invariant broken after step ${b.seq}: ${b.failures.join('; ')}`);
  }
  // B-10 gates on `ok` and on rejections (`{code, message}` byte for byte). Value diffs are kept in
  // the output for reading (they also taught the id map), but the typed parity cases own DTO parity.
  for (const d of result.diffs) {
    if (/\.(ok|error)(?:$|\.)/.test(d.path)) result.problems.push(`diff ${d.kind} at ${d.path}: mock=${JSON.stringify(d.mock)} rust=${JSON.stringify(d.rust)}`);
  }
  return result;
}

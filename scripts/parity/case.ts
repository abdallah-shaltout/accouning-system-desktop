/**
 * The parity case API (plan 21 Part 04, phase B, B-6). One file per case under
 * `scripts/parity/cases/<domain>/<case-id>.ts`, default-exporting `defineCase({...})`.
 *
 * A case is one business story written against the **real TS services** (`invoiceService`,
 * `partyService`, …). `bun run parity` runs it twice: once on the mock (`usesRust() = false`) and
 * once on Rust (`setParityTransport` → `parity_host`), then diffs every recorded step, the books
 * (trial balance, customers, suppliers, stock) and the invariants (decision P4-2, P4-6).
 *
 *   import { defineCase } from '../../case';
 *   import * as invoiceService from '../../../../src/modules/invoices/services/invoiceService';
 *
 *   export default defineCase({
 *     name: 'invoices/sale-cash-inclusive',          // '<domain>/<id from the 03-domains §8(b) list>'
 *     source: '03-domains/08-invoices.md §8(b)',     // traceability (required)
 *     base: 'demo-sa',                               // 'demo-sa' | 'demo-eg' | 'edge' | 'empty' | () => MockDb
 *     user: 'admin',                                 // logged in on both sides before run(); null = signed out
 *     async run(s) {
 *       const customer = s.baseId('cust-1');         // 'cust-1' on the mock, its UUID on Rust
 *       const sale = await s.step('create', () => invoiceService.createSale({ customerId: customer, … }));
 *       await s.step('detail', () => invoiceService.getInvoice(sale.id));
 *       await s.expectError('over-refund', () => invoiceService.createRefund(sale.id, { … }));
 *     },
 *   });
 *
 * Rules (enforced by `defineCase` and the runner):
 * - `name` is `<domain>/<case-id>` and must equal the file's path under `cases/` (without `.ts`).
 * - Step names are unique within a case (use `pay-${i}` in a loop).
 * - `s.step` that throws aborts the case (it is recorded, and the case fails). A step that is
 *   *meant* to fail is `s.expectError`: it records `{code, message}` and returns it; if the call
 *   succeeds instead, that is recorded and the case aborts too.
 * - Every `allow` entry's `reason` must cite a written decision or quirk id (`A-D1`, `Q6`, `R-7`,
 *   `12-accounting #10`, `D-8`, …) — see `allow.ts`'s `citesDecision`. The runner rejects the run
 *   otherwise.
 * - `epsilon` only for the fields 13 R-7 / 13b §8(b) / 14b §8(b) name; `unordered` only where Q6 /
 *   Q-I10 say so.
 *
 * Path patterns (`allow`, `epsilon`, `unordered`) address the recorded result tree:
 *   `steps.<step>.value.<field>…`, `steps.<step>.error.message`, `books.trialBalance[3].debit`,
 *   `books.customers`, `books.suppliers`, `books.inventory`, `invariants`.
 * `*` matches one key segment, `[]` any array index, `**` any depth. An `allow` pattern also covers
 * everything below the node it matches.
 *
 * This file is the stable contract the five B2 case lanes write against — change it only through
 * the manager (entry file §5, "After Wave 0 the runner core becomes manager-owned").
 */
import type { MockDb } from '../../src/mocks/db';

/** The four shared starting states (B-6, `bases.ts`). A function base builds a custom `MockDb`
 * (it runs under the pinned clock and the base's timezone, and the same data is imported on Rust). */
export type BaseName = 'demo-sa' | 'demo-eg' | 'edge' | 'empty';
export type BaseSpec = BaseName | (() => MockDb);

/** A tolerated difference. `reason` must cite a decision/quirk id (P4-6). */
export interface AllowEntry {
  /** Path pattern (see header). Also covers every path below the matched node. */
  path: string;
  /** Why this is allowed — must cite a written decision or quirk id, e.g. `'12-accounting #10: [] vs absent'`. */
  reason: string;
  /**
   * Narrows the entry to the "`[]` ≡ absent" representation gap only: it then covers just a diff
   * *at* a matched node (not below it) where one side is absent and the other is an empty array —
   * so a non-empty array, or any value inside one, still diffs. For fields a decision declares
   * always-an-array in Rust (06 Q-7 `Product.prices`, 05 D-6 `phones`).
   */
  emptyArrayOnly?: boolean;
  /**
   * Narrows the entry to the "instant ≡ its business day" representation gap only: it then covers
   * just a `value` diff *at* a matched node where the mock holds an ISO instant and Rust a
   * `YYYY-MM-DD` day that instant can fall on in some UTC−14…UTC+14 zone — so a missing/extra field
   * or a different day still diffs. For a field a decision declares a calendar day in Rust
   * (07 D-U8 `supplierInvoiceDate`).
   */
  dayOfInstantOnly?: boolean;
}

/** A structured error, compared byte for byte (`code` and `message`) between the two backends. */
export interface RecordedError {
  code: string;
  message: string;
}

/** What one step recorded. `value` is the JSON-normalized return value (`undefined` keys dropped;
 * a step that returns nothing records `null`). */
export type StepRecord =
  | { ok: true; value: unknown; expectedError?: false }
  | { ok: false; error: RecordedError; expectedError: boolean };

/** The handle a case's `run(s)` receives. Identical API on both passes — a case never branches on
 * which backend is answering. */
export interface CaseContext {
  /** Runs `fn`, records `{ ok, value }` under `name`, returns the value so later steps chain on real
   * ids. A throw is recorded as `{ code, message }` and aborts the case. */
  step<T>(name: string, fn: () => Promise<T> | T): Promise<T>;
  /** Runs `fn`, which must fail: records and returns its `{ code, message }`. Success aborts the case. */
  expectError(name: string, fn: () => Promise<unknown> | unknown): Promise<RecordedError>;
  /** A base-snapshot id (`'cust-1'`, `'branch-main'`) as it exists in the current pass: unchanged on
   * the mock, the importer's UUID on Rust (from `__reset`'s `idPairs`). Throws for an unknown id. */
  baseId(mockId: string): string;
  /** Moves the pinned clock (both sides) — for aging, due dates, recurring "post due" cases. */
  setClock(iso: string): Promise<void>;
  /** Switches the signed-in user mid-case (both sides). `password` defaults to the base snapshot's
   * `credentials[username]`. Recorded as a step named `login:<username>`. */
  login(username: string, password?: string): Promise<void>;
  /** Signs out (both sides). Recorded as a step named `logout`. */
  logout(): Promise<void>;
}

export interface ParityCase {
  /** `<domain>/<case-id>` — the domain folder under `cases/` and the id from the 03-domains §8(b) list. */
  name: string;
  /** Traceability, e.g. `'03-domains/08-invoices.md §8(b)'`. Required. */
  source: string;
  base: BaseSpec;
  /** Logged in on both sides before `run()`. Default `'admin'`. `null` = signed out (e.g. setup wizard on `empty`). */
  user?: string | null;
  /** Password for `user`. Default: the base snapshot's `credentials[user]`. */
  password?: string;
  /** Pinned "now" (ISO instant). Default: the base's `savedAt` (`2026-06-30T09:00:00.000Z` for the demo bases). */
  clock?: string;
  /** IANA zone override. Default: the base's country zone (SA → Asia/Riyadh, EG → Africa/Cairo). */
  tz?: string;
  /** Owning B2 lane when it differs from the domain's default lane (e.g. `settings/settings-revaluation-post` is L4). */
  lane?: Lane;
  /** Print templates JSON (the `pdf_templates_v1` localStorage value) imported with the base on Rust. */
  templatesJson?: string;
  allow?: AllowEntry[];
  /** Paths compared with `|a − b| ≤ 1e-9` (13 R-7, 13b §8(b), 14b §8(b) fields only). */
  epsilon?: string[];
  /** Array paths compared as multisets (only where Q6 / Q-I10 say so). */
  unordered?: string[];
  run(s: CaseContext): Promise<void>;
}

/** The five B2 lanes (phase-b2-parity-cases.md "Lanes"). */
export type Lane = 'L1' | 'L2' | 'L3' | 'L4' | 'L5';

/** Case domain folder → default lane. A case overrides with `lane` (L4 owns `settings-revaluation-post`). */
export const DOMAIN_LANE: Record<string, Lane> = {
  settings: 'L1',
  users: 'L1',
  approvals: 'L1',
  templates: 'L1',
  diagnostics: 'L1',
  backup: 'L1',
  import: 'L1',
  attachments: 'L1',
  parties: 'L2',
  products: 'L2',
  inventory: 'L2',
  purchases: 'L2',
  invoices: 'L3',
  shifts: 'L3',
  payments: 'L3',
  vouchers: 'L3',
  accounting: 'L4',
  expenses: 'L4',
  setup: 'L4',
  reports: 'L5',
  analytics: 'L5',
  dashboard: 'L5',
};

const NAME_RE = /^[a-z_][a-z0-9_-]*\/[a-z0-9][a-z0-9._-]*$/;

/** Validates and returns the case. Throws on a malformed definition so a bad case fails at load
 * time, before either backend runs. */
export function defineCase(c: ParityCase): ParityCase {
  if (!NAME_RE.test(c.name)) throw new Error(`parity case name "${c.name}" must be '<domain>/<case-id>' (lower-case)`);
  if (!c.source?.trim()) throw new Error(`parity case "${c.name}" needs a source (e.g. '03-domains/08-invoices.md §8(b)')`);
  if (typeof c.run !== 'function') throw new Error(`parity case "${c.name}" needs run(s)`);
  if (typeof c.base !== 'function' && !['demo-sa', 'demo-eg', 'edge', 'empty'].includes(c.base)) {
    throw new Error(`parity case "${c.name}": unknown base "${String(c.base)}"`);
  }
  if (c.base === 'empty' && c.user !== null) {
    throw new Error(`parity case "${c.name}": base 'empty' has no users — set user: null and log in with s.login() after bootstrap`);
  }
  if (c.clock !== undefined && Number.isNaN(Date.parse(c.clock))) throw new Error(`parity case "${c.name}": clock "${c.clock}" is not an ISO instant`);
  const domain = c.name.split('/')[0];
  if (domain !== '_selftest' && !c.lane && !DOMAIN_LANE[domain]) {
    throw new Error(`parity case "${c.name}": domain "${domain}" has no lane — add it to DOMAIN_LANE or set lane`);
  }
  return c;
}

/** The lane a case belongs to (`null` for `_selftest/*`, which runs in every lane). */
export function laneOf(c: ParityCase): Lane | null {
  if (c.lane) return c.lane;
  const domain = c.name.split('/')[0];
  return DOMAIN_LANE[domain] ?? null;
}

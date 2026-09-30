/**
 * L1 platform lane. Import parity for the `edge` base (00-import.md §8(b) — "import-edge (the edge
 * fixture)"), the hand-written `src-tauri/tests/fixtures/mock-snapshot-edge.json` §8(a) also uses:
 * `freetext-0` line with `product_id NULL`, one `pos-1` shift/held-sale pair, empty
 * `settings.currency` (→ falls back to EGP on import), `theme` ignored, one minimal CoA (only
 * `cash`/`4010` sales/`2110` vat-output — no `receivable`/`payable`/`inventory` role).
 *
 * **Not written in Wave 1 (L1 status note): this base broke the harness itself before any case code
 * ran** — `checkArApControl`/`checkInventoryGl` (`src/mocks/backend/invariants.ts`, called from every
 * pass's `invariantsNow()`) call `accountFor('receivable'|'payable'|'inventory')`, which throws
 * `NOT_FOUND` because this fixture's CoA has none of those three roles. That crashed the harness
 * before `run()` ever executed, for any case, not just import's. Fixed harness-side (not by editing
 * the fixture or `invariants.ts` — both off-limits): `scripts/parity/books.ts`'s `mockInvariants()`
 * now catches a thrown invariant check and returns one synthetic `{ key: 'invariants-crashed',
 * passed: false, message: <thrown error> }` row instead of aborting. `edge`'s own
 * `checkArApControl`/`checkInventoryGl` throw is now a normal *pre-existing* invariant failure that
 * the runner's baseline subtraction (`pass.ts`: `failing(await invariantsNow())` captured right after
 * `__reset`/`prepareFrontend`, before `run()`) absorbs — exactly like `reports/vat-multicurrency-refund`
 * absorbs the pre-existing `allocations-within-total` failure on `demo-sa`.
 *
 * **Login fails for the fixture's own admin, deliberately pinned:** `u-admin` has no `active` field
 * (`db.ts`'s `User.active` reads `undefined` → falsy), so `authService.login('admin', …)` throws
 * `FORBIDDEN` "هذا الحساب موقوف — تواصل مع مدير النظام" on the mock, and on Rust since 00-import D-13
 * (Wave 2). The reads below run as an extra active admin the case's base adds (see `edgeWithReader`).
 * The fixture's accounts/branch/product carry explicit `active: true` (Wave 2 manager decision) so its
 * own invariants hold on both backends.
 *
 * `invoiceService.getShifts()`/`getShift()` are not read here: the fixture's `shift-1` row (§8(a)'s
 * hand-written minimal fixture, off-limits to edit) has no `movements` array, and
 * `src/mocks/backend/shifts.ts`'s shift-summary math (`shift.movements.filter(...)`) assumes every
 * shift row has one — a real gap in this Rust-test fixture's shape versus the TS mock's runtime
 * `Shift` type, not something this case can route around without touching the fixture. `held-sales`
 * and `invoices` don't touch `movements`, so they're read as-is.
 */
import { readFileSync } from 'node:fs';
import { defineCase } from '../../case';
import * as authService from '../../../../src/modules/users/services/authService';
import { EDGE_FIXTURE } from '../../bases';
import type { MockDb } from '../../../../src/mocks/db';
import * as settingsService from '../../../../src/modules/settings/services/settingsService';
import * as branchesService from '../../../../src/modules/settings/services/branchesService';
import * as userService from '../../../../src/modules/users/services/userService';
import * as productService from '../../../../src/modules/products/services/productService';
import * as invoiceService from '../../../../src/modules/invoices/services/invoiceService';
import * as auditService from '../../../../src/modules/diagnostics/services/auditService';

/**
 * Wave 2: the `edge` fixture exactly, plus one extra **active** admin (`reader`) to read with.
 *
 * Why: the fixture's own admin deliberately has no `active` key, so it is inactive on the mock and —
 * since the Wave 2 importer fix (00-import D-13) — on Rust too, and nobody could sign in. Rust gates
 * every read on a session (01-settings D-2, 13 G-32) while the mock's reads are ungated, so a
 * signed-out read pass compared nothing but `UNAUTHORIZED` against data. The case first pins the
 * `admin` refusal on both sides, then signs in as `reader` and reads every edge quirk below.
 */
function edgeWithReader(): MockDb {
  const snapshot = JSON.parse(readFileSync(EDGE_FIXTURE, 'utf-8')) as { data: MockDb };
  const data = snapshot.data;
  data.users = [...data.users, { id: 'u-reader', username: 'reader', name: 'قارئ', role: 'admin', maxDiscount: 0, active: true } as MockDb['users'][number]];
  data.credentials = { ...data.credentials, reader: 'reader123' };
  return data;
}

export default defineCase({
  name: 'import/import-edge',
  source: '03-domains/00-import.md §8(b)',
  base: edgeWithReader,
  user: null,
  allow: [
    { path: 'steps.settings.value.currency', reason: "00-import D-14: empty settings.currency imports as EGP (the mock keeps '')" },
    { path: 'steps.settings.value.theme', reason: '00-import D-14 / C-14: settings.theme is dead and not imported' },
    { path: 'steps.users.value[].active', reason: "00-import D-13: the fixture admin's absent active flag is stored as false (NOT NULL column); the mock reads absent as falsy" },
    { path: 'steps.products.value[].prices', reason: '06-products Q-7: Rust returns prices: [] where a seeded mock product has no prices key' },
  ],
  async run(s) {
    // The fixture's own admin has no `active` key — inactive on both backends (00-import D-13).
    await s.expectError('login-inactive-admin', () => authService.login('admin', 'admin123'));
    await s.login('reader');

    await s.step('settings', () => settingsService.getSettings());
    await s.step('branches', () => branchesService.getBranches());
    await s.step('users', () => userService.getUsers());
    await s.step('products', () => productService.getProducts());
    await s.step('invoices', () => invoiceService.getInvoices());
    await s.step('invoice-detail', () => invoiceService.getInvoice(s.baseId('inv-1')));
    await s.step('held-sales', () => invoiceService.getHeldSales('pos-1'));
    await s.step('activity-invoices', () => auditService.getAuditEntries({ entity: 'invoices' }));
  },
});

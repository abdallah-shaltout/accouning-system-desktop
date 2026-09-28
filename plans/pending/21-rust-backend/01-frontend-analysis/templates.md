# 21 · 01.B — `templates` contract

> **Status:** done (2026-09-27) · **Inventory:** `docs/backend/contract/templates.md`
> (regenerate with `bun run contract`) · **Mock spec:** none — this module never touched
> `src/mocks/db.ts`/`persist.ts` at all; everything lives in `localStorage` today · **Services:**
> `src/modules/templates/services/templateService.ts` · **Types:** `src/modules/templates/types/index.ts`
>
> **The last module in 01.B.** Print-template CRUD for the designer (docs 17 Phase 11a). Every one
> of its 11 functions was heuristically flagged `frontend` by the contract generator (since
> `localStorage` is a browser global, per `scripts/contract/config.ts`'s `browserGlobals` list) and
> then **overridden to `port` under D9** ("print templates move to the database, shared per
> branch") before this review even started — recorded in `scripts/contract/config.ts` → `overrides`
> with the shared reason `d9`. This review confirms that override is correct and complete, not a
> disposition decision to make from scratch.
>
> **Out of scope, noted for completeness:** plan 22 (invoice templates, a separate in-progress,
> uncommitted piece of work this session found evidence of in `settings.md`'s review) adds
> `StoreSettings.printer.a4Template`/`imageTemplate` fields and a parallel
> `src/modules/invoices/helpers/invoiceTemplates.ts` registry of **built-in, code-defined** A4/image
> layouts (`A4Banner.vue`, `A4Bento.vue`, etc.) — a *different* concept from this module's
> **user-designed, DB-stored** `PdfTemplate` records. The two systems are related (both pick "which
> layout prints an invoice") but not the same table, and plan 22's files were explicitly not
> reviewed or touched here, per the standing "report, don't fix" rule for out-of-scope work.

## 1. Endpoints

| Function | Disposition (confirmed / changed + why) | Rust command | Request DTO | Response DTO | Writes (confirmed) | Shared | Undo | Notes |
|---|---|---|---|---|---|---|---|---|
| `listTemplates` | port (D9 override, confirmed correct) | `templates_list_templates` | `{ kind?: DocumentKind }` | `PdfTemplate[]` | — | — | n/a (read) | **Auto-seeds** two default templates (`invoice_standard` "الفاتورة الضريبية القياسية", `invoice_simplified` "الفاتورة الضريبية المبسطة") on first-ever call if the store is empty or unparseable (`seedDefaults()`, called from `load()`). Under D9 this seeding moves to a **one-time DB migration/bootstrap** step (same category as the `seedEmptyCompany()`/D10 importer already discussed in `setup.md`), not something the read command itself should do on every empty-result call — see §8. |
| `getTemplate` | port (D9 override, confirmed correct) | `templates_get_template` | `{ id: string }` | `PdfTemplate \| undefined` | — | — | n/a (read) | Returns `undefined`, not a `NOT_FOUND` error, when the id doesn't exist — a deliberate "not found is a valid answer" contract (the designer UI treats a missing template as "show the picker," not an error state). Port this exactly: `Option<PdfTemplate>`, not an `AppError`. |
| `getDefaultTemplate` | port (D9 override, confirmed correct) | `templates_get_default_template` | `{ kind: DocumentKind }` | `PdfTemplate \| undefined` | — | — | n/a (read) | Falls back to the **first** template of that kind (`all[0]`) if none is explicitly marked `isDefault` — a defensive fallback for a data state that shouldn't normally occur (every kind should always have exactly one default once `setAsDefault`'s invariant holds), not a bug. |
| `saveTemplate` | port (D9 override, confirmed correct) | `templates_save_template` | `PdfTemplate` | `PdfTemplate` | `print_templates` (create-or-update by `id`) | — | not undoable | Create-or-update by presence in the existing list (find-by-id, not an explicit `id?` parameter like every other module's save functions) — **the client always sends a full `PdfTemplate` including its own `id`**, meaning the client (not the server) decides whether this is a create or an update, by whether it already has an id. This is a notable difference from every other module's `save*(input, id?)` pattern reviewed so far — flagged in §2/§8, since a Rust command taking a full entity-with-id as input needs its own care around trusting a client-supplied id for a *new* row (must generate the UUIDv7 server-side even though the mock's `uid()` happens to run client-side today). |
| `setAsDefault` | port (D9 override, confirmed correct) | `templates_set_as_default` | `{ id: string }` | — | `print_templates.isDefault` (the target row **and every other row of the same `kind`**) | — | not undoable | **Silently no-ops** (no error) if `id` doesn't resolve to an existing template — same "missing is fine" contract as `getTemplate`. Flips every other template of the same `DocumentKind` to `isDefault: false` in the same operation — must be atomic (a partial application would leave two defaults, or zero, for one kind). |
| `duplicateTemplate` | port (D9 override, confirmed correct) | `templates_duplicate_template` | `{ id: string }` | `PdfTemplate \| undefined` | `print_templates` (insert) | — | not undoable | Deep-clones `options` via `JSON.parse(JSON.stringify(...))` — a Rust equivalent just needs an ordinary struct clone (no shared-reference risk in a typed language the way it is in JS). New copy is never `isDefault`. Returns `undefined` on a missing source id, same pattern as above. |
| `deleteTemplate` | port (D9 override, confirmed correct) | `templates_delete_template` | `{ id: string }` | — | `print_templates` (delete) | — | not undoable (hard delete) | **No existence check, no error on a missing id** (a `filter` that simply doesn't remove anything) — consistent with this module's "missing is a no-op, not an error" style throughout. **No reference/state check at all**: nothing stops deleting the current default template for a `DocumentKind`, or a template actively selected as `settings.printer.a4Template`/`imageTemplate` (plan 22, out of scope) — flagged in §8, this is the module's one real fix candidate. |
| `resetTemplateToDefaults` | port (D9 override, confirmed correct) | `templates_reset_template_to_defaults` | `{ id: string }` | `PdfTemplate \| undefined` | `print_templates.options`, `.customSource` (reset to `defaultTemplateOptions()`/`null`) | — | not undoable | Resets *design options*, not the template's `name`/`kind`/`baseTemplateId`/`isDefault` — a "restore factory settings for this template's look," not a delete-and-reseed. |
| `exportTemplate` | port (D9 override, confirmed correct) | `templates_export_template` | `PdfTemplate` | `TemplateExport` | — | — | n/a (pure transform, no `db`/store read at all — takes the full template as input, not an id) | **Unusual signature**: unlike every other function here, this one takes the **template object itself**, not an id to look up — meaning it never reads `print_templates` at all, it's a pure "strip the server-assigned fields" projection. Rust's `templates_export_template` command should likewise just transform its input, not query the DB — a trivial, stateless command. |
| `importTemplate` | port (D9 override, confirmed correct) | `templates_import_template` | `TemplateExport` | `PdfTemplate` | `print_templates` (insert) | — | not undoable | **Fixed this review** — see §8: was throwing a plain `Error`, not `ApiError`. Validates `json.schema === 'pdf-template-v1'` and that `json.template` is present; always assigns a fresh server-side id and `isDefault: false` regardless of what's in the imported JSON (can't import something that silently becomes the default). |
| `createTemplate` | port (D9 override, confirmed correct) | `templates_create_template` | `{ kind: DocumentKind, baseTemplateId: BaseTemplateId, name: string }` | `PdfTemplate` | `print_templates` (insert) | — | not undoable | The "duplicate/new" designer top-bar action — always starts from `defaultTemplateOptions()`, never `isDefault`. |

## 2. DTOs → Rust

| Type | Field | Rust type | Column | Why |
|---|---|---|---|---|
| `PdfTemplate` | `id` | `Uuid` | `UUID` | UUIDv7 (D2) — **must be generated server-side** in every create path (`saveTemplate` for a new row, `duplicateTemplate`, `importTemplate`, `createTemplate`), never trusted from the client even though `saveTemplate`'s request DTO happens to carry an `id` field already (see §1's note on that function's unusual create-vs-update-by-client-supplied-id shape). |
| `PdfTemplate` | `kind` | `enum DocumentKind { Invoice, Quotation, CreditNote, DebitNote, PurchaseOrder, Voucher, Statement, ZReport, TransferNote, Report }` | `ENUM(...)` | `#[serde(rename_all = "camelCase")]` — 10 variants, matches the TS union exactly. |
| `PdfTemplate` | `baseTemplateId` | `enum BaseTemplateId { InvoiceStandard, InvoiceSimplified, Quotation, CreditNote, DebitNote, PurchaseOrder, Voucher, Statement, ZReport, TransferNote, GenericReport, LabelSheet, LabelThermal }` | `ENUM(...)` | `#[serde(rename_all = "snake_case")]` — 13 variants (a superset of `DocumentKind`'s 10, since label-sheet/label-thermal/generic-report don't have their own `DocumentKind` — they're `report`-kind templates with a specific base layout). |
| `PdfTemplate` | `options` | `TemplateOptions` (nested struct, below) | JSON column (the whole design-options tree, not normalized into columns — this is a design-tool blob, not transactional data with its own referential integrity needs) | Recommend one JSON column, matching how `settings.md` treats `StoreSettings`'s nested option blocks (`printer`, `pos`, `accounting`, etc.) — normalizing every nested toggle into its own column would be substantial schema churn for zero query benefit (nothing ever filters/sorts by an individual option field). |
| `PdfTemplate` | `customSource` | `Option<String>` | `LONGTEXT`/`TEXT` nullable | Presumably a custom Typst/template-source override (not deeply reviewed here — the designer's "custom code" escape hatch; out of scope to reverse-engineer its exact grammar for this contract review, just note the column needs to hold arbitrary-length text). |
| `PdfTemplate` | `createdAt` / `updatedAt` | `DateTime<Utc>` (both) | `DATETIME(3)` | Instants — a template's design-metadata timestamps, not a business-local accounting date. |
| `TemplateOptions.header.*` / `.totals.*` / `.footer.*` / `.qr.*` | (all boolean/string leaf fields) | Nested struct fields, `bool`/`String` | (part of the JSON blob) | No decimal/uuid/date/route hints apply anywhere in this nested tree — pure design toggles and copy strings. |
| `TemplateOptions.columns` | `Vec<TemplateColumn>` | (part of the JSON blob, or a normalized child table if column order/visibility ever needs its own query — **not needed today**, no code reads columns outside the owning template) | — | `TemplateColumn.key` → `enum ColumnKey { Index, Sku, Barcode, Name, Unit, Qty, Price, Discount, Net, VatRate, Vat, Total }`, `#[serde(rename_all = "camelCase")]`. |
| `TemplateOptions.fontFamily` (also `LabelOptions.fontFamily`) | `enum FontFamilyOption { Cairo, NotoNaskhArabic, IbmPlexSansArabic, Tajawal }` | — (part of the JSON blob) | `#[serde(rename = "...")]` per variant, since the TS literals have spaces (`'Noto Naskh Arabic'`) that aren't valid Rust identifiers as-is. |
| `TemplateOptions.paper` | `enum PaperSize { A4, A5, Letter, Mm80, Mm58 }` | — | `#[serde(rename = "80mm")]`/`#[serde(rename = "58mm")]` for the two numeric-prefixed variants (not valid bare identifiers). |
| `LabelOptions` / `LabelPreset` | (a **separate, sibling** concept to `PdfTemplate` — label-sheet/thermal print layouts for product labels, not a document template) | Same JSON-blob treatment | — | **Confirmed out of this module's 11-function surface**: `LabelOptions`/`LabelPreset` are exported types but **no service function in `templateService.ts` reads or writes them** — they're consumed elsewhere (likely `core.md`'s `buildLabelItems`/`renderLabels*` functions, already reviewed there). Listed here only because they live in this module's `types/index.ts` file; not this module's endpoints to design a table for. |
| `TemplateExport` | `schema` | `enum TemplateExportSchema { PdfTemplateV1 }` (a 1-variant enum, or just a `const` string check) | — (request/response only, never persisted) | `#[serde(rename = "pdf-template-v1")]`. A future schema version would add a variant here, not break this one — the mock's own `importTemplate` already checks this discriminant defensively. |

## 3. Validation and errors

| Function | Rule (source: mock check line) | Code | Exact Arabic message |
|---|---|---|---|
| `importTemplate` | `json.schema` must equal `'pdf-template-v1'` and `json.template` must be present | `VALIDATION` — **fixed this review**, was a plain `Error` with no code (see §8) | `ملف القالب غير صالح` |

Every other function in this module has **no validation at all** — not a gap, a deliberate design-tool permissiveness (a template's `name`/`options` are free-form design choices, there's no "invalid" state to police server-side beyond the one structural check on import). `getTemplate`/`getDefaultTemplate`/`setAsDefault`/`duplicateTemplate`/`deleteTemplate` all treat a missing id as a valid, error-free outcome (see §1) rather than a validation failure.

## 4. Undo matrix (every function that writes)

| Function | Undoable? | Compensation (existing fn) | Refused when | Period rule (D7) |
|---|---|---|---|---|
| `saveTemplate` / `duplicateTemplate` / `importTemplate` / `createTemplate` | No | — | n/a | n/a — design metadata, no accounting consequence |
| `setAsDefault` | No (but trivially re-callable — calling it again on a different id simply changes the default again; there's no state that becomes permanently "stuck") | — | n/a | n/a |
| `resetTemplateToDefaults` | No — this **is** effectively its own "undo" primitive for a template's design changes, but it can't be un-done itself (resetting, then wanting the pre-reset design back, has no path other than re-designing or importing a previously-exported copy) | none (partial: `importTemplate` of a prior `exportTemplate` snapshot, if the user thought to export first) | n/a | n/a |
| `deleteTemplate` | No (hard delete, no reference check today — see §8) | — | n/a | n/a |

No function in this module touches the ledger/stock/numbering/period shared managers — matches `approvals.md`/`analytics.md`'s pattern of "master-plan §3 rule 7's undo registry doesn't apply here at all," since nothing here is an accounting action.

## 5. Concurrency under D8 (several terminals on one DB)

| Race | Rows | Settled by |
|---|---|---|
| Two terminals call `setAsDefault` for two different templates of the same `kind` at once | `print_templates.isDefault` (every row of that `kind`) | Needs a row lock across all templates of the target `kind` for the duration of the "clear everyone else, set this one" sequence (`SELECT ... FOR UPDATE` on the kind-scoped set) — otherwise two concurrent calls could both "win," leaving two rows `isDefault: true` for the same kind, or (if they interleave the other way) zero. This is the module's one real concurrency risk, structurally identical to `settings.md`'s "only one default tax per type" race already flagged there. |
| Two terminals edit the same template's `options` concurrently | `print_templates` row | Ordinary last-write-wins is acceptable here — same class of risk as any other master-data/design-tool edit (no financial consequence), consistent with how `parties.md`/`settings.md` treated similar non-critical concurrent edits. |
| A terminal deletes a template another terminal has open in the designer | `print_templates` row | No special handling needed at the DB layer (the delete just succeeds); the **UI** experience of "your open template just vanished" is a frontend concern for whoever builds the Rust-backed designer page, not a backend contract issue. |

Since this module is **shared per branch** (D9), every terminal in a branch reads/writes the same `print_templates` rows — unlike `settings.md`'s device-vs-branch split, there is no "device-local" variant of a print template at all; the whole point of D9 is that every cashier prints the same formats.

## 6. Events and side effects

- **No activity/audit rows anywhere in this module** — confirmed: `templateService.ts` never imports or calls `logActivity`/`logAudit`. This is a genuine, module-wide gap once templates become shared, multi-terminal DB data rather than a single device's `localStorage` — under D9, one cashier's terminal silently changing what every other terminal prints is exactly the kind of change that should leave a trail. **Flagged in §8/§9** as the module's most significant open item, more consequential than the missing-reference-check on delete, since it affects every write function, not just one.
- **No events emitted** (`ledger:changed`/`catalog:changed`/`parties:changed`) — correctly so, since nothing here is ledger/catalog/party data. Under D9's multi-terminal reality, though, some cross-terminal refresh signal IS needed (if terminal A changes the default invoice template, terminal B's next print should use it) — this belongs to `01-FRONTEND-ANALYSIS.md`'s 01.D cross-cutting doc (the "three event names" + change-version-table mechanism already scoped there for D8), not a new event this module invents on its own. Flagged for that doc, not fixed here.
- **Attachments/printing**: this module doesn't itself print anything — it's the template *designer*, consumed by `core.md`'s `render`/`renderPreview`/`printReceipt` functions (already reviewed there) which read a `PdfTemplate` by id/kind to know how to lay out a document.

## 7. Aggregations (reports / analytics / dashboard / insights only)

Not applicable — no reporting/aggregation output in this module.

## 8. Contract fixes needed in the mock (01.C — resolved 2026-09-27)

- [x] **`importTemplate` now throws `ApiError('ملف القالب غير صالح', 'VALIDATION')`** instead of a plain `Error` (`src/modules/templates/services/templateService.ts`) — closes the F5 error-code gap, the same class of fix applied in `settings.md`/`setup.md`/`payments.md`. `verify:mocks` confirmed **128 ok, 0 failed** both before and after (this module has zero accounting reach, so the fix carried no risk, but the baseline was checked per this session's standing practice regardless).
- [ ] **`deleteTemplate` has no reference/state check** — nothing stops deleting the current default template for its `DocumentKind` (leaving that kind with **zero** templates, which would make `getDefaultTemplate` fall through to `all[0]` on an empty array and return `undefined` — every caller of `getDefaultTemplate` needs to already handle that `undefined` case, since it's a documented possible return, so this isn't a crash risk, just a surprising UX where invoices stop knowing which template to use). Also doesn't check whether the template is currently selected as `settings.printer.a4Template`/`imageTemplate` (plan 22's fields, out of scope to touch, but worth naming as a *future* consumer that would care). **Not fixed** — this needs a product decision (refuse deleting the last template of a kind? refuse deleting the current default? both?) rather than a guessed-at rule, since the mock today has genuinely never had this guard and nothing currently exercises the gap in a way `verify:mocks` would catch. Recorded as a genuine open question in §9.
- [ ] **No audit trail at all** across every write function (see §6) — **not fixed**, this is a module-wide policy decision (add `logActivity('settings', ...)`-style calls to all 6 write functions?) rather than a single narrow bug, matching how `products.md` handled its own "catalog master-data writes have zero audit trail across ten functions" finding: flagged as a batch decision, not patched function-by-function without a stated scope. Recorded in §9.
- [ ] **`listTemplates`'s auto-seed-on-empty behavior** (see §1) needs to become a one-time DB bootstrap/migration step under D9, not a per-call fallback inside a read command — **not a mock fix**, a Part 02/03 implementation note for whoever designs the `print_templates` table and its initial-seed migration (likely reusing the same `seedEmptyCompany()`-adjacent bootstrap path `setup.md`/`users.md` already established for the fresh-install case).
- No F7 path-string-link findings — this module has no `link`/route field on `PdfTemplate`/`TemplateExport` at all.

## 9. Open questions (→ decisions in `00-MASTER-PLAN.md`)

- **Should `deleteTemplate` refuse deleting the last template of a `DocumentKind`, or the current default, or both?** No accounting risk either way — this is a UX/data-integrity design choice for the print-template designer, not a financial-safety question, but it's genuinely undecided rather than obviously "the strict option," so it's listed here rather than auto-picked. **Recommendation if a default must be chosen**: refuse deleting the last template of a kind (mirrors `settings.md`'s "can't delete the default tax" pattern), but allow deleting a non-default one freely even if it happens to be the only *other* one — i.e. the floor is "every kind always has at least one template," not "every kind always has at least two." This is a recommendation, not an applied fix, since it changes user-visible delete behavior.
- **Should every write function in this module get an audit trail**, now that templates are shared per-branch DB data (D9) rather than one device's local preference? **Recommendation**: yes, at minimum for `saveTemplate`/`deleteTemplate`/`setAsDefault` (the three that change what every terminal in the branch prints) — `duplicateTemplate`/`createTemplate`/`importTemplate`/`resetTemplateToDefaults` are lower-stakes (they don't change any *other* template's behavior) but could reasonably get one too for completeness. Left as a recommendation rather than applied, since it's a batch policy call across the whole module, not a single-line fix — matching `products.md`'s and this same session's precedent for "policy decisions on audit-trail scope go to §9, narrow single-function gaps get fixed directly."
- **Cross-terminal refresh signal for template changes** (see §6) — belongs to the not-yet-written `01-frontend-analysis/cross-cutting.md` (01.D), not to this file. Noted here so whoever writes that doc picks it up.

## Gate

- [x] Every inventory function is in §1 with a confirmed disposition (11/11, all `port` under the pre-existing D9 override — confirmed correct, no change needed to `scripts/contract/config.ts`).
- [x] Every write function is in §4.
- [x] Every DTO field needing a non-default mapping is in §2.
- [x] `bun run contract:check` — no override change, contract unchanged (11/11 port, D9 override already in place before this review). Also green: `bun run build`, `bun run check`, `bun run verify:mocks` (128 ok, 0 failed, unchanged before/after the one fix), `bun run memory:check` (0 new seam violations — confirmed the new `ApiError` import from `@/mocks` does not trip the seam guard), `bun run diag:check`.

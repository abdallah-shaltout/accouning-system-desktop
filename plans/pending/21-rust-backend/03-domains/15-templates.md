# 21 · 03.15 — `templates` (Typst print-template designer store, D9)

> **Status (2026-09-28): code complete, not yet compiled/tested.** `domains/templates/{mod,commands,
> service,dto}.rs`, `tests/domain_templates.rs`, the 10 frontend switch lines, the async contract fix
> (T-9) and its 3 caller edits, and `templates/types/contract.check.ts` are all written. Needs from
> the manager: `pub mod templates;` + `templates::ipc_signatures()` + `templates::dto::PdfTemplate/
> DocumentKind/BaseTemplateId::export_all(cfg)` wired into `domains/mod.rs`, the 10 commands added to
> `lib.rs`'s `generate_handler!`, and the T-6 `scripts/contract/config.ts` override
> (`templates.exportTemplate → frontend`, "pure transform"). Test/gate items (cargo check, `bun run
> bindings`/`build`/`memory`, the deferred DB test pass) → ⏳ deferred time-boxed test pass. Depends
> on: Part 02 (`print_templates` entity + migration m0014, `settings.default_branch_id` P2-20,
> `with_tx`), 00-import (it imports the old `localStorage` `pdf_templates_v1` list, D10/D9), 13's G-8
> (binding export hook). Independent of plan 22 (see §7 "Plan 22"). No `01-settings`/`02-setup`
> `TestDb` seeding fixture existed yet when `tests/domain_templates.rs` was written, so it carries its
> own local `seed_branch_and_settings` helper (see that file's top comment) — replace with a shared
> one if/when `01-settings` lands one.

**Goal.** Move the 10 DB-touching `templateService` functions from per-device `localStorage` to the
branch MariaDB (`print_templates`, D9), so every terminal prints with the same templates, keeping the
mock's contract: missing ids are not errors, the two default invoice templates reappear whenever the
store is empty, and exactly one default per document kind.

**Read first.** [`01-frontend-analysis/templates.md`](../01-frontend-analysis/templates.md) (`AT§n`) ·
`src/modules/templates/services/templateService.ts` (`ts:<line>`, the mock) ·
`src/modules/templates/types/index.ts` (`tt:<line>`) · `src-tauri/src/entities/platform/print_templates.rs`
· `src-tauri/migration/src/m0014_templates.rs` (`default_key` generated unique, C-15) · callers:
`templates/pages/TemplateDesignerPage.vue:63-175`, `templates/pages/TemplateListPage.vue:19-28`,
`core/services/pdfService.ts:465-514` · [`../../22-invoice-templates/README.md`](../../22-invoice-templates/README.md).

## 1. Commands

| Mock fn (`ts:` line) | Disposition | Rust command | Args → Return | Area / Access | tx | events |
|---|---|---|---|---|---|---|
| `listTemplates` (69) | port | `templates_list_templates` | `{ kind?: DocumentKind }` → `Vec<PdfTemplate>` | session only (T-3) | `with_tx` (may seed, T-2) | `catalog` if seeded |
| `getTemplate` (74) | port | `templates_get_template` | `{ id: String }` → `Option<PdfTemplate>` | session only | `with_tx` | `catalog` if seeded |
| `getDefaultTemplate` (78) | port | `templates_get_default_template` | `{ kind: DocumentKind }` → `Option<PdfTemplate>` | session only | `with_tx` | `catalog` if seeded |
| `saveTemplate` (83) | port | `templates_save_template` | `{ template: PdfTemplate }` → `PdfTemplate` | Settings / Write | `with_tx` | `catalog` |
| `setAsDefault` (93) | port | `templates_set_as_default` | `{ id: String }` → `()` | Settings / Write | `with_tx` | `catalog` (when found) |
| `duplicateTemplate` (103) | port | `templates_duplicate_template` | `{ id: String }` → `Option<PdfTemplate>` | Settings / Write | `with_tx` | `catalog` (when found) |
| `deleteTemplate` (122) | port | `templates_delete_template` | `{ id: String }` → `()` | Settings / Write | `with_tx` | `catalog` (when found) |
| `resetTemplateToDefaults` (127) | port | `templates_reset_template_to_defaults` | `{ id: String }` → `Option<PdfTemplate>` | Settings / Write | `with_tx` | `catalog` (when found) |
| `exportTemplate` (138) | port → **stay-frontend** (T-6) | — | — | — | — | — |
| `importTemplate` (143) | port | `templates_import_template` | `{ json: TemplateExport }` → `PdfTemplate` | Settings / Write | `with_tx` | `catalog` |
| `createTemplate` (162) | port | `templates_create_template` | `{ kind, baseTemplateId, name }` → `PdfTemplate` | Settings / Write | `with_tx` | `catalog` |

Writes need `Settings / Write` because the designer lives under `/settings/templates`
(`settings/routes/index.ts:33-34`, `area: 'settings'`). Reads need only a session: every role prints
(POS receipts, transfer notes, statements) through `pdfService.resolveTemplate` (`pdfService.ts:465-467`).
Events: `catalog` per cross-cutting §5 (templates reuse `catalog:changed`; no fourth category).

## 2. DTOs (`domains/templates/dto.rs`, `#[ts(export_to = "templates/types/gen/")]`)

| Rust DTO | TS type | Notes |
|---|---|---|
| `DocumentKind` (10 variants, `rename_all = "camelCase"`) | `tt:13-23` | Separate from the entity enum (`print_templates.rs:13-34`); `From` both ways. |
| `BaseTemplateId` (13 variants, `rename_all = "snake_case"`) | `tt:134-147` | Stored as the text in `base_template_id VARCHAR(32)`. |
| `PdfTemplate` | `tt:149-160` | `id: Id` (`#[ts(type = "string")]`); `options: serde_json::Map<String, Value>` with `#[ts(type = "import('../index').TemplateOptions")]` (opaque, T-5); `customSource: Option<String>` serialized as `null`, **not** skipped (`string \| null`, `#[ts(type = "string \| null")]`, no `skip_serializing_none` on this struct); `isDefault: bool`; `createdAt`/`updatedAt: String` (`format_iso_ms`). |
| `TemplatesImportTemplateArgs { json: serde_json::Value }` | `TemplateExport` (`tt:162-165`) | `#[ts(type = "import('../index').TemplateExport")]`; validated by hand (§3.9) so a bad file gets the mock's Arabic message instead of a serde error. |
| `TemplatesCreateTemplateArgs { kind, baseTemplateId, name }` and the id/kind args | — | ids are `String` (T-4). |

`src/modules/templates/types/contract.check.ts` (new): `Equals` for `PdfTemplate`, `DocumentKind`,
`BaseTemplateId`; `PdfTemplate.options` gets `// contract-ok: opaque JSON typed as TemplateOptions`.
`LabelOptions`/`LabelPreset` are not DTOs (no service function returns them, AT§2).

## 3. Service logic (`domains/templates/service.rs`)

### 3.0 Helpers

- **`branch(conn)`** = `settings.default_branch_id` (`core::settings::load`, T-1). Every query is
  scoped `branch_id = branch AND deleted_at IS NULL` (live rows; the mock hard-deletes).
- **Order** of a list = `(created_at, id)` = the mock's array order (seeds first, then `push`es).
- **`find(conn, id: &str)`**: parse `id` as `Id`; unparsable or no live row in the branch → `None`
  (the mock's `find` → `undefined`, T-4).
- **`default_options() -> Map`**: a literal port of `defaultTemplateOptions()` (`tt:90-126`): `accentColor
  "#4f46e5"`, `logoPosition "start"`, `logoSize "m"`, `fontFamily "Cairo"`, `fontSize 10`, `header { showCompanyName
  true, showAddress true, showVatNumber true, showCommercialRegister true, showPhone true, showEmail false,
  showWebsite false, title "فاتورة ضريبية", titleEn "TAX INVOICE" }`, `columns` = the 9 `DEFAULT_COLUMNS`
  (`tt:36-46`, same order/labels/visibility), `totals { showAmountInWords true, showBalance false }`,
  `footer { terms "", bankDetails "", showSignatureLines false, thankYouLine "شكراً لتعاملكم معنا",
  showPageNumbers true }`, `qr { position "center", size "3cm" }`, `paper "a4"`. A unit test compares it
  with `JSON.stringify(defaultTemplateOptions())` captured into a fixture file.
- **`ensure_seeded(conn, cx, branch)`** (port of `load()`/`seedDefaults()`, `ts:21-67`; T-2): if the
  branch has no live template: `lock::for_update_by_id(conn, "branches", branch)`, count again (READ
  COMMITTED sees a concurrent seeder's commit), and if still 0 insert, in this order and with the same
  `now = cx.clock.now`: (1) `"الفاتورة الضريبية القياسية"`, kind `invoice`, base `invoice_standard`,
  `default_options()`, `custom_source NULL`, `is_default true`; (2) `"الفاتورة الضريبية المبسطة"`, kind
  `invoice`, base `invoice_simplified`, `default_options()` with `header.title = "فاتورة ضريبية مبسطة"` and
  `header.titleEn = "SIMPLIFIED TAX INVOICE"`, `is_default false`; then `cx.touch(Catalog)`. Every command
  calls it first, exactly as every mock function calls `load()`.
- **`to_dto(model)`**: `PdfTemplate` with `createdAt`/`updatedAt` = `format_iso_ms`.

### 3.1 `templates_list_templates` (`ts:69-72`)

1. `ensure_seeded`. 2. Live rows of the branch, filtered by `kind` when given, in order → DTOs.

### 3.2 `templates_get_template` (`ts:74-76`)

1. `ensure_seeded`. 2. `find(id)` → `Some(dto)` or `None` (never `NOT_FOUND`, AT§1).

### 3.3 `templates_get_default_template` (`ts:78-81`)

1. `ensure_seeded`. 2. Live rows of `kind` in order; the first with `is_default`, else the first row,
   else `None` (AT§1).

### 3.4 `templates_save_template` (`ts:83-91`)

1. `cx.require(Settings, Write)`. `ensure_seeded`.
2. `find(template.id)`; `None` → `NOT_FOUND "القالب غير موجود"` (T-7: a save never inserts a row with a
   client-chosen id; the designer only saves templates it loaded — creation goes through 3.6/3.9/3.10).
   Lock that row `FOR UPDATE`.
3. Update only the design fields: `name`, `base_template_id`, `options`, `custom_source`, and
   `updated_at = cx.clock.now`. `kind`, `is_default` and `created_at` keep their stored values (T-7:
   the default flag changes only through 3.5, so the one-default rule can't be broken by a stale client copy).
4. `cx.touch(Catalog)`; return the stored row as DTO.

### 3.5 `templates_set_as_default` (`ts:93-101`)

1. `require(Settings, Write)`; `ensure_seeded`; `find(id)`; `None` → return `()` (silent no-op, AT§1).
2. Lock every live row of `(branch, target.kind)` `FOR UPDATE`, sorted by id (AT§5: the race between two
   terminals setting two defaults).
3. Two statements, in this order (a single `SET is_default = (id = ?)` could briefly hold two defaults and
   trip `uq_print_templates_default_key` mid-statement): `UPDATE … SET is_default = FALSE, updated_at = now
   WHERE branch_id = ? AND kind = ? AND is_default AND deleted_at IS NULL AND id <> ?`, then `UPDATE … SET
   is_default = TRUE, updated_at = now WHERE id = ?`. (The mock does not touch `updatedAt` here; see Q-2.)
4. `cx.touch(Catalog)`.

### 3.6 `templates_duplicate_template` (`ts:103-120`)

1. `require`; `ensure_seeded`; `find(id)`; `None` → `None`.
2. Insert a new row: new `Id`, `name = "<source name> (نسخة)"`, same `kind`, `base_template_id`,
   `options` (a copy), `custom_source`, `is_default false`, `created_at = updated_at = now`.
3. `touch(Catalog)`; return it.

### 3.7 `templates_delete_template` (`ts:122-125`)

1. `require`; `ensure_seeded`; `find(id)`; `None` → `()` (no-op).
2. Soft delete (`deleted_at = now`, P2-16); `default_key` becomes `NULL` by its generated definition. No
   reference check (open question U-1). If it was the last live row, the next call re-seeds the two
   defaults — the mock's behaviour when its array becomes empty (`ts:26`).
3. `touch(Catalog)`.

### 3.8 `templates_reset_template_to_defaults` (`ts:127-136`)

1. `require`; `ensure_seeded`; `find(id)`; `None` → `None`.
2. `options = default_options()`, `custom_source = NULL`, `updated_at = now`; `name`, `kind`, base and
   `is_default` unchanged (AT§1).
3. `touch(Catalog)`; return it.

### 3.9 `templates_import_template` (`ts:143-159`)

1. `require(Settings, Write)`; `ensure_seeded`.
2. Validate (T-8): `json.schema == "pdf-template-v1"`; `json.template` is an object; `template.name` is a
   string; `template.kind` is one of the 10 `DocumentKind` values; `template.baseTemplateId` one of the 13;
   `template.options` an object; `template.customSource` absent, `null` or a string. Any failure →
   `VALIDATION "ملف القالب غير صالح"` (the mock's message and code, AT§3).
3. Insert: new `Id`, those fields, `is_default false`, `created_at = updated_at = now` (unknown extra keys
   in `template` are not stored).
4. `touch(Catalog)`; return it.

### 3.10 `templates_create_template` (`ts:162-179`)

1. `require(Settings, Write)`; `ensure_seeded`.
2. Insert `{ new Id, name, kind, base_template_id, default_options(), custom_source NULL, is_default false,
   now, now }`. No validation of `name` (the mock has none, AT§3).
3. `touch(Catalog)`; return it.

## 4. Concurrency (D8, AT§5)

- **Two terminals set two defaults of one kind at once:** 3.5 locks the kind's rows `FOR UPDATE`, so the
  second waits and then re-applies; the generated `default_key` unique (`m0014_templates.rs:41-45`) is the
  backstop. A duplicate-key error there maps to `CONFLICT "قيد التعديل من جهاز آخر"` via `AppError::map_unique`
  on `uq_print_templates_default_key` (P2-22).
- **Two terminals seed an empty branch at once:** the branch row lock + re-count in `ensure_seeded`
  inserts the defaults once.
- **Concurrent edits of one template:** last write wins (AT§5); the `FOR UPDATE` in 3.4 only serialises them.
- Lock order inside this domain: `branches` row (seed only) → `print_templates` rows (sorted) →
  `change_versions` (by `with_tx` at commit).

## 5. Undo

Not undoable via the registry (entry §3.5; AT§4): design metadata with no accounting effect.
`resetTemplateToDefaults` and an earlier export are the user's own "undo". No audit/activity row is
written (the mock writes none; open question U-2).

## 6. Frontend switch lines, and the one contract fix

**Contract fix T-9 (mock too, before the switch lines).** All ten DB-touching functions are synchronous
today (`ts:69-179`). A synchronous function cannot await IPC, and a fake-synchronous write would report
"saved" while the save could still fail (zero data loss). They become `async` (`Promise<…>`) in the mock,
and their callers `await` them — a bounded exception to master §5's "pages never change", limited to:
- `TemplateListPage.vue:19,26`: `const templates = ref<PdfTemplate[]>([])` + `onMounted(async () => {
  templates.value = await listTemplates('invoice') })`; `newTemplate` becomes `async` and awaits `createTemplate`.
- `TemplateDesignerPage.vue`: `load()` becomes `async` (awaits `getTemplate`/`listTemplates`; call sites
  `void load()` and `watch(templateId, load)`); `persist`, `markDefault`, `duplicate` (awaits `persist()`
  first), `resetToDefaults` and `onImportFile` await their service calls; toasts move after the await.
  `exportJson` is unchanged (`exportTemplate` stays synchronous, T-6).
- `pdfService.ts`: `resolveTemplate` becomes `async`; `render` (`:514`) does `const template = await resolveTemplate(kind, templateId);`.

Then, in `src/modules/templates/services/templateService.ts`, inside each `wrap(...)`
(`usesRust('templates')`):

| Function | Switch line |
|---|---|
| `listTemplates` | `if (usesRust('templates')) return backendCall('templates_list_templates', { kind });` |
| `getTemplate` | `… return (await backendCall('templates_get_template', { id })) ?? undefined;` |
| `getDefaultTemplate` | `… return (await backendCall('templates_get_default_template', { kind })) ?? undefined;` |
| `saveTemplate` | `… return backendCall('templates_save_template', { template });` |
| `setAsDefault` | `… { await backendCall('templates_set_as_default', { id }); return; }` |
| `duplicateTemplate` | `… return (await backendCall('templates_duplicate_template', { id })) ?? undefined;` |
| `deleteTemplate` | `… { await backendCall('templates_delete_template', { id }); return; }` |
| `resetTemplateToDefaults` | `… return (await backendCall('templates_reset_template_to_defaults', { id })) ?? undefined;` |
| `importTemplate` | `… return backendCall('templates_import_template', { json });` |
| `createTemplate` | `… return backendCall('templates_create_template', { kind, baseTemplateId, name });` |

`Option` returns arrive as `null`; `?? undefined` keeps the TS `PdfTemplate | undefined` contract.

## 7. Known mock quirks (kept), decisions, open questions, plan 22

**Quirks:**
- **Q-1** Deleting every template silently brings back the two defaults (`ts:26`).
- **Q-2** `setAsDefault` doesn't change `updatedAt` in the mock; Rust sets `updated_at` on the rows it
  flips (the column is `ON UPDATE CURRENT_TIMESTAMP(3)` anyway, `m0014_templates.rs:31`). Only
  `updatedAt` of those rows can differ; the parity case ignores `updatedAt` on `setAsDefault` rows.
- **Q-3** `deleteTemplate` has no caller today (no page imports it); ported for completeness (AT§1).

**Decisions (strictest option, logged):**
- **T-1** One template set per database, keyed to `settings.default_branch_id`. The mock has one set per
  device regardless of branch; D1 puts one DB per branch, so "shared per branch" (D9) = shared per DB.
  Per-branch sets inside one multi-branch DB (C-17) are "later".
- **T-2** Seeding on empty stays in every command (not a one-time bootstrap as AT§8 suggested), because the
  mock also re-seeds after all templates are deleted (Q-1); a bootstrap would change that behaviour. So all
  commands are `with_tx`.
- **T-3** Reads require a session but no area: every role prints.
- **T-4** Id args are `String`; an unparsable id (e.g. the old `tpl_…` ids or `pdfService`'s `'fallback'`)
  behaves as "not found", matching the mock's `find`, instead of a deserialisation error.
- **T-5** `options` is stored and returned as opaque JSON (P2-18; the type's own note says unknown/future
  keys must survive, `tt:1-6`): a typed struct would silently drop keys on save.
- **T-6** `exportTemplate` stays in TypeScript: it is a pure projection of an object the page already
  holds (`ts:138-141`, AT§1 "never reads `print_templates`"); an IPC round trip would only force a sync→async
  change on its caller for no gain. Needs a disposition override in `scripts/contract/config.ts`
  (`templates.exportTemplate → frontend`, reason "pure transform") — a manager task, registry G-40.
- **T-7** `saveTemplate` updates only design fields of an **existing** row; unknown id → `NOT_FOUND
  "القالب غير موجود"` (same text the designer shows, `TemplateDesignerPage.vue:72`); `kind`/`isDefault`/
  `createdAt` from the client are ignored (server authority; AT§2 "never trust a client-supplied id for a new row").
- **T-8** `importTemplate` validates every field it stores, with the mock's single message.
- **T-9** The ten functions become async (above).

**Open questions for the user:**
- **U-1** Should deleting the current default (or the last template of a kind) be refused? AT§9
  recommends refusing the last one. Kept as the mock (no guard) until decided; no page deletes today (Q-3).
- **U-2** Should template writes leave an audit trail now that they're shared? `shared::activity::record`
  always writes an `activity` row too (`record.rs:130-134`), which would show template edits in the home
  activity feed — a visible change, so not auto-decided. Recommendation: yes, for save/delete/setAsDefault.

**Plan 22 (`plans/pending/22-invoice-templates/`).** Plan 22's A4/image layouts are **code-defined Vue
components** chosen by `StoreSettings.printer.a4Template`/`imageTemplate` (its decisions 2, 5, 9), which
cross-cutting §3 classifies as **branch** fields — they belong to 01-settings, not to `print_templates`, and
they hold registry keys (`standard`, `instapay`, …), not `PdfTemplate` ids, so `deleteTemplate` never
breaks them. This file does not touch plan 22's files. If plan 22 later stores user-made A4/image layouts
in `print_templates`, the Rust side follows by: new `DocumentKind`/`BaseTemplateId` variants in the DTO
enums **and** a new migration (`m0016+`) extending the `kind` `ENUM` (never an edit to m0014); `options`
needs no migration (opaque JSON).

## 8. Tests

**(a) `src-tauri/tests/domain_templates.rs`:**
- first `list` on an empty DB seeds exactly the two defaults (names, bases, `is_default`, options equal to
  the fixture), touches `catalog`; a second `list` inserts nothing;
- two concurrent first calls (two connections) seed once;
- `get` with unknown, soft-deleted and unparsable ids → `None`;
- `getDefault` falls back to the first row when no default is flagged;
- `save`: design fields updated, `kind`/`isDefault` ignored, unknown id → `NOT_FOUND "القالب غير موجود"`;
- `setAsDefault`: exactly one default after, two concurrent calls leave one default; unknown id → no-op;
- `duplicate`: `(نسخة)` suffix, not default, options copied; unknown → `None`;
- `delete`: soft delete; deleting all → next call re-seeds (Q-1);
- `reset`: options/customSource reset, name kept;
- `import`: each invalid shape → `VALIDATION "ملف القالب غير صالح"`; valid → new id, not default;
- `create`: default options;
- writes without `settings: write` (cashier) → `FORBIDDEN`; reads with a cashier session succeed;
  no session → `UNAUTHORIZED`;
- unknown option keys survive a save round trip (T-5).

**(b) Parity cases for Part 04:** start empty → list (seed) → create → save (options edit) → set default
→ duplicate → reset → import a valid and an invalid file → delete → list, on mock vs Rust; compare DTOs
with ids mapped and `createdAt`/`updatedAt` compared as "present, ISO-ms" only (clock differs by design).

## 9. Checklist

- [x] Contract fix T-9: make the ten functions `async` in `templateService.ts` and update the three callers (§6); `bun run build` green on the mock alone. ⏳ deferred time-boxed test pass (build not run this session).
- [x] `domains/templates/{mod,commands,service,dto}.rs`: DTOs + `export_bindings`, helpers (§3.0), 10 service fns, 10 commands, `ipc_signatures()`. Written; `export_bindings`/`generate_handler!`/`pub mod` wiring is manager-owned (§3.7) — see status note above for the exact lines needed.
- [x] `default_options()` fixture test (capture `JSON.stringify(defaultTemplateOptions())` into `src-tauri/tests/fixtures/default_template_options.json`). Fixture written by hand from `types/index.ts:90-126` (no live `bun`/node run this session) plus `default_options_matches_the_mock_fixture` in `tests/domain_templates.rs`.
- [x] Report the 10 command names to the manager; request the T-6 contract override. See this file's final report / status note above.
- [x] 10 switch lines (§6).
- [x] `src/modules/templates/types/contract.check.ts`.
- [x] `tests/domain_templates.rs` (§8a); parity sequence (§8b) into the Part 04 list. ⏳ deferred time-boxed test pass — uses a local `seed_branch_and_settings` fixture (no shared `01-settings` `TestDb` helper existed yet); the concurrency test drives two real overlapping transactions directly (`ensure_seeded`'s lock/re-count/insert body inlined) rather than through `with_tx`, since `TxCtx` has no public constructor outside it.
- [ ] Confirm with 00-import that `pdf_templates_v1` rows land with `branch_id = default_branch_id`, in array order, ids remapped, `isDefault` kept. Not confirmed — `00-import`'s importer file is owned by a different W1 implementer; flagged for the manager to cross-check once both land.

## Gate

`cargo check` clean (manager's throttled run after W1); tests and parity case written (deferred run);
`bun run build` green with the async functions; switch lines present; `contract.check.ts` compiles;
`bun run memory:check`: 10 `templates_*` commands invoked + registered, 0 contract gaps.

⏳ All of the above run in the deferred, time-boxed test/build pass — not executed this session per the
no-cargo/no-bun hard rule for implementers.

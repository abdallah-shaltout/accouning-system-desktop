/**
 * Single source of configuration for the backend-contract inventory (`bun run contract`,
 * plans/pending/21-rust-backend Part 01). Change behaviour here, never inside a stage.
 */
import path from 'node:path';
import type { Disposition } from './types';

/** D9 (plan 21): print templates move from per-device localStorage to the DB, shared per branch. */
const d9 = { disposition: 'port', reason: 'D9 — templates move from localStorage to the DB, shared per branch' } as const;

/** 01.B diagnostics review (F9): the accounting-debugger tab (/dev/diagnostics -> Al-Muhasaba) and
 * repro recording are dev-build-only per CLAUDE.md's Diagnostics section — never a production command. */
const diagDevOnly = {
  disposition: 'dev-only',
  reason: '01.B diagnostics review — powers the /dev/diagnostics accounting-debugger tab and repro recording (18.F3-F4), dev-build-only per CLAUDE.md, never a production command',
} as const;

export const config = {
  /** Repo root — this file lives in scripts/contract/. */
  root: path.resolve(import.meta.dirname, '../..'),
  tsconfig: 'tsconfig.json',
  /** Generated output folder (repo-relative). Never hand-edit what lands here. */
  outDir: 'docs/backend/contract',
  jsonFile: 'contract.gen.json',

  modulesDir: 'src/modules',
  mocksDir: 'src/mocks',
  mockBackendDir: 'src/mocks/backend',
  mockDbFile: 'src/mocks/db.ts',
  /** The `MockDb` interface — its members are the table list. */
  mockDbInterface: 'MockDb',
  /** Service wrapper (diagnostics/services/defineService.ts). Every call to it is one endpoint. */
  wrapFn: 'wrap',

  /** Array methods that change the array they are called on. */
  mutatingMethods: ['push', 'splice', 'unshift', 'pop', 'shift', 'sort', 'reverse', 'fill', 'copyWithin'],
  /** Array methods whose result is a reference INTO the table (so writing through it writes the table). */
  referenceMethods: ['find', 'findLast', 'at'],
  /** A call to one of these marks every table referenced inside its callback as written. */
  writeScopes: ['mutate'],
  /** Identifiers that mean "this runs in the browser / webview, not the backend". */
  browserGlobals: ['indexedDB', 'localStorage', 'sessionStorage', 'document', 'navigator', 'fetch', 'FileReader', 'Blob', 'window'],

  /**
   * Shared-manager capabilities (master plan §3 rule 3): reaching one of these mock functions means
   * the Rust port must go through that `shared::` module. Keys become the "shared" column.
   */
  capabilities: {
    ledger: ['src/mocks/backend/core.ts#postJournal', 'src/mocks/backend/core.ts#postDraftJournal', 'src/mocks/backend/core.ts#draftJournal'],
    stock: ['src/mocks/backend/core.ts#applyStockChange', 'src/mocks/backend/inventory.ts#receiveBatch', 'src/mocks/backend/inventory.ts#consumeFefo'],
    activity: ['src/mocks/backend/core.ts#logAudit', 'src/mocks/backend/core.ts#logActivity'],
    numbering: ['src/mocks/db.ts#nextNumber'],
    currency: ['src/mocks/backend/currency.ts#toBase', 'src/mocks/backend/currency.ts#convertLinesToBase', 'src/mocks/backend/currency.ts#requireRate'],
    period: ['src/mocks/backend/core.ts#assertOpenPeriod'],
  } as Record<string, string[]>,

  /**
   * Hand decisions that override the heuristic (Part 01 reviewers add rows here, always with a
   * reason). Key = the `wrap()` source name.
   */
  overrides: {
    'templates.listTemplates': d9,
    'templates.getTemplate': d9,
    'templates.getDefaultTemplate': d9,
    'templates.saveTemplate': d9,
    'templates.setAsDefault': d9,
    'templates.duplicateTemplate': d9,
    'templates.deleteTemplate': d9,
    'templates.resetTemplateToDefaults': d9,
    'templates.exportTemplate': {
      disposition: 'frontend',
      reason:
        '21.03 15-templates T-6 (G-40): a pure projection of a `PdfTemplate` object the page already holds in memory — it never reads `print_templates`, so an IPC round trip would only force a needless sync→async change on its one caller (`TemplateDesignerPage.vue`) for no gain',
    },
    'templates.importTemplate': d9,
    'products.batchAlertTone': {
      disposition: 'frontend',
      reason:
        '21.03 06b (G-P10): a pure display rule over a batch the page already holds; pages call it synchronously, so an IPC round trip would force page edits for no gain',
    },
    'products.branchStockQty': {
      disposition: 'frontend',
      reason:
        '21.03 06b (G-P10): reads the per-branch stock the Rust product reads already return (cached by `rememberBranchStock`); pages call it synchronously',
    },
    'templates.createTemplate': d9,
    'settings.listHistory': {
      disposition: 'frontend',
      reason: '01.B settings review — lists local .zip files via plugin-fs (Tauri) or an IndexedDB store (browser); the folder path it reads comes from the already-ported getSettings/backupSettings, so no capability is lost by keeping this frontend-only',
    },
    'setup.persistProgress': {
      disposition: 'drop',
      reason: '01.B setup review — only calls flushSnapshot() (mock-specific IndexedDB persistence); under MariaDB every write already commits durably inside its own transaction, so there is no Rust equivalent',
    },
    'setup.previewCoaTemplate': {
      disposition: 'frontend',
      reason:
        "21.04 phase A (A-2 finding): the function's own doc comment already says \"D-6: stays frontend-only even on Rust — pure/synchronous, used in a computed (StepCoa.vue)\", and its mock body (`src/mocks/backend/setup.ts#previewCoaTemplate`) is itself documented \"without touching db\" — a pure tree-builder over its own arguments, not a backend read. The heuristic mis-suggested `port` only because it reaches a `src/mocks/backend/**` function at all, not because that function touches any table",
    },
    'setup.hasLegacySnapshot': {
      disposition: 'frontend',
      reason:
        "P4-9 (21.04 entry file): reads the browser's OWN legacy IndexedDB snapshot directly via `src/mocks/persist.ts#readPersistedSnapshot` — this is not a MockDb business table, it is the one-time legacy-import source data that stays in IndexedDB even after the Rust flip (the importer then hands it to `setup_import_snapshot`). The heuristic mis-suggested `port` because `readPersistedSnapshot` lives under `src/mocks/`, but it never reads `db.*`",
    },
    'parties.findDuplicates': {
      disposition: 'frontend',
      reason:
        '21.04 phase A (A-2 finding): a synchronous internal helper over `db.customers`/`db.suppliers` with no `usesRust` guard of its own — its only caller in `src/` is `parties.checkDuplicates` (same file), which already has its own gated `backendCall(\'parties_check_duplicates\', …)` switch line and only falls back to this sync helper on the mock path. No page or component calls `findDuplicates` directly (grep confirmed), so there is no seam to close — it is exported only because `wrap()` requires an export, not because it is a standalone Rust-backed entry point',
    },
    'core.getThresholds': {
      disposition: 'port',
      reason:
        "21.04 phase A (A-2 finding): its Rust branch reads `useSettingsStore().settings?.insightThresholds` instead of calling `backendCall` itself — the store's own `load()`/`update()` already go through the gated `settings.getSettings`/`settings.updateSettings` switch lines (settingsService.ts). This is the same pattern CLAUDE.md's router-guard/store-load rule describes for a synchronous reader (A-5): the switch line lives in the loader, not in every function that later reads the loaded state. Kept as `port` since it genuinely is Rust-backed once the store has loaded; allowlisted here so A-2 doesn't demand a second, redundant backendCall in this function's own body",
    },
    'core.setThresholds': {
      disposition: 'port',
      reason:
        '21.04 phase A (A-2 finding): same pattern as core.getThresholds — its Rust branch calls `useSettingsStore().update(...)`, which is `settings.updateSettings`\'s own already-gated switch line, not a second backendCall here',
    },
    'users.getDemoAccounts': {
      disposition: 'dev-only',
      reason: '01.B users review — returns plaintext passwords for the dev/demo login-screen account picker; must never exist as a general-purpose production command (same class of restriction as devToolsService, F9)',
    },
    'core.resetToEmpty': {
      disposition: 'dev-only',
      reason: '01.B core review — wipes all persisted data with no confirmation/audit trail beyond the caller reloading the app; a dev/demo reset capability, must never exist as a general-purpose production command (same class as users.getDemoAccounts, F9)',
    },
    'core.reloadDemoData': {
      disposition: 'dev-only',
      reason: '01.B core review — reseeds and overwrites nearly every table with fixed demo data; a dev/demo capability, must never exist as a general-purpose production command (same class as users.getDemoAccounts, F9)',
    },
    'diagnostics.explainAccountBalance': diagDevOnly,
    'diagnostics.getBalancesAround': diagDevOnly,
    'diagnostics.getDriftReport': diagDevOnly,
    'diagnostics.getInvariantResults': diagDevOnly,
    'diagnostics.getJournalEntryRaw': diagDevOnly,
    'diagnostics.getPostingTrace': diagDevOnly,
    'diagnostics.listRecentDocuments': diagDevOnly,
    'diagnostics.startReproRecording': diagDevOnly,
  } as Record<string, { disposition: Disposition; reason: string }>,

  /** Field-name patterns → DTO/column hints (only applied to fields whose type fits). */
  hints: {
    decimal: /(amount|total|price|cost|balance|tax|vat|discount|value|paid|outstanding|debit|credit|qty|quantity|rate|fee|variance|net|gross|cash|limit|budget|margin|profit|revenue|expense|sales|stock|opening|closing|due|change|tender|counted|expected|share|percent|pct)/i,
    date: /(date|At|On|Until|From|To|month|period)$/,
    route: /^(link|to|actionTo|sourceLink|refLink|href|route)$/,
  },

  /**
   * Repo-relative path patterns excluded from the type inventory (21.02-F, F-6): ts-rs-generated
   * bindings (`**\/types/gen/**`, already inventoried on the Rust side via their own DTOs) and the
   * drift-check files (`**\/contract.check.ts`, which exist only to be type-checked by `vue-tsc`,
   * never a type any service exposes).
   */
  excludeFromTypes: [/\/types\/gen\//, /\/contract\.check\.ts$/],

  /** Where `BackendDomain` (`core/services/backend.ts`) and `IpcCommands` (`core/types/gen/ipc.gen.ts`)
   * live — the A-2 switch-line coverage check reads both textually instead of hardcoding the lists here. */
  backendServiceFile: 'src/modules/core/services/backend.ts',
  ipcManifestFile: 'src/modules/core/types/gen/ipc.gen.ts',

  /**
   * A-2 (21.04 phase A): `backendCall('<cmd>', …)` sites that are allowed to have **no** enclosing
   * `usesRust('<domain>')` guard, each with a reason. Everything else a `port` function calls (or any
   * other wrapped service calls) must sit inside exactly one such guard, keyed by the domain the
   * command's own prefix names.
   */
  ungatedSwitchSites: {
    core_backend_status: 'F-2 — the real backend\'s own connection/role/schema status; no mock equivalent to switch away from, so it is a literal backendCall with no domain guard (core.getBackendStatus is `rust-existing`, not `port`)',
  } as Record<string, string>,

  /**
   * A-2 rule (b) exception: a `backendCall` command prefix that legitimately does not equal its
   * enclosing `usesRust('<domain>')` guard's domain, each with a reason. Rule (b)'s "`<d>` equals the
   * command's prefix" holds for every switch line except these documented ones.
   */
  switchDomainPrefixExceptions: {
    attachments: 'core',
  } as Record<string, string>,

  /**
   * A-4 (21.04 phase A): `stay-frontend` service functions allowed to still read mock `db.*` tables
   * after the flip — dev-only tooling that is refused or rerouted on Rust rather than ported, each
   * with a reason tied to the finding that justifies it.
   */
  stayFrontendMockReadAllowlist: {} as Record<string, string>,
};

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
};

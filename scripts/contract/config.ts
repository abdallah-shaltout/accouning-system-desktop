/**
 * Single source of configuration for the backend-contract inventory (`bun run contract`,
 * plans/pending/21-rust-backend Part 01). Change behaviour here, never inside a stage.
 */
import path from 'node:path';
import type { Disposition } from './types';

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
  overrides: {} as Record<string, { disposition: Disposition; reason: string }>,

  /** Field-name patterns → DTO/column hints (only applied to fields whose type fits). */
  hints: {
    decimal: /(amount|total|price|cost|balance|tax|vat|discount|value|paid|outstanding|debit|credit|qty|quantity|rate|fee|variance|net|gross|cash|limit|budget|margin|profit|revenue|expense|sales|stock|opening|closing|due|change|tender|counted|expected|share|percent|pct)/i,
    date: /(date|At|On|Until|From|To|month|period)$/,
    route: /^(link|to|actionTo|sourceLink|refLink|href|route)$/,
  },
};

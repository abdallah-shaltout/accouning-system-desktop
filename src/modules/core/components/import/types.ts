/**
 * v2 phase 5 (docs/v2/05-onboarding.md §5): the generic Excel-import descriptor pattern. One
 * `ImportWizard.vue` component drives every import type (customers, suppliers, products, opening
 * stock, opening balances, price updates) through a small per-type descriptor — `exceljs` stays a
 * single lazy-loaded dependency instead of being imported from a dozen pages.
 */

export interface ImportColumn<Row = any> {
  key: string;
  label: string;
  /** Header text (Arabic/English) this column auto-maps from, matched case/space-insensitively. */
  synonyms: string[];
  required?: boolean;
  /** Sample value shown in the downloaded template's example row. */
  sample?: string | number;
  /** A short note shown in the template's notes row, e.g. allowed values. */
  note?: string;
  /** Data-validation dropdown list for the template (Excel's in-cell list). */
  options?: string[];
  /** Parses a raw cell value into the row's typed field. Defaults to the raw string. */
  parse?: (raw: unknown) => unknown;
  /** Formats a parsed value for the validation table (e.g. 'debit' -> 'مدين'). Defaults to the raw value. */
  display?: (value: unknown) => string;
  /** Returns an error message if the parsed value is invalid, or undefined if OK. */
  validate?: (value: unknown, row: Partial<Row>) => string | undefined;
}

export interface ImportRowResult<Row = any> {
  index: number;
  raw: Record<string, unknown>;
  row: Partial<Row>;
  errors: Record<string, string>;
  warnings: Record<string, string>;
  /** Set once matched against an existing record (by the descriptor's `dedupe` key). */
  duplicateOf?: string;
  duplicateAction?: 'update' | 'skip' | 'create';
  status: 'ok' | 'warning' | 'error';
}

export interface ImportCommitSummary {
  created: number;
  updated: number;
  skipped: number;
  failed: { index: number; reason: string }[];
}

export interface ImportDescriptor<Row = any> {
  key: string;
  title: string;
  /** Shown on the template-download button and the upload dropzone hint. */
  entityLabel: string;
  columns: ImportColumn<Row>[];
  /** Field (already-parsed row key) used to detect a duplicate against existing records — e.g. 'sku' | 'phone'. */
  dedupeKey?: string;
  /** Looks up an existing record id by the dedupe value; undefined = no match (a new record). */
  findExisting?: (dedupeValue: unknown) => Promise<{ id: string; label: string } | undefined>;
  /** Runs the actual create/update calls in batches. Returns per-row outcome. */
  commit: (rows: ImportRowResult<Row>[], onProgress?: (done: number, total: number) => void) => Promise<ImportCommitSummary & { createdRows?: any[] }>;
  /** Sample rows shown in the downloaded template. */
  sampleRows?: Record<string, string | number>[];
}

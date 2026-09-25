/**
 * The official printed-report model — the equivalent of the Handlebars "locals" the reference
 * accounting system hands to its `templates/*.hbs` report files. A report page describes WHAT to
 * print (already-formatted strings, grouped into blocks); `renderHtml.ts` (print + preview) and
 * `src-tauri/templates/report.typ` (native PDF) only lay it out. Printing never reuses the on-screen
 * component: every print/PDF is a fresh render of this model.
 */

export type PrintAlign = 'start' | 'center' | 'end';

export interface PrintColumn {
  label: string;
  /** Default: `end` (LTR-isolated, decimal-aligned) when `numeric`, otherwise `start`. */
  align?: PrintAlign;
  /** Numbers/amounts: tabular digits, isolated LTR so minus signs and separators never flip. */
  numeric?: boolean;
  /** Relative width (like CSS `fr`). Omit for auto. */
  width?: number;
  /** Smaller, muted text (codes, dates, row numbers). */
  dim?: boolean;
}

/**
 * - `section`  full-width dark band (e.g. "الأصول")
 * - `subhead`  full-width light band (e.g. "الأصول المتداولة")
 * - `opening`  italic muted row (e.g. "رصيد أول المدة")
 * - `subtotal` bold row with rules above/below
 * - `total`    closing total of a table
 * - `grand`    the statement's bottom line (e.g. "صافي الربح")
 */
export type PrintRowKind = 'normal' | 'section' | 'subhead' | 'opening' | 'subtotal' | 'total' | 'grand';

export interface PrintRow {
  cells: string[];
  kind?: PrintRowKind;
}

export interface PrintKpi {
  label: string;
  value: string;
  /** Highlighted cell (the one number the reader should see first). */
  emphasis?: boolean;
}

export type PrintBlock =
  | { type: 'kpis'; items: PrintKpi[] }
  | { type: 'heading'; text: string }
  | { type: 'table'; columns: PrintColumn[]; rows: PrintRow[]; emptyText?: string; hideHeader?: boolean }
  /** Side-by-side stacks, e.g. assets | liabilities + equity. */
  | { type: 'columns'; columns: PrintBlock[][] }
  /** Account/party banner above a ledger. */
  | { type: 'banner'; title: string; subtitle?: string; tag?: string }
  | { type: 'note'; text: string; tone?: 'ok' | 'warn' | 'info' }
  /** Large-number boxes (VAT output / input / net). */
  | { type: 'boxes'; items: { title: string; value: string; sub?: string; emphasis?: boolean }[] };

export interface PrintCompany {
  name: string;
  address?: string;
  phone?: string;
  vatNumber?: string;
  commercialRegister?: string;
  /** data: URL */
  logo?: string;
}

export interface ReportDocument {
  title: string;
  subtitle?: string;
  /** Top-left tag, e.g. "تقرير رسمي". */
  badge: string;
  company: PrintCompany;
  /** The strip under the title: period, issue date, currency, prepared by, active filters. */
  meta: { label: string; value: string }[];
  blocks: PrintBlock[];
  /** Signature box titles; empty = no signature row. */
  signatures: string[];
  orientation: 'portrait' | 'landscape';
  /** Formatted issue timestamp (also shown in the footer). */
  issuedAt: string;
  /** ISO date of issue — the Typst template stamps it into the PDF metadata. */
  issuedIso: string;
  footerNote: string;
}

/** What a report page contributes; `ReportShell` fills in company, period, issue date, user and filters. */
export interface ReportPrintSpec {
  /** Override the screen title/subtitle on paper (e.g. the ledger prints "كشف حساب" + an account banner). */
  title?: string;
  subtitle?: string;
  blocks: PrintBlock[];
  /** Extra meta cells (e.g. "الحساب", "العميل"). */
  meta?: { label: string; value: string }[];
  /** Default on for financial statements. */
  signatures?: boolean;
  orientation?: 'portrait' | 'landscape';
}

import { formatMoney, formatNumber } from '@/modules/core/helpers/format';
import type { ExportTable } from '../helpers/export';
import type { PrintBlock, PrintColumn, PrintKpi, PrintRow, PrintRowKind } from './types';

/** Signature boxes printed under financial statements (toggleable in the preview). */
export const DEFAULT_SIGNATURES = ['المحاسب', 'المدير المالي', 'المدير العام'];

// ---- Cell formatters (every printed number goes through format.ts, like the screens) ----------

/** 1,234.50 — or "—" for zero when `dashZero`. */
export function money(v: number | null | undefined, opts: { dashZero?: boolean } = {}): string {
  // Anything that rounds to zero (incl. -0 and -0.001) prints as a clean 0.00, never "-0.00".
  const n = Math.abs(v ?? 0) < 0.005 ? 0 : (v as number);
  if (opts.dashZero && n === 0) return '—';
  return formatMoney(n);
}

const clean0 = (v: number | null | undefined) => (Math.abs(v ?? 0) < 1e-9 ? 0 : (v as number));

/** Quantities keep up to 3 decimals (weighed items), counts none. */
export function qty(v: number | null | undefined): string {
  return formatNumber(clean0(v), 3);
}

export function count(v: number | null | undefined): string {
  return formatNumber(clean0(v), 0);
}

export function pct(v: number | null | undefined, digits = 1): string {
  return `${formatNumber(clean0(v), digits)}%`;
}

/**
 * Meta-strip columns: one row when it fits (6 portrait / 8 landscape), otherwise balanced rows
 * (8 cells → 4 + 4, not 6 + 2). Shared by the HTML renderer; `report.typ` applies the same rule.
 */
export function metaColumns(cells: number, landscape: boolean): number {
  const max = landscape ? 8 : 6;
  if (cells <= max) return Math.max(1, cells);
  return Math.ceil(cells / Math.ceil(cells / max));
}

// ---- Small block constructors, so page specs read like the document they describe ---------------

export const kpis = (items: PrintKpi[]): PrintBlock => ({ type: 'kpis', items });
export const heading = (text: string): PrintBlock => ({ type: 'heading', text });
export const note = (text: string, tone: 'ok' | 'warn' | 'info' = 'info'): PrintBlock => ({ type: 'note', text, tone });
export const row = (cells: string[], kind: PrintRowKind = 'normal'): PrintRow => ({ cells, kind });
export const table = (columns: PrintColumn[], rows: PrintRow[], emptyText?: string): PrintBlock => ({ type: 'table', columns, rows, emptyText });
export const boxes = (items: { title: string; value: string; sub?: string; emphasis?: boolean }[]): PrintBlock => ({ type: 'boxes', items });
export const banner = (title: string, subtitle?: string, tag?: string): PrintBlock => ({ type: 'banner', title, subtitle, tag });

/** A 3-column statement section (code | account | amount) with a subtotal — the balance sheet / income statement shape. */
export function statementRows(
  title: string,
  lines: { code?: string; name: string; amount: number }[],
  total: number,
  totalLabel = `إجمالي ${title}`,
): PrintRow[] {
  return [
    row([title], 'section'),
    ...lines.map((l) => row([l.code ?? '', l.name, money(l.amount)])),
    ...(lines.length ? [] : [row(['', 'لا توجد أرصدة', '—'], 'opening')]),
    row(['', totalLabel, money(total)], 'subtotal'),
  ];
}

export const STATEMENT_COLUMNS: PrintColumn[] = [
  { label: 'الرمز', dim: true, width: 0.7 },
  { label: 'الحساب', width: 3 },
  { label: 'المبلغ', numeric: true, width: 1.3 },
];

// ---- Generic conversion: any report's ExportTable → an official table block ----------------------
// Reports without a hand-written print spec still print as a real document (not a screenshot of the
// page): column kinds are inferred from the header labels, and section/total rows from their text.

type ColKind = 'text' | 'money' | 'qty' | 'pct';

const QTY_LABEL = /كمية|الكمية|عدد|أيام|النتيجة|الحد الأدنى|المقترح/;
const PCT_LABEL = /%|نسبة|هامش|الهامش/;
const MONEY_LABEL =
  /مبلغ|قيمة|القيمة|إجمالي|الإجمالي|رصيد|الرصيد|مدين|دائن|تكلفة|سعر|ضريبة|الضريبة|مبيعات|المبيعات|ربح|الربح|خصم|صافي|مصروف|إيراد|محصل|المتبقي|ميزانية|الميزانية|الفعلي|المستحق|الخاضع|متوسط|النقدية|المتوقع|المعدود|الفرق|الفترة/;

function columnKind(label: string, values: (string | number)[]): ColKind {
  const nums = values.filter((v): v is number => typeof v === 'number');
  if (!nums.length) return 'text';
  if (PCT_LABEL.test(label)) return 'pct';
  if (QTY_LABEL.test(label) && !/قيمة|تكلفة|سعر/.test(label)) return 'qty';
  if (MONEY_LABEL.test(label)) return 'money';
  return nums.every((n) => Number.isInteger(n)) ? 'qty' : 'money';
}

function formatCell(v: string | number, kind: ColKind): string {
  if (typeof v !== 'number') return v;
  if (kind === 'pct') return pct(v);
  if (kind === 'qty') return qty(v);
  return money(v);
}

const TOTAL_TEXT = /^(الإجمالي|إجمالي|المجموع|الرصيد الختامي|رصيد النقدية آخر المدة)/;
const SUBTOTAL_TEXT = /^(صافي|مجمل)/;
const OPENING_TEXT = /^(رصيد أول المدة|رصيد النقدية أول المدة)/;

function rowKind(cells: (string | number)[], isLast: boolean): PrintRowKind {
  const first = cells.find((c) => c !== '' && c !== null && c !== undefined);
  const text = typeof first === 'string' ? first.trim() : '';
  const filled = cells.filter((c) => c !== '' && c !== null && c !== undefined).length;
  if (OPENING_TEXT.test(text)) return 'opening';
  if (TOTAL_TEXT.test(text)) return isLast ? 'total' : 'subtotal';
  if (SUBTOTAL_TEXT.test(text)) return 'subtotal';
  if (filled === 1 && typeof first === 'string' && cells.length > 1 && cells[0] === first) return 'section';
  return 'normal';
}

export function blocksFromTable(t: ExportTable, metrics?: { label: string; value: string }[]): PrintBlock[] {
  const kinds = t.columns.map((label, i) => columnKind(label, t.rows.map((r) => r[i])));
  // The widest text column (names / descriptions) takes the free space; codes, dates and numbers
  // stay at their natural width.
  const avgLength = (i: number) => t.rows.reduce((a, r) => a + String(r[i] ?? '').length, 0) / Math.max(1, t.rows.length);
  const textCols = kinds.map((k, i) => (k === 'text' ? i : -1)).filter((i) => i >= 0);
  const main = textCols.length ? textCols.reduce((best, i) => (avgLength(i) > avgLength(best) ? i : best)) : -1;
  const columns: PrintColumn[] = t.columns.map((label, i) => ({
    label,
    numeric: kinds[i] !== 'text',
    width: i === main ? 3 : undefined,
  }));
  const rows: PrintRow[] = t.rows.map((r, idx) => {
    const kind = rowKind(r, idx === t.rows.length - 1);
    if (kind === 'section') return row([String(r[0])], 'section');
    return row(r.map((v, i) => formatCell(v, kinds[i])), kind);
  });

  // Flat lists (no section/total rows of their own) close with a totals row — only over columns
  // where a sum means something: amounts (not prices/averages/balances) and counts/plain quantities.
  const flat = rows.every((r) => r.kind === 'normal');
  const summable = (i: number) =>
    (kinds[i] === 'money' && !/متوسط|سعر|الوحدة|رصيد|الرصيد/.test(t.columns[i])) || (kinds[i] === 'qty' && /^(عدد|الكمية$)/.test(t.columns[i]));
  if (flat && t.rows.length > 1 && kinds.some((_, i) => summable(i))) {
    const labelCol = kinds.findIndex((k) => k === 'text');
    rows.push(
      row(
        t.columns.map((_, i) => {
          if (i === labelCol) return 'الإجمالي';
          if (!summable(i)) return '';
          const sum = t.rows.reduce((a, r) => a + (typeof r[i] === 'number' ? (r[i] as number) : 0), 0);
          return formatCell(Math.round(sum * 1000) / 1000, kinds[i]);
        }),
        'total',
      ),
    );
  }
  const blocks: PrintBlock[] = [];
  if (metrics && metrics.length >= 2) blocks.push(kpis(metrics.map((m) => ({ label: m.label, value: m.value }))));
  blocks.push(table(columns, rows, 'لا توجد بيانات لهذه الفترة'));
  return blocks;
}

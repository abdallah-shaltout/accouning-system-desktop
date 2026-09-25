/**
 * Excel export used by `DataTable`'s "تصدير" (export) button (docs/v2/14-platform.md §7 — every
 * table needs Excel export). `exceljs` is heavy, so it's dynamically imported here and only ever
 * loaded once the user actually clicks export.
 */
import { APP_NAME_AR } from './brand';
import { saveFile } from '@/modules/core/services/saveFile';

export interface ExportColumn<R = any> {
  key: string;
  label: string;
  /** Right-aligned, `.num`-formatted numeric column. */
  numeric?: boolean;
  /** Extract the exported value from a row; defaults to `row[key]`. */
  value?: (row: R) => string | number | Date | null | undefined;
}

export interface ExportXlsxOptions<R = any> {
  /** File name without extension. */
  fileName: string;
  sheetName?: string;
  columns: ExportColumn<R>[];
  /** Plain-array mode: every row to export (already filtered/sorted by the caller). */
  rows?: R[];
  /**
   * Server/paged mode: pulls every page of the *filtered* result set (not just the page on
   * screen) using the same paged contract `DataTable` uses. Ignored if `rows` is given.
   */
  fetchAll?: (query: { page: number; pageSize: number }) => Promise<{ rows: R[]; total: number }>;
  /** Page size used when paging through `fetchAll`. */
  pageSize?: number;
}

async function collectRows<R>(options: ExportXlsxOptions<R>): Promise<R[]> {
  if (options.rows) return options.rows;
  if (!options.fetchAll) return [];
  const pageSize = options.pageSize ?? 500;
  const all: R[] = [];
  let page = 1;
  // eslint-disable-next-line no-constant-condition
  while (true) {
    const result = await options.fetchAll({ page, pageSize });
    all.push(...result.rows);
    if (all.length >= result.total || !result.rows.length) break;
    page += 1;
  }
  return all;
}

/** Builds and downloads an .xlsx file from columns + rows (or a paged-fetch function for ALL filtered rows). */
export async function exportXlsx<R = any>(options: ExportXlsxOptions<R>): Promise<void> {
  const [{ default: ExcelJS }, rows] = await Promise.all([import('exceljs'), collectRows(options)]);

  const workbook = new ExcelJS.Workbook();
  workbook.creator = APP_NAME_AR;
  workbook.created = new Date();
  const sheet = workbook.addWorksheet(options.sheetName ?? 'البيانات', { views: [{ rightToLeft: true }] });

  sheet.columns = options.columns.map((c) => ({
    header: c.label,
    key: c.key,
    width: Math.max(12, c.label.length + 4),
    style: c.numeric ? { numFmt: '#,##0.00', alignment: { horizontal: 'right' } } : undefined,
  }));
  sheet.getRow(1).font = { bold: true };
  sheet.getRow(1).alignment = { horizontal: 'center' };

  for (const row of rows) {
    const record: Record<string, unknown> = {};
    for (const col of options.columns) record[col.key] = col.value ? col.value(row) : (row as any)[col.key];
    sheet.addRow(record);
  }

  const buffer = await workbook.xlsx.writeBuffer();
  await saveFile(new Uint8Array(buffer), { suggestedName: `${options.fileName}.xlsx`, kind: 'excel' });
}

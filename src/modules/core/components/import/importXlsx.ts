/**
 * v2 phase 5 (docs/v2/05-onboarding.md §5): xlsx template download + upload parsing for the generic
 * `ImportWizard`. `exceljs` is lazy-loaded here, same pattern as `modules/core/helpers/exportXlsx.ts`.
 */
import type { ImportColumn, ImportDescriptor } from './types';
import { APP_NAME_AR } from '@/modules/core/helpers/brand';
import { saveFile } from '@/modules/core/services/saveFile';

/** Builds and downloads the Arabic-header, RTL, data-validated .xlsx template for a descriptor. */
export async function downloadImportTemplate(descriptor: ImportDescriptor): Promise<void> {
  const { default: ExcelJS } = await import('exceljs');
  const workbook = new ExcelJS.Workbook();
  workbook.creator = APP_NAME_AR;
  workbook.created = new Date();
  const sheet = workbook.addWorksheet(descriptor.title, { views: [{ rightToLeft: true }] });

  sheet.columns = descriptor.columns.map((c) => ({ header: c.label, key: c.key, width: Math.max(14, c.label.length + 6) }));
  const headerRow = sheet.getRow(1);
  headerRow.font = { bold: true };
  headerRow.alignment = { horizontal: 'center' };

  // Notes row (row 2): required/format hints per column.
  const notes: Record<string, string> = {};
  descriptor.columns.forEach((c) => {
    notes[c.key] = [c.required ? 'مطلوب' : 'اختياري', c.note].filter(Boolean).join(' — ');
  });
  const notesRow = sheet.addRow(notes);
  notesRow.font = { italic: true, size: 9, color: { argb: 'FF888888' } };

  // Sample rows.
  for (const sample of descriptor.sampleRows ?? []) sheet.addRow(sample);

  // Data-validation dropdown lists for columns that declare `options`.
  descriptor.columns.forEach((c, i) => {
    if (!c.options?.length) return;
    const colLetter = sheet.getColumn(i + 1).letter;
    for (let r = 3; r <= 200; r++) {
      sheet.getCell(`${colLetter}${r}`).dataValidation = { type: 'list', allowBlank: true, formulae: [`"${c.options!.join(',')}"`] };
    }
  });

  const buffer = await workbook.xlsx.writeBuffer();
  await saveFile(new Uint8Array(buffer), { suggestedName: `قالب_${descriptor.title}.xlsx`, kind: 'excel' });
}

/** Normalizes a header string for fuzzy synonym matching (trim, collapse spaces, lowercase). */
function normalizeHeader(s: string): string {
  return s.trim().replace(/\s+/g, ' ').toLowerCase();
}

/** Auto-maps a sheet's header row to descriptor columns by synonym match. Returns headerIndex -> columnKey (or undefined if unmatched). */
export function autoMapHeaders(headers: string[], columns: ImportColumn[]): (string | undefined)[] {
  return headers.map((h) => {
    const norm = normalizeHeader(h ?? '');
    if (!norm) return undefined;
    const match = columns.find((c) => normalizeHeader(c.label) === norm || c.synonyms.some((s) => normalizeHeader(s) === norm));
    return match?.key;
  });
}

export interface ParsedSheet {
  headers: string[];
  rows: Record<string, unknown>[];
}

/** Reads an uploaded .xlsx or .csv file into headers + raw row objects (first row = headers; the descriptor's notes/sample rows aren't expected on upload — a user's own file works too). */
export async function parseUploadedFile(file: File): Promise<ParsedSheet> {
  if (file.name.toLowerCase().endsWith('.csv')) return parseCsv(await file.text());
  const { default: ExcelJS } = await import('exceljs');
  const workbook = new ExcelJS.Workbook();
  const buffer = await file.arrayBuffer();
  await workbook.xlsx.load(buffer);
  const sheet = workbook.worksheets[0];
  if (!sheet) return { headers: [], rows: [] };
  const headers: string[] = [];
  sheet.getRow(1).eachCell({ includeEmpty: true }, (cell, colNumber) => {
    headers[colNumber - 1] = String(cell.value ?? '').trim();
  });
  const rows: Record<string, unknown>[] = [];
  sheet.eachRow((row, rowNumber) => {
    if (rowNumber === 1) return;
    const obj: Record<string, unknown> = {};
    let hasValue = false;
    headers.forEach((h, i) => {
      if (!h) return;
      const cell = row.getCell(i + 1);
      let value: unknown = cell.value;
      if (value && typeof value === 'object' && 'text' in (value as any)) value = (value as any).text;
      if (value && typeof value === 'object' && 'result' in (value as any)) value = (value as any).result;
      obj[h] = value ?? undefined;
      if (value !== undefined && value !== null && value !== '') hasValue = true;
    });
    if (hasValue) rows.push(obj);
  });
  return { headers: headers.filter(Boolean), rows };
}

function parseCsv(text: string): ParsedSheet {
  const lines = text.split(/\r?\n/).filter((l) => l.trim().length);
  if (!lines.length) return { headers: [], rows: [] };
  const splitLine = (line: string) => line.split(',').map((c) => c.trim().replace(/^"|"$/g, ''));
  const headers = splitLine(lines[0]);
  const rows = lines.slice(1).map((line) => {
    const cells = splitLine(line);
    const obj: Record<string, unknown> = {};
    headers.forEach((h, i) => (obj[h] = cells[i]));
    return obj;
  });
  return { headers, rows };
}

/** Downloads a .xlsx of failed rows with a "reason" column (docs/v2/05 §5 "a downloadable error file"). */
export async function downloadErrorFile(descriptor: ImportDescriptor, failedRows: { raw: Record<string, unknown>; reason: string }[]): Promise<void> {
  const { default: ExcelJS } = await import('exceljs');
  const workbook = new ExcelJS.Workbook();
  const sheet = workbook.addWorksheet('الأخطاء', { views: [{ rightToLeft: true }] });
  const headerKeys = descriptor.columns.map((c) => c.label);
  sheet.addRow([...headerKeys, 'السبب']).font = { bold: true };
  for (const f of failedRows) {
    sheet.addRow([...descriptor.columns.map((c) => String(f.raw[c.label] ?? '')), f.reason]);
  }
  const buffer = await workbook.xlsx.writeBuffer();
  await saveFile(new Uint8Array(buffer), { suggestedName: `أخطاء_${descriptor.title}.xlsx`, kind: 'excel' });
}

const MAPPING_STORAGE_PREFIX = 'import-mapping:';

export function loadSavedMapping(descriptorKey: string): Record<string, string> | undefined {
  try {
    const raw = localStorage.getItem(MAPPING_STORAGE_PREFIX + descriptorKey);
    return raw ? JSON.parse(raw) : undefined;
  } catch {
    return undefined;
  }
}

export function saveMapping(descriptorKey: string, headerToColumnKey: Record<string, string>): void {
  try {
    localStorage.setItem(MAPPING_STORAGE_PREFIX + descriptorKey, JSON.stringify(headerToColumnKey));
  } catch {
    /* best-effort */
  }
}

import { isTauri } from '@tauri-apps/api/core';

/** A plain table snapshot of whatever a report screen currently renders. */
export interface ExportTable {
  title: string;
  /** Extra context lines (period, generated-at…) placed above the table. */
  meta?: string[];
  columns: string[];
  rows: (string | number)[][];
}

const cell = (v: string | number) => (typeof v === 'number' ? v.toFixed(2).replace(/\.00$/, '') : v);

export function toCsv(table: ExportTable): string {
  const esc = (v: string | number) => {
    const s = String(cell(v));
    return /[",\n]/.test(s) ? `"${s.replace(/"/g, '""')}"` : s;
  };
  const lines = [...(table.meta ?? []).map((m) => esc(m)), table.columns.map(esc).join(','), ...table.rows.map((r) => r.map(esc).join(','))];
  // BOM so Excel opens Arabic text as UTF-8.
  return '﻿' + lines.join('\r\n');
}

/** GitHub-flavored Markdown table — easy to paste into docs or hand to an AI agent. */
export function toMarkdown(table: ExportTable): string {
  const esc = (v: string | number) => String(cell(v)).replace(/\|/g, '\\|').replace(/\n/g, ' ');
  const out = [`# ${table.title}`, ''];
  for (const m of table.meta ?? []) out.push(`- ${m}`);
  if (table.meta?.length) out.push('');
  out.push(`| ${table.columns.map(esc).join(' | ')} |`);
  out.push(`| ${table.columns.map(() => '---').join(' | ')} |`);
  for (const r of table.rows) out.push(`| ${r.map(esc).join(' | ')} |`);
  return out.join('\n') + '\n';
}

function safeName(name: string) {
  return name.replace(/[\\/:*?"<>|]+/g, '-').trim();
}

/**
 * Save text to disk: a native "Save as" dialog under Tauri, a regular download in the browser.
 * Returns false if the user cancelled the dialog.
 */
export async function saveTextFile(filename: string, content: string, kind: 'csv' | 'md'): Promise<boolean> {
  const name = safeName(filename);
  if (isTauri()) {
    const [{ save }, { writeTextFile }] = await Promise.all([import('@tauri-apps/plugin-dialog'), import('@tauri-apps/plugin-fs')]);
    const path = await save({
      defaultPath: name,
      filters: [kind === 'csv' ? { name: 'CSV', extensions: ['csv'] } : { name: 'Markdown', extensions: ['md'] }],
    });
    if (!path) return false;
    await writeTextFile(path, content);
    return true;
  }
  const blob = new Blob([content], { type: kind === 'csv' ? 'text/csv;charset=utf-8' : 'text/markdown;charset=utf-8' });
  const url = URL.createObjectURL(blob);
  const a = document.createElement('a');
  a.href = url;
  a.download = name;
  document.body.appendChild(a);
  a.click();
  a.remove();
  setTimeout(() => URL.revokeObjectURL(url), 1000);
  return true;
}

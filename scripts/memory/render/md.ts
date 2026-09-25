/** Markdown primitives shared by every section renderer. */

const cell = (v: unknown) => String(v ?? '').replace(/\|/g, '\\|').replace(/\n/g, ' ');

export function table(headers: string[], rows: unknown[][]): string {
  if (!rows.length) return '_none_';
  return [
    `| ${headers.join(' | ')} |`,
    `|${headers.map(() => '---').join('|')}|`,
    ...rows.map((r) => `| ${r.map(cell).join(' | ')} |`),
  ].join('\n');
}

export const code = (s: string) => '`' + s + '`';

/** Comma list with a hard cap so one huge file cannot bloat the memory. */
export function list(items: string[], max = 30, wrap = code): string {
  if (!items.length) return '—';
  const shown = items.slice(0, max).map(wrap).join(', ');
  return items.length > max ? `${shown} … +${items.length - max} more` : shown;
}

export const section = (title: string, ...body: string[]) => [`## ${title}`, ...body].join('\n\n');

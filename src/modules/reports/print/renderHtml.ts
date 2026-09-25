/**
 * ReportDocument → a standalone, official A4 HTML document (letterhead, title row with badge, meta
 * strip, KPI bar, tables with section/subtotal/total rows, signatures, running footer with page
 * x of y). Layout ported from the reference accounting system's report partials
 * (`references/accouning-system/.../pdf/report/shared/layout.partial.ts` + `templates/*.hbs`),
 * which render Handlebars → HTML → Chromium PDF. Here the same HTML is printed from an iframe
 * (browser + desktop) and shown in the preview dialog; the desktop "PDF" button renders the same
 * model through `src-tauri/templates/report.typ` instead.
 */
import plexArabic400 from '@fontsource/ibm-plex-sans-arabic/files/ibm-plex-sans-arabic-arabic-400-normal.woff2?url';
import plexArabic700 from '@fontsource/ibm-plex-sans-arabic/files/ibm-plex-sans-arabic-arabic-700-normal.woff2?url';
import plexLatin400 from '@fontsource/ibm-plex-sans-arabic/files/ibm-plex-sans-arabic-latin-400-normal.woff2?url';
import plexLatin700 from '@fontsource/ibm-plex-sans-arabic/files/ibm-plex-sans-arabic-latin-700-normal.woff2?url';
import { metaColumns } from './build';
import type { PrintBlock, PrintColumn, PrintRow, ReportDocument } from './types';

const ARABIC_RANGE = 'U+0600-06FF,U+0750-077F,U+0870-088E,U+0890-0891,U+0897-08E1,U+08E3-08FF,U+200C-200E,U+2010-2011,U+204F,U+2E41,U+FB50-FDFF,U+FE70-FE74,U+FE76-FEFC';
const LATIN_RANGE = 'U+0000-00FF,U+0131,U+0152-0153,U+02BB-02BC,U+02C6,U+02DA,U+02DC,U+0304,U+0308,U+0329,U+2000-206F,U+20AC,U+2122,U+2191,U+2193,U+2212,U+2215,U+FEFF,U+FFFD';

const abs = (url: string) => new URL(url, window.location.href).href;

function fontFaces(): string {
  const face = (url: string, weight: number, range: string) =>
    `@font-face{font-family:'ReportFont';font-style:normal;font-weight:${weight};font-display:block;src:url('${abs(url)}') format('woff2');unicode-range:${range};}`;
  return [
    face(plexArabic400, 400, ARABIC_RANGE),
    face(plexArabic700, 700, ARABIC_RANGE),
    face(plexLatin400, 400, LATIN_RANGE),
    face(plexLatin700, 700, LATIN_RANGE),
  ].join('\n');
}

const esc = (s: string | undefined | null) =>
  String(s ?? '')
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;')
    .replace(/"/g, '&quot;');

/** A CSS string literal (for `@page` margin-box `content`). */
const cssString = (s: string) => `"${s.replace(/\\/g, '\\\\').replace(/"/g, '\\"').replace(/\n/g, ' ')}"`;

function styles(doc: ReportDocument): string {
  const landscape = doc.orientation === 'landscape';
  return `
${fontFaces()}
@page {
  size: A4 ${landscape ? 'landscape' : 'portrait'};
  margin: 11mm 12mm 14mm;
  @bottom-left { content: "صفحة " counter(page) " من " counter(pages); font-family: 'ReportFont', Tahoma, Arial, sans-serif; font-size: 7pt; color: #777; }
  @bottom-center { content: ${cssString(doc.footerNote)}; font-family: 'ReportFont', Tahoma, Arial, sans-serif; font-size: 7pt; color: #999; }
  @bottom-right { content: ${cssString(`${doc.company.name} — ${doc.title}`)}; font-family: 'ReportFont', Tahoma, Arial, sans-serif; font-size: 7pt; color: #777; }
}
*, *::before, *::after { margin: 0; padding: 0; box-sizing: border-box; -webkit-print-color-adjust: exact; print-color-adjust: exact; }
:root { --black: #262626; --ink: #333; --mid: #555; --muted: #808080; --rule: #c6c6c6; --hair: #e2e2e2; --stripe: #f4f4f4; --band: #eceff4; --paper: #fff; }
html, body { font-family: 'ReportFont', Tahoma, Arial, sans-serif; font-size: 10px; color: var(--ink); line-height: 1.5; }
body.screen { background: #dcdcdc; padding: 18px 0; }
.page { background: var(--paper); }
body.screen .page { width: ${landscape ? '297mm' : '210mm'}; min-height: ${landscape ? '210mm' : '297mm'}; margin: 0 auto; padding: 11mm 12mm 12mm; box-shadow: 0 1px 3px rgba(0,0,0,.18), 0 6px 24px rgba(0,0,0,.08); zoom: var(--fit, 1); }

/* Letterhead */
.hdr { display: flex; justify-content: space-between; align-items: flex-start; gap: 16px; padding-bottom: 9px; border-bottom: 3px solid var(--black); margin-bottom: 10px; }
.hdr .name { font-size: 16px; font-weight: 700; color: var(--black); }
.hdr .facts { margin-top: 4px; font-size: 9px; color: var(--mid); line-height: 1.7; }
.hdr .facts span + span::before { content: "  ·  "; color: var(--rule); }
.hdr .logo { border: 1px solid var(--rule); background: var(--stripe); padding: 6px 12px; border-radius: 3px; }
.hdr .logo img { max-width: 84px; max-height: 50px; object-fit: contain; display: block; }

/* Title row */
.title-row { display: flex; justify-content: space-between; align-items: flex-end; gap: 12px; padding-bottom: 7px; margin-bottom: 8px; border-bottom: 1px solid var(--rule); }
.title-row h1 { font-size: 15px; font-weight: 700; color: var(--black); }
.title-row .subtitle { font-size: 9px; color: var(--muted); margin-top: 2px; }
.badge { font-size: 8.5px; font-weight: 700; color: var(--mid); border: 1px solid var(--rule); padding: 3px 10px; white-space: nowrap; }

/* Meta strip — a ruled grid (1px gaps over the rule colour draw the lines) */
.meta { display: grid; gap: 1px; background: var(--rule); border: 1px solid var(--rule); margin-bottom: 10px; break-inside: avoid; }
.meta .cell { background: var(--paper); min-width: 0; padding: 5px 10px; }
.meta .lbl { font-size: 8px; font-weight: 700; color: var(--muted); margin-bottom: 1px; }
.meta .val { font-size: 10px; font-weight: 700; color: var(--ink); }

/* KPI bar */
.kpis { display: flex; border: 1px solid var(--rule); margin-bottom: 10px; break-inside: avoid; }
.kpis .cell { flex: 1 1 0; padding: 6px 10px; border-left: 1px solid var(--rule); text-align: center; }
.kpis .cell:last-child { border-left: none; }
.kpis .cell.em { background: var(--band); }
.kpis .lbl { font-size: 8px; color: var(--muted); margin-bottom: 2px; }
.kpis .val { font-size: 13px; font-weight: 700; color: var(--black); direction: ltr; unicode-bidi: isolate; font-variant-numeric: tabular-nums; }

/* Section label */
.sec { font-size: 10px; font-weight: 700; color: var(--black); margin: 12px 0 5px; padding-right: 7px; border-right: 3px solid var(--black); break-after: avoid; }

/* Tables */
table { width: 100%; border-collapse: collapse; font-size: 9.5px; margin-bottom: 10px; }
thead { display: table-header-group; }
thead th { background: var(--band); color: var(--black); font-weight: 700; font-size: 9px; padding: 5px 7px; text-align: right; white-space: nowrap; border-top: 1px solid var(--black); border-bottom: 1px solid var(--black); }
tbody td { padding: 4px 7px; border-bottom: 1px solid var(--hair); vertical-align: top; }
tbody tr { break-inside: avoid; }
tbody tr.normal:nth-child(even) td { background: var(--stripe); }
th.a-center, td.a-center { text-align: center; }
th.a-end, td.a-end { text-align: left; }
th.n { text-align: left; }
td.n { direction: ltr; text-align: left; white-space: nowrap; font-variant-numeric: tabular-nums; }
td.n.a-center { text-align: center; }
td.n.a-start { text-align: right; }
td.dim { color: var(--muted); font-size: 8.5px; }
tr.section td { background: var(--band); color: var(--black); font-weight: 700; font-size: 10px; padding: 5px 8px; border-bottom: 1px solid var(--rule); }
tr.subhead td { background: var(--stripe); color: var(--mid); font-weight: 700; padding: 4px 8px; }
tr.opening td { background: var(--stripe); color: var(--mid); font-style: italic; }
tr.subtotal td { font-weight: 700; border-top: 1px solid var(--rule); border-bottom: 2px solid var(--rule); background: #fafafa; }
tr.total td { font-weight: 700; background: var(--band); border-top: 1.5px solid var(--black); border-bottom: 1.5px solid var(--black); color: var(--black); }
tr.grand td { font-weight: 700; font-size: 11px; color: var(--black); border-top: 2px solid var(--black); border-bottom: 3px double var(--black); padding: 6px 8px; }
.empty { text-align: center; padding: 18px; color: var(--muted); border: 1px dashed var(--rule); margin-bottom: 10px; }

/* Side-by-side columns */
.cols { display: flex; gap: 10px; align-items: flex-start; }
.cols > .col { flex: 1 1 0; min-width: 0; }

/* Banner (ledger account / party) */
.banner { display: flex; justify-content: space-between; align-items: center; border: 1px solid var(--rule); background: var(--stripe); padding: 7px 12px; margin-bottom: 10px; }
.banner .t { font-size: 12px; font-weight: 700; color: var(--black); }
.banner .s { font-size: 9px; color: var(--muted); margin-top: 1px; }
.banner .tag { font-size: 8.5px; border: 1px solid var(--rule); background: var(--paper); padding: 2px 8px; color: var(--mid); }

/* Note */
.note { border: 1px solid var(--rule); border-right: 3px solid var(--black); padding: 6px 10px; margin-bottom: 10px; font-weight: 700; font-size: 9.5px; break-inside: avoid; }
.note.warn { border-right-color: #9b1c1c; color: #9b1c1c; }

/* Big-number boxes */
.boxes { display: flex; gap: 8px; margin-bottom: 12px; break-inside: avoid; }
.boxes .box { flex: 1 1 0; border: 1px solid var(--rule); padding: 10px 12px; }
.boxes .box.em { background: var(--band); border-color: var(--black); }
.boxes .bt { font-size: 8.5px; font-weight: 700; color: var(--muted); border-bottom: 1px solid var(--rule); padding-bottom: 4px; margin-bottom: 6px; }
.boxes .bv { font-size: 17px; font-weight: 700; color: var(--black); direction: ltr; unicode-bidi: isolate; text-align: right; font-variant-numeric: tabular-nums; }
.boxes .bs { font-size: 8.5px; color: var(--muted); margin-top: 2px; }

/* Signatures */
.signatures { display: flex; justify-content: space-around; gap: 16px; margin-top: 22px; padding-top: 10px; break-inside: avoid; }
.sig { text-align: center; width: 150px; }
.sig .st { font-size: 9px; font-weight: 700; color: var(--mid); margin-bottom: 28px; }
.sig .sl { border-top: 1px solid var(--ink); padding-top: 3px; font-size: 8px; color: var(--muted); }

/* End-of-report rule + screen-only footer (on paper the @page margin boxes carry it on every page) */
.end { margin-top: 12px; border-top: 2px solid var(--black); }
.ftr { display: flex; justify-content: space-between; gap: 12px; padding-top: 5px; font-size: 8px; color: var(--muted); }

@media print {
  body.screen { background: none; padding: 0; }
  body.screen .page { width: auto; min-height: 0; margin: 0; padding: 0; box-shadow: none; zoom: 1; }
  .end, .ftr { display: none; }
}
`;
}

function alignClass(c: PrintColumn): string {
  const align = c.align ?? (c.numeric ? 'end' : 'start');
  return `a-${align}`;
}

function cellClass(c: PrintColumn): string {
  return [c.numeric ? 'n' : '', alignClass(c), c.dim ? 'dim' : ''].filter(Boolean).join(' ');
}

function renderTable(columns: PrintColumn[], rows: PrintRow[], emptyText?: string, hideHeader?: boolean): string {
  if (!rows.length && emptyText) return `<div class="empty">${esc(emptyText)}</div>`;
  // Relative widths: columns without one weigh 1, so a single `width: 3` means "three shares".
  const sized = columns.some((c) => c.width);
  const totalWidth = columns.reduce((a, c) => a + (c.width ?? 1), 0);
  const colgroup = sized ? `<colgroup>${columns.map((c) => `<col style="width:${(((c.width ?? 1) / totalWidth) * 100).toFixed(2)}%">`).join('')}</colgroup>` : '';
  const head = hideHeader ? '' : `<thead><tr>${columns.map((c) => `<th class="${c.numeric ? 'n ' : ''}${alignClass(c)}">${esc(c.label)}</th>`).join('')}</tr></thead>`;
  const body = rows
    .map((r) => {
      const kind = r.kind ?? 'normal';
      if (kind === 'section' || kind === 'subhead') return `<tr class="${kind}"><td colspan="${columns.length}">${esc(r.cells.find((c) => c) ?? '')}</td></tr>`;
      return `<tr class="${kind}">${columns.map((c, i) => `<td class="${cellClass(c)}">${esc(r.cells[i] ?? '')}</td>`).join('')}</tr>`;
    })
    .join('');
  return `<table>${colgroup}${head}<tbody>${body}</tbody></table>`;
}

function renderBlock(b: PrintBlock): string {
  switch (b.type) {
    case 'kpis':
      return `<div class="kpis">${b.items.map((k) => `<div class="cell${k.emphasis ? ' em' : ''}"><div class="lbl">${esc(k.label)}</div><div class="val">${esc(k.value)}</div></div>`).join('')}</div>`;
    case 'heading':
      return `<div class="sec">${esc(b.text)}</div>`;
    case 'table':
      return renderTable(b.columns, b.rows, b.emptyText, b.hideHeader);
    case 'columns':
      return `<div class="cols">${b.columns.map((stack) => `<div class="col">${stack.map(renderBlock).join('')}</div>`).join('')}</div>`;
    case 'banner':
      return `<div class="banner"><div><div class="t">${esc(b.title)}</div>${b.subtitle ? `<div class="s">${esc(b.subtitle)}</div>` : ''}</div>${b.tag ? `<div class="tag">${esc(b.tag)}</div>` : ''}</div>`;
    case 'note':
      return `<div class="note${b.tone === 'warn' ? ' warn' : ''}">${esc(b.text)}</div>`;
    case 'boxes':
      return `<div class="boxes">${b.items
        .map((x) => `<div class="box${x.emphasis ? ' em' : ''}"><div class="bt">${esc(x.title)}</div><div class="bv">${esc(x.value)}</div>${x.sub ? `<div class="bs">${esc(x.sub)}</div>` : ''}</div>`)
        .join('')}</div>`;
  }
}

/** `mode: 'screen'` shows the sheet on a grey desk (preview); printing always uses the print styles. */
export function renderReportHtml(doc: ReportDocument, mode: 'screen' | 'print' = 'screen'): string {
  const c = doc.company;
  const facts = [
    c.vatNumber && `الرقم الضريبي: ${c.vatNumber}`,
    c.commercialRegister && `السجل التجاري: ${c.commercialRegister}`,
    c.address,
    c.phone && `هاتف: ${c.phone}`,
  ].filter(Boolean) as string[];

  const header = `<header class="hdr">
  <div><div class="name">${esc(c.name)}</div>${facts.length ? `<div class="facts">${facts.map((f) => `<span>${esc(f)}</span>`).join('')}</div>` : ''}</div>
  ${c.logo ? `<div class="logo"><img src="${esc(c.logo)}" alt=""></div>` : ''}
</header>`;

  const title = `<div class="title-row"><div><h1>${esc(doc.title)}</h1>${doc.subtitle ? `<div class="subtitle">${esc(doc.subtitle)}</div>` : ''}</div><div class="badge">${esc(doc.badge)}</div></div>`;
  const metaCols = metaColumns(doc.meta.length, doc.orientation === 'landscape');
  const metaPad = (metaCols - (doc.meta.length % metaCols)) % metaCols;
  const meta = doc.meta.length
    ? `<div class="meta" style="grid-template-columns:repeat(${metaCols},minmax(0,1fr))">${doc.meta
        .map((m) => `<div class="cell"><div class="lbl">${esc(m.label)}</div><div class="val">${esc(m.value)}</div></div>`)
        .join('')}${'<div class="cell"></div>'.repeat(metaPad)}</div>`
    : '';
  const signatures = doc.signatures.length
    ? `<div class="signatures">${doc.signatures.map((s) => `<div class="sig"><div class="st">${esc(s)}</div><div class="sl">الاسم / التوقيع</div></div>`).join('')}</div>`
    : '';
  const footer = `<div class="end"></div><footer class="ftr"><span>${esc(doc.company.name)} — ${esc(doc.title)}</span><span>${esc(doc.footerNote)}</span><span>تاريخ الإصدار: ${esc(doc.issuedAt)}</span></footer>`;

  // Screen preview: shrink the fixed-width sheet to fit the dialog. Print styles reset the zoom.
  const fit =
    mode === 'screen'
      ? `<script>(function(){var w=${doc.orientation === 'landscape' ? 1123 : 794};function f(){var s=Math.min(1,(window.innerWidth-36)/w);document.documentElement.style.setProperty('--fit',String(s>0.3?s:0.3));}f();window.addEventListener('resize',f);})();</script>`
      : '';

  return `<!DOCTYPE html>
<html lang="ar" dir="rtl">
<head><meta charset="utf-8"><title>${esc(doc.title)}</title><style>${styles(doc)}</style></head>
<body class="${mode}">
<div class="page">
${header}
${title}
${meta}
${doc.blocks.map(renderBlock).join('\n')}
${signatures}
${footer}
</div>
${fit}
</body>
</html>`;
}

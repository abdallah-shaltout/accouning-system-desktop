/**
 * Shared "Save as…" helper (docs/v2/17-ui-system-rtl-themes.md Phase C): every exported file
 * (Excel, JSON, CSV/MD, backup, PDF) goes through the native Tauri Save dialog on desktop, with a
 * plain `<a download>` fallback in a browser (dev/e2e — Playwright's `expect_download()` needs a
 * real anchor click, not a Tauri dialog it can't drive).
 *
 * Plan 21 Part 04 phase D (D-3): the Tauri e2e path (`--target tauri`, `scripts/e2e/common.py`)
 * drives the real Tauri window, where `isTauri()` is true and a native Save dialog would block
 * automation the same way it does in a real browser. Rule 21 allows the browser-download fallback
 * "only for dev/e2e", and this is exactly that case: when running a dev build
 * (`import.meta.env.DEV`) with `localStorage['equal.e2e'] === '1'` (set by `open_page()`/
 * `reset_backend()` in common.py, never in a production build), the Tauri branch below also uses
 * `browserDownload()` instead of the native dialog. `expect_saved_file()` (common.py) then watches
 * the CDP session's configured download directory for the result. Production builds are
 * unaffected: `import.meta.env.DEV` is `false` there regardless of `localStorage`.
 */
import { isTauri } from '@tauri-apps/api/core';
import { useToast } from '@/modules/core/controllers/useToast';

import { wrap } from '@/modules/diagnostics/services/defineService';

export type SaveFileKind = 'excel' | 'pdf' | 'backup' | 'json' | 'text' | 'image';

const EXTENSION_FILTERS: Record<SaveFileKind, { name: string; extensions: string[] }> = {
  excel: { name: 'Excel', extensions: ['xlsx'] },
  pdf: { name: 'PDF', extensions: ['pdf'] },
  backup: { name: 'Backup', extensions: ['zip'] },
  json: { name: 'JSON', extensions: ['json'] },
  text: { name: 'Text', extensions: ['csv', 'md', 'txt'] },
  image: { name: 'PNG', extensions: ['png'] },
};

const LAST_FOLDER_KEY_PREFIX = 'app_save_last_folder:';

function safeName(name: string): string {
  return name.replace(/[\\/:*?"<>|]+/g, '-').trim();
}

function lastFolder(kind: SaveFileKind): string | undefined {
  try {
    return localStorage.getItem(LAST_FOLDER_KEY_PREFIX + kind) ?? undefined;
  } catch {
    return undefined;
  }
}

function rememberFolder(kind: SaveFileKind, path: string): void {
  const idx = Math.max(path.lastIndexOf('/'), path.lastIndexOf('\\'));
  if (idx <= 0) return;
  try {
    localStorage.setItem(LAST_FOLDER_KEY_PREFIX + kind, path.slice(0, idx));
  } catch {
    /* private mode — preference just won't persist */
  }
}

function toBytes(data: Uint8Array | Blob | string): Promise<Uint8Array> | Uint8Array {
  if (data instanceof Uint8Array) return data;
  if (typeof data === 'string') return new TextEncoder().encode(data);
  return data.arrayBuffer().then((buf) => new Uint8Array(buf));
}

function mimeFor(kind: SaveFileKind): string {
  switch (kind) {
    case 'excel':
      return 'application/vnd.openxmlformats-officedocument.spreadsheetml.sheet';
    case 'pdf':
      return 'application/pdf';
    case 'backup':
      return 'application/zip';
    case 'json':
      return 'application/json';
    case 'text':
      return 'text/plain;charset=utf-8';
    case 'image':
      return 'image/png';
  }
}

function browserDownload(bytes: Uint8Array, filename: string, kind: SaveFileKind): void {
  const blob = new Blob([bytes.slice().buffer], { type: mimeFor(kind) });
  const url = URL.createObjectURL(blob);
  const a = document.createElement('a');
  a.href = url;
  a.download = filename;
  document.body.appendChild(a);
  a.click();
  a.remove();
  setTimeout(() => URL.revokeObjectURL(url), 1000);
}

/** D-3: true only in a dev build running under the Tauri e2e path (`scripts/e2e/common.py`'s
 * `open_page()`/`reset_backend()` set this flag) — never true in a production build, since
 * `import.meta.env.DEV` is always false there regardless of `localStorage` content. */
function isE2eSaveFallback(): boolean {
  if (!import.meta.env.DEV) return false;
  try {
    return localStorage.getItem('equal.e2e') === '1';
  } catch {
    return false;
  }
}

export interface SaveFileOptions {
  /** File name including extension, e.g. "الفواتير 2026-01-01_2026-09-25.xlsx". */
  suggestedName: string;
  kind: SaveFileKind;
  /** Skip the "تم الحفظ" toast (the caller shows its own, e.g. the PDF viewer opening instead). */
  silent?: boolean;
}

/**
 * Saves `data` to disk: the native Tauri Save dialog + `plugin-fs` write on desktop (remembering
 * the last folder used per file kind), a plain `<a download>` in a browser. Returns the chosen
 * path (desktop) or `true` (browser, path unknown) on success, `null` if the user cancelled.
 */
export const saveFile = wrap('core.saveFile', async function saveFile(data: Uint8Array | Blob | string, options: SaveFileOptions): Promise<string | true | null> {
  const name = safeName(options.suggestedName);
  const bytes = await toBytes(data);

  if (!isTauri() || isE2eSaveFallback()) {
    browserDownload(bytes, name, options.kind);
    return true;
  }

  const [{ save }, { writeFile }] = await Promise.all([import('@tauri-apps/plugin-dialog'), import('@tauri-apps/plugin-fs')]);
  const path = await save({
    defaultPath: lastFolder(options.kind) ? `${lastFolder(options.kind)}/${name}` : name,
    filters: [EXTENSION_FILTERS[options.kind]],
  });
  if (!path) return null;

  await writeFile(path, bytes);
  rememberFolder(options.kind, path);

  if (!options.silent) {
    const fileName = path.slice(Math.max(path.lastIndexOf('/'), path.lastIndexOf('\\')) + 1);
    useToast().successWithActions('تم الحفظ', fileName, [
      {
        label: 'فتح المجلد',
        onClick: () => {
          import('@tauri-apps/plugin-opener').then(({ revealItemInDir }) => revealItemInDir(path)).catch(() => {});
        },
      },
    ]);
  }

  return path;
});

/**
 * "تصدير ملف التشخيص" (18.B6) — a zip a non-technical owner can send to support: every log
 * channel's entries, app version, OS, and redacted store settings, plus an optional (explicit
 * opt-in) DB snapshot. Saved through the shared `saveFile` helper (rule 21 — native Save dialog,
 * never a silent `<a download>`). Seam-safe: settings/DB are read through `@/mocks`'s public
 * surface, never `src/mocks/*` directly from a page.
 */
import { zipSync } from 'fflate';
import { isTauri } from '@tauri-apps/api/core';
import { clone, db } from '@/mocks';
import { saveFile } from '@/modules/core/services/saveFile';
import { REDACTED_KEYS } from '../config';
import { exportAll } from './diagnosticsService';
import { wrap } from './defineService';

const REDACTED_VALUE = '[محجوب]';
const REDACTED_KEY_SET = new Set(REDACTED_KEYS.map((k) => k.toLowerCase()));

function redact(value: unknown, depth = 0): unknown {
  if (depth > 8 || value == null) return value;
  if (Array.isArray(value)) return value.map((v) => redact(v, depth + 1));
  if (typeof value === 'object') {
    const out: Record<string, unknown> = {};
    for (const [k, v] of Object.entries(value as Record<string, unknown>)) {
      out[k] = REDACTED_KEY_SET.has(k.toLowerCase()) ? REDACTED_VALUE : redact(v, depth + 1);
    }
    return out;
  }
  return value;
}

async function appVersion(): Promise<string> {
  if (!isTauri()) return '0.0.0-dev';
  try {
    const { getVersion } = await import('@tauri-apps/api/app');
    return await getVersion();
  } catch {
    return 'unknown';
  }
}

function osInfo(): string {
  // No `@tauri-apps/plugin-os` dependency yet — the user agent string is enough for a support
  // bundle's "what platform is this" line and needs no new Rust/Cargo surface.
  return typeof navigator !== 'undefined' ? navigator.userAgent : 'unknown';
}

export interface SupportBundleOptions {
  /** Explicit opt-in only (18.B6) — the bundle never includes a DB snapshot silently. */
  includeDbSnapshot?: boolean;
}

/** Builds and saves the support bundle zip. Returns the saved path (desktop) or `true` (browser),
 * `null` if the user cancelled the save dialog. */
export const exportSupportBundle = wrap('diagnostics.exportSupportBundle', async function exportSupportBundle(options: SupportBundleOptions = {}): Promise<string | true | null> {
  const [logs, version] = await Promise.all([exportAll(), appVersion()]);

  const meta = {
    generatedAt: new Date().toISOString(),
    appVersion: version,
    os: osInfo(),
    schemaVersion: 1,
  };

  const files: Record<string, Uint8Array> = {
    'meta.json': new TextEncoder().encode(JSON.stringify(meta, null, 2)),
    'settings.redacted.json': new TextEncoder().encode(JSON.stringify(redact(clone(db.settings)), null, 2)),
  };
  for (const [channel, entries] of Object.entries(logs)) {
    files[`logs/${channel}.jsonl`] = new TextEncoder().encode(entries.map((e) => JSON.stringify(e)).join('\n'));
  }
  if (options.includeDbSnapshot) {
    files['db-snapshot.json'] = new TextEncoder().encode(JSON.stringify(clone(db)));
  }

  const bytes = zipSync(files, { level: 6 });
  const stamp = new Date().toISOString().slice(0, 19).replace(/[:T]/g, '-');
  return saveFile(bytes, { suggestedName: `equal-diagnostics-${stamp}.zip`, kind: 'backup' });
});

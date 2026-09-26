/**
 * Rebuilds `src/modules/core/data/geo/{eg,sa}.json` from pinned upstream source commits.
 *
 * Run with: `bun run geo:build`
 *
 * STATUS (2026-09-26): this script has NOT been run against the real upstream data yet — the
 * session that wrote it had no network access. The committed `eg.json`/`sa.json` are hand-authored
 * placeholder/seed datasets (see their own `source`/`license` fields). This script documents the
 * exact shape/URLs so a future run with network access can fetch, normalize, and overwrite them.
 *
 * Egypt (MIT — safe to bundle for real):
 *   Repo: https://github.com/Tech-Labs/egypt-governorates-and-cities-db
 *   Files (raw, pin to a specific commit SHA before shipping — do not track a moving branch):
 *     https://raw.githubusercontent.com/Tech-Labs/egypt-governorates-and-cities-db/<COMMIT>/governorates.json
 *       -> 27 rows: { id, governorate_name_ar, governorate_name_en }
 *     https://raw.githubusercontent.com/Tech-Labs/egypt-governorates-and-cities-db/<COMMIT>/cities.json
 *       -> ~396 rows: { id, governorate_id, city_name_ar, city_name_en }
 *   Egypt has no open "district/قسم" level in this dataset — `districts` stays `[]` for every city,
 *   matching the placeholder's shape (the address form's `district` field then falls back to
 *   free text, per AddressFields' "غير موجود في القائمة؟" path).
 *
 * Saudi Arabia (GPL-2.0 — BLOCKED, see plans/pending/18-countries-a11y-diagnostics/README.md):
 *   Repo: homaily/saudi-arabia-regions-cities-districts (or equivalent — the open licensing
 *   decision must be resolved by the user before this half of the script is ever pointed at real
 *   data). Do not wire this up until that decision lands; the committed sa.json stays a small
 *   hand-authored seed until then.
 *
 * Output shape (both countries — see phase-e-address-picker.md):
 *   { country, source, sourceCommit, license, generatedAt,
 *     labels: { region, city, district },
 *     regions: [{ id, ar, en, cities: [{ id, ar, en, districts: [{ id, ar, en }] }] }] }
 */

import fs from 'node:fs';
import path from 'node:path';

const OUT_DIR = path.resolve(import.meta.dirname, '../../src/modules/core/data/geo');

const EG_COMMIT = process.env.EG_GEO_COMMIT ?? '';
const EG_BASE = 'https://raw.githubusercontent.com/Tech-Labs/egypt-governorates-and-cities-db';

interface EgGovernorate {
  id: number | string;
  governorate_name_ar: string;
  governorate_name_en: string;
}
interface EgCity {
  id: number | string;
  governorate_id: number | string;
  city_name_ar: string;
  city_name_en: string;
}

async function fetchJson<T>(url: string): Promise<T> {
  const res = await fetch(url);
  if (!res.ok) throw new Error(`fetch failed: ${url} (${res.status})`);
  return (await res.json()) as T;
}

function dedupeSort<T extends { ar: string }>(items: T[]): T[] {
  const seen = new Set<string>();
  const out: T[] = [];
  for (const item of items) {
    const key = item.ar.trim();
    if (!key || seen.has(key)) continue;
    seen.add(key);
    out.push(item);
  }
  return out.sort((a, b) => a.ar.localeCompare(b.ar, 'ar'));
}

async function buildEgypt(): Promise<void> {
  if (!EG_COMMIT) {
    console.error(
      'EG_GEO_COMMIT env var not set — refusing to fetch from a moving branch. ' +
        'Pin a commit SHA from Tech-Labs/egypt-governorates-and-cities-db and re-run:\n' +
        '  EG_GEO_COMMIT=<sha> bun run geo:build',
    );
    process.exitCode = 1;
    return;
  }

  const [governorates, cities] = await Promise.all([
    fetchJson<EgGovernorate[]>(`${EG_BASE}/${EG_COMMIT}/governorates.json`),
    fetchJson<EgCity[]>(`${EG_BASE}/${EG_COMMIT}/cities.json`),
  ]);

  const regions = dedupeSort(
    governorates.map((g) => ({
      id: String(g.id),
      ar: g.governorate_name_ar.trim(),
      en: g.governorate_name_en.trim(),
      cities: dedupeSort(
        cities
          .filter((c) => String(c.governorate_id) === String(g.id))
          .map((c) => ({ id: String(c.id), ar: c.city_name_ar.trim(), en: c.city_name_en.trim(), districts: [] as unknown[] })),
      ),
    })),
  );

  const out = {
    country: 'EG',
    source: 'Tech-Labs/egypt-governorates-and-cities-db',
    sourceCommit: EG_COMMIT,
    license: 'MIT',
    generatedAt: new Date().toISOString(),
    labels: { region: 'المحافظة', city: 'المدينة / المركز', district: null },
    regions,
  };

  fs.writeFileSync(path.join(OUT_DIR, 'eg.json'), JSON.stringify(out, null, 2) + '\n', 'utf8');
  console.log(`wrote ${regions.length} governorates to eg.json`);
}

async function main() {
  await buildEgypt();
  console.log(
    '\nsa.json was NOT rebuilt: the Saudi source (homaily, GPL-2.0) is blocked on an open licensing ' +
      'decision — see plans/pending/18-countries-a11y-diagnostics/README.md. The committed sa.json ' +
      'stays a hand-authored placeholder until that decision is made.',
  );
}

main().catch((err) => {
  console.error(err);
  process.exitCode = 1;
});

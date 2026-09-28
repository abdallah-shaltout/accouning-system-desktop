/**
 * `bun run verify:export-snapshot` — writes `src-tauri/tests/fixtures/mock-snapshot-demo.json`, a
 * `{ version, savedAt, data }` snapshot of `seedDatabase(new Date('2026-06-30T09:00:00Z'), 'SA')`,
 * exactly the shape the D10 importer's `tests/domain_import.rs` reads (`03-domains/00-import.md`
 * §8a). Gitignored (`src-tauri/.gitignore`) — regenerated locally whenever the seed changes,
 * deterministic given the fixed `now`/country, so it never needs to be reviewed as a diff.
 */
import fs from 'node:fs';
import path from 'node:path';
import { db } from '../../src/mocks/db';
import { seedDatabase } from '../../src/mocks/seed';
import { SCHEMA_VERSION } from '../../src/mocks/persist';

const NOW = new Date('2026-06-30T09:00:00.000Z');

seedDatabase(NOW, 'SA');

const snapshot = {
  version: SCHEMA_VERSION,
  savedAt: NOW.toISOString(),
  data: db,
};

const outDir = path.resolve(__dirname, '../../src-tauri/tests/fixtures');
fs.mkdirSync(outDir, { recursive: true });
const outPath = path.join(outDir, 'mock-snapshot-demo.json');
fs.writeFileSync(outPath, JSON.stringify(snapshot));

// eslint-disable-next-line no-console
console.log(`wrote ${outPath} (${(fs.statSync(outPath).size / 1024).toFixed(1)} KB)`);

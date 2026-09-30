/**
 * `bun run scripts/verify/export-backup-fixtures.ts` — writes the two TS-format backup archives
 * `src-tauri/tests/domain_backup.rs` reads (`03-domains/17-backup.md` §8a, "compatibility with the
 * TS format"): `backup-browser-plain.zip` and `backup-browser-encrypted.zip` (password `1234`).
 *
 * Both are built by the real browser builder (`buildBackupArchive`), from the small checked-in
 * `mock-snapshot-edge.json` data, so they stay tiny and are checked in next to it. Re-run only when
 * the archive format changes (the salt/IV/createdAt differ per run, so a re-run always rewrites them).
 */
import fs from 'node:fs';
import path from 'node:path';
import { buildBackupArchive } from '../../src/modules/settings/helpers/backupArchive';
import { SCHEMA_VERSION } from '../../src/mocks/persist';

const fixturesDir = path.resolve(__dirname, '../../src-tauri/tests/fixtures');
const edge = JSON.parse(fs.readFileSync(path.join(fixturesDir, 'mock-snapshot-edge.json'), 'utf8'));

const plain = await buildBackupArchive('manual', edge.data, [], SCHEMA_VERSION);
const encrypted = await buildBackupArchive('manual', edge.data, [], SCHEMA_VERSION, '1234');

for (const [name, built] of [
  ['backup-browser-plain.zip', plain],
  ['backup-browser-encrypted.zip', encrypted],
] as const) {
  const outPath = path.join(fixturesDir, name);
  fs.writeFileSync(outPath, built.bytes);
  // eslint-disable-next-line no-console
  console.log(`wrote ${outPath} (${built.bytes.length} bytes, checksum ${built.manifest.checksum.slice(0, 12)}…)`);
}
